//! Answer-operation registry. Every typed, capture, scroll and voice answer gets an id
//! here *before* any work starts, so F3 can cancel exactly the operations that were
//! running at the moment it was pressed.

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tauri::State;
use tokio_util::sync::CancellationToken;

const CANCELLED_CAP: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpKind {
    Typed,
    Capture,
    Scroll,
    Voice,
}

impl OpKind {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "typed" => Some(Self::Typed),
            "capture" => Some(Self::Capture),
            "scroll" => Some(Self::Scroll),
            "voice" => Some(Self::Voice),
            _ => None,
        }
    }
}

/// Shared stop flag for scroll capture. PageDown is only sent while holding this lock,
/// and stopping takes the same lock, so no PageDown can follow a completed stop.
pub type ScrollGuard = Arc<Mutex<bool>>;

struct OpEntry {
    #[allow(dead_code)]
    kind: OpKind,
    token: CancellationToken,
    scroll_guard: ScrollGuard,
}

pub struct OpRegistry {
    next_id: AtomicU64,
    active: Mutex<HashMap<u64, OpEntry>>,
    cancelled: Mutex<VecDeque<u64>>,
}

impl OpRegistry {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            active: Mutex::new(HashMap::new()),
            cancelled: Mutex::new(VecDeque::new()),
        }
    }

    pub fn begin(&self, kind: OpKind) -> (u64, CancellationToken) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let token = CancellationToken::new();
        self.active.lock().unwrap().insert(
            id,
            OpEntry {
                kind,
                token: token.clone(),
                scroll_guard: Arc::new(Mutex::new(false)),
            },
        );
        (id, token)
    }

    pub fn end(&self, id: u64) {
        self.active.lock().unwrap().remove(&id);
    }

    pub fn token(&self, id: u64) -> Option<CancellationToken> {
        self.active.lock().unwrap().get(&id).map(|e| e.token.clone())
    }

    pub fn scroll_guard(&self, id: u64) -> Option<ScrollGuard> {
        self.active
            .lock()
            .unwrap()
            .get(&id)
            .map(|e| Arc::clone(&e.scroll_guard))
    }

    pub fn is_cancelled(&self, id: u64) -> bool {
        self.cancelled.lock().unwrap().contains(&id)
    }

    /// Cancel one operation (used by timeouts).
    pub fn cancel(&self, id: u64) {
        let entry = self
            .active
            .lock()
            .unwrap()
            .get(&id)
            .map(|e| (e.token.clone(), Arc::clone(&e.scroll_guard)));
        if let Some((token, guard)) = entry {
            stop(&token, &guard);
            self.mark_cancelled(id);
        }
    }

    /// F3: snapshot the operations active right now and cancel only those.
    pub fn cancel_snapshot(&self) -> Vec<u64> {
        let snapshot: Vec<(u64, CancellationToken, ScrollGuard)> = self
            .active
            .lock()
            .unwrap()
            .iter()
            .map(|(id, e)| (*id, e.token.clone(), Arc::clone(&e.scroll_guard)))
            .collect();
        let mut ids = Vec::with_capacity(snapshot.len());
        for (id, token, guard) in snapshot {
            stop(&token, &guard);
            self.mark_cancelled(id);
            ids.push(id);
        }
        ids.sort_unstable();
        ids
    }

    fn mark_cancelled(&self, id: u64) {
        let mut c = self.cancelled.lock().unwrap();
        if !c.contains(&id) {
            c.push_back(id);
            while c.len() > CANCELLED_CAP {
                c.pop_front();
            }
        }
    }
}

fn stop(token: &CancellationToken, guard: &ScrollGuard) {
    token.cancel();
    *guard.lock().unwrap() = true;
}

#[tauri::command]
pub fn begin_op_cmd(kind: String, ops: State<'_, OpRegistry>) -> Result<u64, String> {
    let kind = OpKind::parse(&kind).ok_or_else(|| format!("Unknown operation kind: {kind}"))?;
    Ok(ops.begin(kind).0)
}

#[tauri::command]
pub fn end_op_cmd(id: u64, ops: State<'_, OpRegistry>) {
    ops.end(id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancel_snapshot_only_hits_operations_active_at_press_time() {
        let reg = OpRegistry::new();
        let (a, ta) = reg.begin(OpKind::Typed);
        let (b, tb) = reg.begin(OpKind::Voice);
        let ids = reg.cancel_snapshot();
        assert_eq!(ids, vec![a, b]);
        assert!(ta.is_cancelled() && tb.is_cancelled());

        // Started after the snapshot: untouched by it.
        let (c, tc) = reg.begin(OpKind::Typed);
        assert!(c > b);
        assert!(!tc.is_cancelled());
        assert!(!reg.is_cancelled(c));
        assert!(reg.is_cancelled(a) && reg.is_cancelled(b));
    }

    #[test]
    fn cancel_sets_scroll_guard_and_ended_ops_are_not_cancelled() {
        let reg = OpRegistry::new();
        let (s, _) = reg.begin(OpKind::Scroll);
        let guard = reg.scroll_guard(s).unwrap();
        let (done, _) = reg.begin(OpKind::Capture);
        reg.end(done);
        assert_eq!(reg.cancel_snapshot(), vec![s]);
        assert!(*guard.lock().unwrap());
        assert!(!reg.is_cancelled(done));
    }

    #[test]
    fn cancelled_set_is_bounded() {
        let reg = OpRegistry::new();
        for _ in 0..(CANCELLED_CAP + 10) {
            let (id, _) = reg.begin(OpKind::Typed);
            reg.cancel(id);
            reg.end(id);
        }
        assert_eq!(reg.cancelled.lock().unwrap().len(), CANCELLED_CAP);
    }
}

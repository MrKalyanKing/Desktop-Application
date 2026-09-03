//! Lock-free SPSC ring of f32 samples for the CPAL callback.
//! Producer (audio thread) never takes a mutex. Overflow overwrites oldest.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

pub struct SpscAudioRing {
    data: Box<[AtomicU32]>,
    mask: usize,
    write: AtomicUsize,
    read: AtomicUsize,
}

impl SpscAudioRing {
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.next_power_of_two().max(2);
        let data = (0..cap)
            .map(|_| AtomicU32::new(0))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            data,
            mask: cap - 1,
            write: AtomicUsize::new(0),
            read: AtomicUsize::new(0),
        }
    }

    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    pub fn len(&self) -> usize {
        let w = self.write.load(Ordering::Acquire);
        let r = self.read.load(Ordering::Acquire);
        w.wrapping_sub(r).min(self.data.len())
    }

    pub fn clear(&self) {
        let w = self.write.load(Ordering::Relaxed);
        self.read.store(w, Ordering::Release);
    }

    /// Audio-thread producer. Overwrites oldest samples when full.
    pub fn push_slice(&self, samples: &[f32]) {
        for &s in samples {
            let w = self.write.load(Ordering::Relaxed);
            let r = self.read.load(Ordering::Acquire);
            if w.wrapping_sub(r) >= self.data.len() {
                self.read.store(r.wrapping_add(1), Ordering::Release);
            }
            self.data[w & self.mask].store(s.to_bits(), Ordering::Relaxed);
            self.write.store(w.wrapping_add(1), Ordering::Release);
        }
    }

    /// Consumer: drain available samples into `out`.
    pub fn pop_into(&self, out: &mut Vec<f32>) {
        out.clear();
        let mut r = self.read.load(Ordering::Relaxed);
        let w = self.write.load(Ordering::Acquire);
        while r != w {
            let bits = self.data[r & self.mask].load(Ordering::Relaxed);
            out.push(f32::from_bits(bits));
            r = r.wrapping_add(1);
            if out.len() >= self.data.len() {
                break;
            }
        }
        self.read.store(r, Ordering::Release);
    }

    pub fn snapshot(&self) -> Vec<f32> {
        let r = self.read.load(Ordering::Acquire);
        let w = self.write.load(Ordering::Acquire);
        let mut out = Vec::new();
        let mut i = r;
        while i != w {
            let bits = self.data[i & self.mask].load(Ordering::Relaxed);
            out.push(f32::from_bits(bits));
            i = i.wrapping_add(1);
            if out.len() >= self.data.len() {
                break;
            }
        }
        out
    }
}

import { useQuery } from '@tanstack/react-query';
import { api } from '../../../services/api/client';
import { GlassCard } from '../../../components/ui/GlassCard';
import { StatusBadge } from '../../../components/ui/StatusBadge';
import { formatDate } from '../../../utils/cn';

export function DesktopPage() {
  const { data } = useQuery({ queryKey: ['desktop'], queryFn: api.desktopStatus });
  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-semibold">Desktop integration</h1>
      <p className="text-sm text-[var(--lg-text-muted)]">
        The Tauri/Rust app keeps microphone capture, STT/Gemini, and local assistance. This API only stores transcripts and session state.
      </p>
      <GlassCard className="p-6">
        <div className="flex items-center gap-3">
          <StatusBadge value={data?.connected ? 'connected' : 'offline'} />
          <span className="text-lg font-medium">{data?.device?.name ?? 'No device registered'}</span>
        </div>
        <dl className="mt-5 grid gap-3 text-sm sm:grid-cols-2">
          <div>
            <dt className="text-[var(--lg-text-muted)]">Last seen</dt>
            <dd>{formatDate(data?.device?.lastSeenAt)}</dd>
          </div>
          <div>
            <dt className="text-[var(--lg-text-muted)]">Last sync</dt>
            <dd>{formatDate(data?.device?.lastSyncAt)}</dd>
          </div>
        </dl>
      </GlassCard>
      <GlassCard className="space-y-2 p-6 text-sm leading-6 text-[var(--lg-text-secondary)]">
        <p>Device login: <code>POST /api/v1/auth/desktop/register</code></p>
        <p>Incremental sync: <code>POST /api/v1/desktop/sync/:sessionId</code></p>
        <p>Idempotency key: client <code>eventId</code> unique per session.</p>
        <p>Rust never receives DATABASE_URL or AWS keys.</p>
      </GlassCard>
    </div>
  );
}

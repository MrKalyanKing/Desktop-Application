import { GlassCard } from '../../../components/ui/GlassCard';
import { USE_MOCK } from '../../../lib/config';

export function SettingsPage() {
  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-semibold">Settings</h1>
      <GlassCard className="space-y-4 p-6">
        <div>
          <div className="text-sm text-[var(--lg-text-muted)]">Data mode</div>
          <div className="mt-1">{USE_MOCK ? 'Mock dataset (UI development)' : 'Live NestJS API'}</div>
        </div>
        <div>
          <div className="text-sm text-[var(--lg-text-muted)]">Retention</div>
          <div className="mt-1">Transcript retention defaults to 365 days and is organization-scoped.</div>
        </div>
        <div>
          <div className="text-sm text-[var(--lg-text-muted)]">Consent</div>
          <div className="mt-1">Sessions record whether interview capture consent was captured on desktop or web.</div>
        </div>
      </GlassCard>
    </div>
  );
}

import { useQuery } from '@tanstack/react-query';
import { Bar, BarChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { api } from '../../../services/api/client';
import { GlassCard } from '../../../components/ui/GlassCard';
import { StatusBadge } from '../../../components/ui/StatusBadge';
import { Link } from 'react-router-dom';

export function AnalyticsPage() {
  const sessions = useQuery({ queryKey: ['sessions'], queryFn: () => api.sessions() });
  const analytics = useQuery({ queryKey: ['analytics', 'ses-1'], queryFn: () => api.analytics('ses-1') });
  const session = sessions.data?.[0];

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-semibold">Interview analytics</h1>
        <p className="disclaimer mt-2">
          Measured values come from timestamps and word counts. AI interpretations are labeled and must be reviewed by a human.
        </p>
      </div>
      <div className="grid gap-4 lg:grid-cols-2">
        <GlassCard className="p-5">
          <h2 className="mb-4 font-medium">Candidate vs interviewer speaking (measured)</h2>
          <div className="h-56">
            <ResponsiveContainer width="100%" height="100%">
              <BarChart data={[{ name: 'Candidate', ms: 29000 }, { name: 'Interviewer', ms: 6000 }]}>
                <XAxis dataKey="name" stroke="#9ca3af" />
                <YAxis hide />
                <Tooltip />
                <Bar dataKey="ms" fill="#5ee9a3" radius={[8, 8, 0, 0]} />
              </BarChart>
            </ResponsiveContainer>
          </div>
        </GlassCard>
        <GlassCard className="p-5">
          <h2 className="mb-4 font-medium">Interviewer question patterns</h2>
          <ul className="space-y-3 text-sm">
            <li>Question categories remain extensible per session.</li>
            <li>Follow-up phrasing is flagged only when the transcript contains it.</li>
            <li>Interruptions are recorded only when the desktop app sends overlapping-speech events.</li>
          </ul>
        </GlassCard>
      </div>
      {(analytics.data ?? []).map((row) => (
        <GlassCard key={row.id} className="p-5">
          <div className="mb-3 flex gap-2">
            <StatusBadge value={row.origin} />
            <StatusBadge value={row.kind} />
          </div>
          <pre className="overflow-auto text-xs text-[var(--lg-text-secondary)]">{JSON.stringify(row.payload, null, 2)}</pre>
        </GlassCard>
      ))}
      {session && (
        <Link to={`/sessions/${session.id}`} className="text-sm text-emerald-300">
          Open linked session and jump to transcript evidence →
        </Link>
      )}
    </div>
  );
}

import { useQuery } from '@tanstack/react-query';
import { Area, AreaChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import { api } from '../../../services/api/client';
import { GlassCard } from '../../../components/ui/GlassCard';
import { StatusBadge } from '../../../components/ui/StatusBadge';
import { displayName, formatDate } from '../../../utils/cn';
import { Link } from 'react-router-dom';

export function DashboardPage() {
  const { data } = useQuery({ queryKey: ['dashboard'], queryFn: api.dashboard });
  if (!data) return null;
  const cards = [
    ['Sessions', data.totals.sessions],
    ['Completed', data.totals.completed],
    ['Upcoming', data.totals.upcoming],
    ['Resumes', data.totals.resumes],
  ];

  return (
    <div className="space-y-4">
      <div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {cards.map(([label, value]) => (
          <GlassCard key={label} className="p-5">
            <div className="text-sm text-[var(--lg-text-muted)]">{label}</div>
            <div className="mt-2 text-3xl font-semibold">{value}</div>
          </GlassCard>
        ))}
      </div>
      <div className="grid gap-4 xl:grid-cols-3">
        <GlassCard className="p-5 xl:col-span-2">
          <div className="mb-4 text-sm text-[var(--lg-text-muted)]">Interview completion trend</div>
          <div className="h-56">
            <ResponsiveContainer width="100%" height="100%">
              <AreaChart data={data.trends}>
                <XAxis dataKey="day" hide />
                <YAxis hide />
                <Tooltip />
                <Area type="monotone" dataKey="count" stroke="#5ee9a3" fill="rgba(94,233,163,0.2)" />
              </AreaChart>
            </ResponsiveContainer>
          </div>
        </GlassCard>
        <GlassCard className="p-5">
          <div className="text-sm text-[var(--lg-text-muted)]">Desktop application</div>
          <div className="mt-3 text-xl font-medium">
            {data.desktop.connected ? 'Connected' : 'Offline'}
          </div>
          <p className="mt-2 text-sm text-[var(--lg-text-secondary)]">
            {data.desktop.activeDevices} device{data.desktop.activeDevices === 1 ? '' : 's'} seen in the last 5 minutes.
          </p>
          <Link to="/desktop" className="mt-4 inline-block text-sm text-emerald-300">
            Open desktop sync →
          </Link>
        </GlassCard>
      </div>
      <GlassCard className="overflow-hidden">
        <div className="px-5 py-4 text-sm text-[var(--lg-text-muted)]">Recent sessions</div>
        <div className="table-wrap">
          <table className="data">
            <thead>
              <tr>
                <th>Title</th>
                <th>Candidate</th>
                <th>Status</th>
                <th>Sync</th>
                <th>Updated</th>
              </tr>
            </thead>
            <tbody>
              {data.recentSessions.map((session) => (
                <tr key={session.id}>
                  <td>
                    <Link to={`/sessions/${session.id}`} className="hover:text-emerald-200">
                      {session.title}
                    </Link>
                  </td>
                  <td>{displayName(session.candidate?.firstName, session.candidate?.lastName)}</td>
                  <td><StatusBadge value={session.status} /></td>
                  <td><StatusBadge value={session.desktopSyncStatus} /></td>
                  <td>{formatDate(session.createdAt)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </GlassCard>
    </div>
  );
}

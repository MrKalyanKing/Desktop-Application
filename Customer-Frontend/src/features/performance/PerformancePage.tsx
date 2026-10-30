import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { Bar, BarChart, ResponsiveContainer, Tooltip, XAxis } from 'recharts';
import { api } from '../../services/api/client';

export function PerformancePage() {
  const { data } = useQuery({ queryKey: ['performance'], queryFn: api.performance });
  if (!data) return null;
  const chart = [
    { name: 'Practice', n: data.totals.practice },
    { name: 'Completed', n: data.totals.completed },
    { name: 'All', n: data.totals.interviews },
  ];

  return (
    <div className="space-y-5">
      <h1 className="serif text-4xl">Your performance</h1>
      <p className="max-w-2xl text-sm text-[var(--muted)]">{data.disclaimer}</p>
      <div className="panel rounded-[28px] p-5">
        <div className="h-56">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={chart}>
              <XAxis dataKey="name" stroke="#c4a06a" />
              <Tooltip />
              <Bar dataKey="n" fill="#e0c39a" radius={[8, 8, 0, 0]} />
            </BarChart>
          </ResponsiveContainer>
        </div>
      </div>
      {data.latestInsights.map((row) => (
        <article key={row.kind} className="panel rounded-[28px] p-6">
          <div className="chip">{row.origin.replace('_', ' ')}</div>
          {'insights' in row.payload && Array.isArray(row.payload.insights) ? (
            <ul className="mt-4 space-y-2">
              {(row.payload.insights as { dimension: string; label: string }[]).map((insight) => (
                <li key={insight.dimension}>
                  <span className="text-[var(--gold)]">{insight.dimension}:</span> {insight.label}
                </li>
              ))}
            </ul>
          ) : (
            <pre className="mt-4 overflow-auto text-xs">{JSON.stringify(row.payload, null, 2)}</pre>
          )}
        </article>
      ))}
      <Link to="/app/practice" className="btn btn-gold">Practice again</Link>
    </div>
  );
}

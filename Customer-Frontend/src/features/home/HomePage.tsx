import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { api } from '../../services/api/client';
import { useAuth } from '../../app/providers/AuthProvider';
import { formatDate } from '../../utils/cn';

export function HomePage() {
  const { user } = useAuth();
  const performance = useQuery({ queryKey: ['performance'], queryFn: api.performance });
  const resumes = useQuery({ queryKey: ['resumes'], queryFn: api.resumes });
  const data = performance.data;

  return (
    <div className="space-y-5">
      <div>
        <p className="chip">Today</p>
        <h1 className="serif mt-3 text-4xl">Keep the story sharp, {user?.firstName ?? 'there'}.</h1>
      </div>
      <div className="grid gap-4 sm:grid-cols-3">
        {[
          ['Practice rounds', data?.totals.practice ?? '—'],
          ['Completed', data?.totals.completed ?? '—'],
          ['Resumes', resumes.data?.length ?? '—'],
        ].map(([label, value]) => (
          <div key={label} className="panel rounded-[24px] p-5">
            <div className="text-sm text-[var(--muted)]">{label}</div>
            <div className="serif mt-2 text-4xl">{value}</div>
          </div>
        ))}
      </div>
      <div className="grid gap-4 lg:grid-cols-2">
        <div className="panel rounded-[28px] p-6">
          <h2 className="serif text-2xl">Continue</h2>
          <p className="mt-2 text-sm text-[var(--muted)]">A short technical round takes about twelve minutes.</p>
          <Link to="/app/practice" className="btn btn-gold mt-5">Start a practice interview</Link>
        </div>
        <div className="panel rounded-[28px] p-6">
          <h2 className="serif text-2xl">Recent rooms</h2>
          <ul className="mt-4 space-y-3 text-sm">
            {(data?.recent ?? []).map((session) => (
              <li key={session.id} className="flex items-center justify-between gap-3">
                <Link to={`/app/interviews/${session.id}`}>{session.title}</Link>
                <span className="text-[var(--muted)]">{formatDate(session.createdAt)}</span>
              </li>
            ))}
          </ul>
        </div>
      </div>
    </div>
  );
}

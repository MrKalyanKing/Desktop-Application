import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { api } from '../../services/api/client';
import { formatDate } from '../../utils/cn';

export function InterviewsPage() {
  const { data = [] } = useQuery({ queryKey: ['interviews'], queryFn: api.interviews });
  return (
    <div className="space-y-5">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="serif text-4xl">Interviews</h1>
          <p className="mt-2 text-[var(--muted)]">Practice rooms and scheduled conversations, all in one place.</p>
        </div>
        <Link to="/app/practice" className="btn btn-gold">New practice</Link>
      </div>
      <div className="grid gap-3">
        {data.map((session) => (
          <Link key={session.id} to={`/app/interviews/${session.id}`} className="panel block rounded-[24px] p-5">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <div className="capitalize">{session.title}</div>
                <div className="text-sm text-[var(--muted)]">
                  {session.jobTitle} · {session.company}
                </div>
              </div>
              <div className="flex items-center gap-3 text-sm">
                <span className="chip">{session.status.replace('_', ' ')}</span>
                <span className="text-[var(--muted)]">{formatDate(session.scheduledAt ?? session.createdAt)}</span>
              </div>
            </div>
          </Link>
        ))}
      </div>
    </div>
  );
}

import { useQuery } from '@tanstack/react-query';
import { api } from '../../../services/api/client';
import { GlassCard } from '../../../components/ui/GlassCard';
import { StatusBadge } from '../../../components/ui/StatusBadge';

export function CandidatesPage() {
  const { data = [] } = useQuery({ queryKey: ['candidates'], queryFn: api.candidates });
  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-semibold">Candidates</h1>
      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        {data.map((candidate) => (
          <GlassCard key={candidate.id} className="p-5">
            <div className="text-lg font-medium">
              {candidate.firstName} {candidate.lastName}
            </div>
            <div className="text-sm text-[var(--lg-text-muted)]">
              {candidate.currentTitle} · {candidate.currentCompany}
            </div>
            <div className="mt-3 flex flex-wrap gap-2">
              {candidate.skills.map((skill) => (
                <span key={skill} className="badge">{skill}</span>
              ))}
            </div>
            <div className="mt-4">
              <StatusBadge value={candidate.consentGiven ? 'consent' : 'pending consent'} />
            </div>
          </GlassCard>
        ))}
      </div>
    </div>
  );
}

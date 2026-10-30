import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { api } from '../../../services/api/client';
import { Button } from '../../../components/ui/Button';
import { GlassCard } from '../../../components/ui/GlassCard';
import { StatusBadge } from '../../../components/ui/StatusBadge';
import { formatDate } from '../../../utils/cn';

export function ReportsPage() {
  const client = useQueryClient();
  const reports = useQuery({ queryKey: ['reports'], queryFn: api.reports });
  const sessions = useQuery({ queryKey: ['sessions'], queryFn: () => api.sessions() });
  const generate = useMutation({
    mutationFn: () => api.generateReport(sessions.data?.[0]?.id ?? 'ses-1', 'session_summary'),
    onSuccess: () => client.invalidateQueries({ queryKey: ['reports'] }),
  });

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Reports</h1>
          <p className="text-sm text-[var(--lg-text-muted)]">Modular templates: session summary now, more later.</p>
        </div>
        <Button variant="primary" onClick={() => generate.mutate()}>Generate PDF</Button>
      </div>
      <div className="grid gap-3">
        {(reports.data ?? []).map((report) => (
          <GlassCard key={report.id} className="flex flex-col gap-3 p-5 sm:flex-row sm:items-center sm:justify-between">
            <div>
              <div className="font-medium">{report.templateKey.replace('_', ' ')}</div>
              <div className="text-sm text-[var(--lg-text-muted)]">{formatDate(report.createdAt)}</div>
            </div>
            <div className="flex items-center gap-3">
              <StatusBadge value={report.status} />
              <Button>Download</Button>
            </div>
          </GlassCard>
        ))}
      </div>
    </div>
  );
}

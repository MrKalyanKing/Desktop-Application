import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { useForm } from 'react-hook-form';
import { z } from 'zod';
import { zodResolver } from '@hookform/resolvers/zod';
import { Link } from 'react-router-dom';
import { api } from '../../../services/api/client';
import { Button } from '../../../components/ui/Button';
import { GlassCard } from '../../../components/ui/GlassCard';
import { Modal } from '../../../components/ui/Modal';
import { StatusBadge } from '../../../components/ui/StatusBadge';
import { displayName, formatDate } from '../../../utils/cn';
import type { InterviewType } from '../../../types';

const schema = z.object({
  title: z.string().min(3),
  jobTitle: z.string().optional(),
  company: z.string().optional(),
  interviewType: z.enum(['mock', 'technical', 'hr', 'general']),
  candidateId: z.string().optional(),
  resumeId: z.string().optional(),
  scheduledAt: z.string().optional(),
});

type FormValues = z.infer<typeof schema>;

export function SessionsPage() {
  const client = useQueryClient();
  const [search, setSearch] = useState('');
  const [status, setStatus] = useState('');
  const [open, setOpen] = useState(false);
  const [confirmId, setConfirmId] = useState<string | null>(null);
  const sessions = useQuery({ queryKey: ['sessions'], queryFn: () => api.sessions() });
  const candidates = useQuery({ queryKey: ['candidates'], queryFn: api.candidates });
  const resumes = useQuery({ queryKey: ['resumes'], queryFn: api.resumes });
  const create = useMutation({
    mutationFn: (values: FormValues) => api.createSession(values),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ['sessions'] });
      setOpen(false);
    },
  });
  const remove = useMutation({
    mutationFn: (id: string) => api.deleteSession(id),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ['sessions'] });
      setConfirmId(null);
    },
  });

  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: { interviewType: 'technical' },
  });

  const rows = useMemo(
    () =>
      (sessions.data ?? []).filter((session) => {
        const q = search.toLowerCase();
        const matchesSearch =
          !q ||
          session.title.toLowerCase().includes(q) ||
          (session.jobTitle ?? '').toLowerCase().includes(q);
        const matchesStatus = !status || session.status === status;
        return matchesSearch && matchesStatus;
      }),
    [sessions.data, search, status],
  );

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 className="text-2xl font-semibold">Interview sessions</h1>
          <p className="text-sm text-[var(--lg-text-muted)]">Create, schedule, and archive interviews.</p>
        </div>
        <Button variant="primary" onClick={() => setOpen(true)}>New session</Button>
      </div>
      <GlassCard className="flex flex-col gap-3 p-4 sm:flex-row">
        <input className="field" placeholder="Search title or role" value={search} onChange={(e) => setSearch(e.target.value)} />
        <select className="field sm:max-w-48" value={status} onChange={(e) => setStatus(e.target.value)}>
          <option value="">All statuses</option>
          {['draft', 'scheduled', 'in_progress', 'completed', 'archived'].map((value) => (
            <option key={value} value={value}>{value}</option>
          ))}
        </select>
      </GlassCard>
      <div className="grid gap-3 md:hidden">
        {rows.map((session) => (
          <GlassCard key={session.id} className="p-4">
            <Link to={`/sessions/${session.id}`} className="font-medium">{session.title}</Link>
            <div className="mt-2 text-sm text-[var(--lg-text-muted)]">{displayName(session.candidate?.firstName, session.candidate?.lastName)}</div>
            <div className="mt-3 flex flex-wrap gap-2">
              <StatusBadge value={session.status} />
              <StatusBadge value={session.interviewType} />
            </div>
          </GlassCard>
        ))}
      </div>
      <GlassCard className="hidden overflow-hidden md:block">
        <table className="data">
          <thead>
            <tr>
              <th>Session</th>
              <th>Candidate</th>
              <th>Type</th>
              <th>Status</th>
              <th>Sync</th>
              <th>When</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {rows.map((session) => (
              <tr key={session.id}>
                <td>
                  <Link to={`/sessions/${session.id}`}>{session.title}</Link>
                  <div className="text-xs text-[var(--lg-text-muted)]">{session.jobTitle} · {session.company}</div>
                </td>
                <td>{displayName(session.candidate?.firstName, session.candidate?.lastName)}</td>
                <td className="capitalize">{session.interviewType}</td>
                <td><StatusBadge value={session.status} /></td>
                <td><StatusBadge value={session.desktopSyncStatus} /></td>
                <td>{formatDate(session.scheduledAt ?? session.createdAt)}</td>
                <td>
                  <Button className="text-xs" onClick={() => setConfirmId(session.id)}>Delete</Button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </GlassCard>

      <Modal open={open} title="Create interview session" onClose={() => setOpen(false)}>
        <form className="space-y-3" onSubmit={form.handleSubmit((values) => create.mutate(values))}>
          <input className="field" placeholder="Session title" {...form.register('title')} />
          <div className="grid gap-3 sm:grid-cols-2">
            <input className="field" placeholder="Job title" {...form.register('jobTitle')} />
            <input className="field" placeholder="Company" {...form.register('company')} />
          </div>
          <select className="field" {...form.register('interviewType')}>
            {(['mock', 'technical', 'hr', 'general'] as InterviewType[]).map((type) => (
              <option key={type} value={type}>{type}</option>
            ))}
          </select>
          <select className="field" {...form.register('candidateId')}>
            <option value="">Select candidate</option>
            {(candidates.data ?? []).map((candidate) => (
              <option key={candidate.id} value={candidate.id}>
                {candidate.firstName} {candidate.lastName}
              </option>
            ))}
          </select>
          <select className="field" {...form.register('resumeId')}>
            <option value="">Attach resume</option>
            {(resumes.data ?? []).map((resume) => (
              <option key={resume.id} value={resume.id}>{resume.originalFilename}</option>
            ))}
          </select>
          <input className="field" type="datetime-local" {...form.register('scheduledAt')} />
          <Button variant="primary" type="submit" className="w-full">Create session</Button>
        </form>
      </Modal>

      <Modal open={Boolean(confirmId)} title="Delete this session?" onClose={() => setConfirmId(null)}>
        <p className="mb-4 text-sm text-[var(--lg-text-secondary)]">This removes the session and its transcript from the web store. The desktop app is not affected.</p>
        <div className="flex justify-end gap-2">
          <Button onClick={() => setConfirmId(null)}>Cancel</Button>
          <Button variant="danger" onClick={() => confirmId && remove.mutate(confirmId)}>Delete</Button>
        </div>
      </Modal>
    </div>
  );
}

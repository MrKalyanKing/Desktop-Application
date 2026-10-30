import { useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import { useMutation, useQuery } from '@tanstack/react-query';
import { api } from '../../services/api/client';
import type { InterviewType } from '../../types';

const types: InterviewType[] = ['mock', 'technical', 'hr', 'general'];

export function PracticePage() {
  const navigate = useNavigate();
  const resumes = useQuery({ queryKey: ['resumes'], queryFn: api.resumes });
  const form = useForm({
    defaultValues: { interviewType: 'mock' as InterviewType, jobTitle: '', company: '', resumeId: '' },
  });
  const start = useMutation({
    mutationFn: api.startPractice,
    onSuccess: (detail) => navigate(`/app/interviews/${detail.session.id}`),
  });

  return (
    <div className="mx-auto max-w-xl space-y-5">
      <h1 className="serif text-4xl">Open a practice room</h1>
      <p className="text-[var(--muted)]">Five questions. Your words. Coaching notes when you finish.</p>
      <form
        className="panel space-y-4 rounded-[28px] p-6"
        onSubmit={form.handleSubmit((values) => start.mutate(values))}
      >
        <select className="field capitalize" {...form.register('interviewType')}>
          {types.map((type) => (
            <option key={type} value={type}>{type}</option>
          ))}
        </select>
        <input className="field" placeholder="Target role" {...form.register('jobTitle')} />
        <input className="field" placeholder="Company (optional)" {...form.register('company')} />
        <select className="field" {...form.register('resumeId')}>
          <option value="">Attach a resume</option>
          {(resumes.data ?? []).map((resume) => (
            <option key={resume.id} value={resume.id}>{resume.originalFilename}</option>
          ))}
        </select>
        <button className="btn btn-gold w-full" type="submit">Begin</button>
      </form>
    </div>
  );
}

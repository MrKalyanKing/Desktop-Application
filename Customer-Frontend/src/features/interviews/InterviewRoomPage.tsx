import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { api } from '../../services/api/client';

export function InterviewRoomPage() {
  const { id = '' } = useParams();
  const navigate = useNavigate();
  const client = useQueryClient();
  const detail = useQuery({ queryKey: ['interview', id], queryFn: () => api.interview(id) });
  const [index, setIndex] = useState(0);
  const [draft, setDraft] = useState('');
  const answer = useMutation({
    mutationFn: () => api.answer(id, questions[index].id, draft),
    onSuccess: () => {
      setDraft('');
      if (index + 1 < questions.length) setIndex(index + 1);
    },
  });
  const complete = useMutation({
    mutationFn: () => api.complete(id),
    onSuccess: () => {
      client.invalidateQueries({ queryKey: ['performance'] });
      navigate('/app/performance');
    },
  });

  const questions = detail.data?.questions ?? [];
  const current = questions[index];
  const progress = useMemo(
    () => (questions.length ? Math.round((index / questions.length) * 100) : 0),
    [index, questions.length],
  );

  if (detail.isLoading) {
    return <div className="panel rounded-[28px] p-8 text-[var(--muted)]">Preparing the room…</div>;
  }

  if (!current) {
    return (
      <div className="panel rounded-[28px] p-8">
        <h1 className="serif text-3xl">This room is ready to close</h1>
        <button className="btn btn-gold mt-6" onClick={() => complete.mutate()}>See my notes</button>
      </div>
    );
  }

  return (
    <div className="space-y-5">
      <div className="flex items-center justify-between text-sm text-[var(--muted)]">
        <span className="capitalize">{detail.data?.session.interviewType} round</span>
        <span>
          {index + 1} / {questions.length}
        </span>
      </div>
      <div className="h-1 overflow-hidden rounded-full bg-black/30">
        <div className="h-full bg-[var(--gold)]" style={{ width: `${progress}%` }} />
      </div>
      <article className="panel rounded-[32px] p-6 sm:p-10">
        <p className="chip">Question</p>
        <h1 className="serif mt-4 text-3xl leading-tight sm:text-4xl">{current.text}</h1>
        <textarea
          className="field mt-8 min-h-40"
          placeholder="Answer in your own voice. Use an example if you have one."
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
        />
        <div className="mt-5 flex flex-wrap gap-3">
          <button className="btn btn-gold" disabled={!draft.trim() || answer.isPending} onClick={() => answer.mutate()}>
            {index + 1 === questions.length ? 'Save last answer' : 'Next question'}
          </button>
          {index + 1 === questions.length && (
            <button className="btn btn-ghost" onClick={() => complete.mutate()}>Finish interview</button>
          )}
        </div>
      </article>
    </div>
  );
}

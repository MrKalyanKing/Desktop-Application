import { useQuery } from '@tanstack/react-query';
import { useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import { api } from '../../../services/api/client';
import { GlassCard } from '../../../components/ui/GlassCard';
import { StatusBadge } from '../../../components/ui/StatusBadge';
import { Button } from '../../../components/ui/Button';
import { displayName, formatDate } from '../../../utils/cn';

const tabs = ['Overview', 'Resume', 'Transcript', 'Timeline', 'Insights'] as const;

export function SessionDetailPage() {
  const { id = 'ses-1' } = useParams();
  const [tab, setTab] = useState<(typeof tabs)[number]>('Overview');
  const [activeSeq, setActiveSeq] = useState<number | null>(null);
  const session = useQuery({ queryKey: ['session', id], queryFn: () => api.session(id) });
  const transcript = useQuery({ queryKey: ['transcript', id], queryFn: () => api.transcript(id) });
  const analytics = useQuery({ queryKey: ['analytics', id], queryFn: () => api.analytics(id) });
  const data = session.data;
  const segments = transcript.data ?? [];
  const insights = analytics.data ?? [];

  const mapping = useMemo(
    () => segments.map((segment) => ({ seq: segment.sequence, speaker: segment.speakerType })),
    [segments],
  );

  if (!data) return null;

  return (
    <div className="space-y-4">
      <GlassCard className="p-5">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
          <div>
            <div className="text-xs uppercase tracking-[0.18em] text-emerald-200/70">Session</div>
            <h1 className="text-2xl font-semibold">{data.title}</h1>
            <p className="mt-1 text-sm text-[var(--lg-text-muted)]">
              {data.jobTitle} · {data.company} · {displayName(data.candidate?.firstName, data.candidate?.lastName)}
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <StatusBadge value={data.status} />
            <StatusBadge value={data.desktopSyncStatus} />
            <StatusBadge value={data.consentCaptured ? 'consent' : 'no consent'} />
          </div>
        </div>
      </GlassCard>

      <div className="flex gap-2 overflow-x-auto rounded-full bg-black/20 p-1">
        {tabs.map((item) => (
          <button
            key={item}
            className={`rounded-full px-4 py-2 text-sm ${tab === item ? 'glass-btn-primary glass-btn' : 'text-[var(--lg-text-muted)]'}`}
            onClick={() => setTab(item)}
          >
            {item}
          </button>
        ))}
      </div>

      {tab === 'Overview' && (
        <div className="grid gap-4 lg:grid-cols-2">
          <GlassCard className="p-5">
            <h2 className="mb-3 font-medium">Candidate</h2>
            <p>{displayName(data.candidate?.firstName, data.candidate?.lastName)}</p>
            <p className="text-sm text-[var(--lg-text-muted)]">{data.candidate?.currentTitle}</p>
            <div className="mt-3 flex flex-wrap gap-2">
              {(data.candidate?.skills ?? []).map((skill) => (
                <span key={skill} className="badge">{skill}</span>
              ))}
            </div>
          </GlassCard>
          <GlassCard className="p-5">
            <h2 className="mb-3 font-medium">Interview metadata</h2>
            <dl className="space-y-2 text-sm">
              <div className="flex justify-between"><dt>Type</dt><dd className="capitalize">{data.interviewType}</dd></div>
              <div className="flex justify-between"><dt>Started</dt><dd>{formatDate(data.startedAt)}</dd></div>
              <div className="flex justify-between"><dt>Ended</dt><dd>{formatDate(data.endedAt)}</dd></div>
              <div className="flex justify-between"><dt>Resume</dt><dd>{data.resume?.originalFilename ?? '—'}</dd></div>
            </dl>
          </GlassCard>
        </div>
      )}

      {tab === 'Resume' && (
        <GlassCard className="p-5">
          <h2 className="mb-2 font-medium">Resume preview</h2>
          <p className="text-sm text-[var(--lg-text-muted)]">
            Files stay in object storage. PostgreSQL only keeps metadata and extraction JSON.
          </p>
          <div className="mt-4 rounded-2xl border border-white/10 bg-black/20 p-6 text-sm">
            {data.resume?.originalFilename ?? 'No resume attached'}
            <div className="mt-2 text-[var(--lg-text-muted)]">Version {data.resume?.currentVersion ?? 0}</div>
          </div>
        </GlassCard>
      )}

      {tab === 'Transcript' && (
        <GlassCard className="space-y-3 p-5">
          {segments.map((segment) => (
            <button
              key={segment.id}
              onClick={() => setActiveSeq(segment.sequence)}
              className={`w-full rounded-2xl p-3 text-left ${activeSeq === segment.sequence ? 'bg-emerald-400/10' : 'bg-black/15'}`}
            >
              <div className="mb-1 flex items-center justify-between text-xs text-[var(--lg-text-muted)]">
                <span className="capitalize">{segment.speakerType}</span>
                <span>seq {segment.sequence}</span>
              </div>
              {segment.text}
            </button>
          ))}
        </GlassCard>
      )}

      {tab === 'Timeline' && (
        <GlassCard className="p-5">
          <ol className="space-y-4">
            {mapping.map((item) => (
              <li key={item.seq} className="flex gap-3">
                <span className="mt-1 h-2 w-2 rounded-full bg-emerald-300" />
                <div>
                  <div className="text-xs uppercase text-[var(--lg-text-muted)]">{item.speaker} · {item.seq}</div>
                  <div>{segments.find((s) => s.sequence === item.seq)?.text}</div>
                </div>
              </li>
            ))}
          </ol>
        </GlassCard>
      )}

      {tab === 'Insights' && (
        <div className="space-y-4">
          <p className="disclaimer">
            AI-generated assessments are labeled and require human review. They are not an objective measure of candidate ability. Honesty, personality, mental state, and emotion are never inferred.
          </p>
          {insights.map((row) => (
            <GlassCard key={row.id} className="p-5">
              <div className="mb-3 flex flex-wrap items-center gap-2">
                <StatusBadge value={row.origin} />
                <StatusBadge value={row.humanReviewStatus} />
                <span className="text-sm capitalize text-[var(--lg-text-muted)]">{row.kind.replace('_', ' ')}</span>
              </div>
              {'insights' in row.payload && Array.isArray(row.payload.insights) ? (
                <ul className="space-y-2 text-sm">
                  {(row.payload.insights as { dimension: string; label: string; evidenceSequences?: number[] }[]).map((insight) => (
                    <li key={insight.dimension}>
                      <button
                        className="text-left"
                        onClick={() => {
                          setTab('Transcript');
                          setActiveSeq(insight.evidenceSequences?.[0] ?? null);
                        }}
                      >
                        <span className="text-emerald-200">{insight.dimension}:</span> {insight.label}
                      </button>
                    </li>
                  ))}
                </ul>
              ) : (
                <pre className="overflow-auto text-xs text-[var(--lg-text-secondary)]">{JSON.stringify(row.payload, null, 2)}</pre>
              )}
            </GlassCard>
          ))}
          <Button onClick={() => api.generateAnalytics(id)}>Regenerate analysis</Button>
        </div>
      )}
    </div>
  );
}

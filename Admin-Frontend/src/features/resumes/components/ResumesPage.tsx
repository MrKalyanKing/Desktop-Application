import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { api } from '../../../services/api/client';
import { Button } from '../../../components/ui/Button';
import { GlassCard } from '../../../components/ui/GlassCard';
import { formatDate } from '../../../utils/cn';

export function ResumesPage() {
  const client = useQueryClient();
  const [search, setSearch] = useState('');
  const [drag, setDrag] = useState(false);
  const resumes = useQuery({ queryKey: ['resumes'], queryFn: api.resumes });
  const upload = useMutation({
    mutationFn: (file: File) => api.uploadResume(file),
    onSuccess: () => client.invalidateQueries({ queryKey: ['resumes'] }),
  });

  const rows = (resumes.data ?? []).filter((row) =>
    row.originalFilename.toLowerCase().includes(search.toLowerCase()),
  );

  function takeFiles(files: FileList | null) {
    if (!files?.[0]) return;
    upload.mutate(files[0]);
  }

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-semibold">Resumes</h1>
        <p className="text-sm text-[var(--lg-text-muted)]">PDF and DOCX only. Files go to object storage, not PostgreSQL.</p>
      </div>
      <GlassCard
        className={`p-8 text-center ${drag ? 'ring-2 ring-emerald-300/50' : ''}`}
        onDragOver={(e) => {
          e.preventDefault();
          setDrag(true);
        }}
        onDragLeave={() => setDrag(false)}
        onDrop={(e) => {
          e.preventDefault();
          setDrag(false);
          takeFiles(e.dataTransfer.files);
        }}
      >
        <p className="font-medium">Drop a resume here</p>
        <p className="mt-1 text-sm text-[var(--lg-text-muted)]">or choose a PDF / DOCX file</p>
        <label className="mt-4 inline-block">
          <input type="file" accept=".pdf,.docx" className="hidden" onChange={(e) => takeFiles(e.target.files)} />
          <span className="glass-btn glass-btn-primary inline-flex rounded-full px-4 py-2 text-sm">Upload resume</span>
        </label>
      </GlassCard>
      <input className="field" placeholder="Search resumes" value={search} onChange={(e) => setSearch(e.target.value)} />
      <div className="grid gap-3 md:grid-cols-2">
        {rows.map((resume) => (
          <GlassCard key={resume.id} className="p-5">
            <div className="font-medium">{resume.originalFilename}</div>
            <div className="mt-1 text-sm text-[var(--lg-text-muted)]">
              v{resume.currentVersion} · {(resume.sizeBytes / 1024).toFixed(0)} KB · {formatDate(resume.createdAt)}
            </div>
            <div className="mt-3 text-sm">
              {resume.candidate ? `${resume.candidate.firstName} ${resume.candidate.lastName}` : 'Unassigned candidate'}
            </div>
            <div className="mt-4 flex gap-2">
              <Button>Preview</Button>
              <Button>Download</Button>
            </div>
          </GlassCard>
        ))}
      </div>
    </div>
  );
}

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { api } from '../../services/api/client';
import { formatDate } from '../../utils/cn';

export function ResumesPage() {
  const client = useQueryClient();
  const [drag, setDrag] = useState(false);
  const resumes = useQuery({ queryKey: ['resumes'], queryFn: api.resumes });
  const upload = useMutation({
    mutationFn: (file: File) => api.uploadResume(file),
    onSuccess: () => client.invalidateQueries({ queryKey: ['resumes'] }),
  });

  function take(files: FileList | null) {
    if (files?.[0]) upload.mutate(files[0]);
  }

  return (
    <div className="space-y-5">
      <div>
        <h1 className="serif text-4xl">Your resumes</h1>
        <p className="mt-2 text-[var(--muted)]">PDF and DOCX. Versions stay with you, not in a recruiter silo.</p>
      </div>
      <div
        className={`panel rounded-[28px] p-10 text-center ${drag ? 'ring-1 ring-[var(--gold)]' : ''}`}
        onDragOver={(e) => {
          e.preventDefault();
          setDrag(true);
        }}
        onDragLeave={() => setDrag(false)}
        onDrop={(e) => {
          e.preventDefault();
          setDrag(false);
          take(e.dataTransfer.files);
        }}
      >
        <p className="serif text-2xl">Drop the file you actually send</p>
        <label className="mt-5 inline-block">
          <input type="file" accept=".pdf,.docx" className="hidden" onChange={(e) => take(e.target.files)} />
          <span className="btn btn-gold">Upload resume</span>
        </label>
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        {(resumes.data ?? []).map((resume) => (
          <article key={resume.id} className="panel rounded-[24px] p-5">
            <h2 className="font-medium">{resume.originalFilename}</h2>
            <p className="mt-2 text-sm text-[var(--muted)]">
              v{resume.currentVersion} · {(resume.sizeBytes / 1024).toFixed(0)} KB · {formatDate(resume.createdAt)}
            </p>
          </article>
        ))}
      </div>
    </div>
  );
}

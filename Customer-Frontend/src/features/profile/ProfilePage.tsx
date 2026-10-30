import { useForm } from 'react-hook-form';
import { useMutation } from '@tanstack/react-query';
import { api } from '../../services/api/client';
import { useAuth } from '../../app/providers/AuthProvider';

export function ProfilePage() {
  const { user } = useAuth();
  const form = useForm({
    defaultValues: {
      currentTitle: user?.candidate?.currentTitle ?? '',
      currentCompany: user?.candidate?.currentCompany ?? '',
      skills: (user?.candidate?.skills ?? []).join(', '),
    },
  });
  const save = useMutation({
    mutationFn: (values: { currentTitle: string; currentCompany: string; skills: string }) =>
      api.updateProfile({
        currentTitle: values.currentTitle,
        currentCompany: values.currentCompany,
        skills: values.skills.split(',').map((s) => s.trim()).filter(Boolean),
      }),
  });

  return (
    <div className="mx-auto max-w-xl space-y-5">
      <h1 className="serif text-4xl">Profile</h1>
      <form className="panel space-y-4 rounded-[28px] p-6" onSubmit={form.handleSubmit((v) => save.mutate(v))}>
        <input className="field" placeholder="Current title" {...form.register('currentTitle')} />
        <input className="field" placeholder="Current company" {...form.register('currentCompany')} />
        <input className="field" placeholder="Skills, comma separated" {...form.register('skills')} />
        <button className="btn btn-gold" type="submit">Save</button>
      </form>
    </div>
  );
}

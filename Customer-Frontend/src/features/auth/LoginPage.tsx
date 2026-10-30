import { useForm } from 'react-hook-form';
import { z } from 'zod';
import { zodResolver } from '@hookform/resolvers/zod';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../../app/providers/AuthProvider';
import { USE_MOCK } from '../../lib/config';

const schema = z.object({
  email: z.string().email(),
  password: z.string().min(8),
});
type Values = z.infer<typeof schema>;

export function LoginPage() {
  const { login } = useAuth();
  const navigate = useNavigate();
  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: { email: 'maya@studio.parakeet.ai', password: 'Studio123!' },
  });

  return (
    <div className="studio-bg flex min-h-svh items-center justify-center p-4">
      <form
        className="panel w-full max-w-md rounded-[32px] p-8"
        onSubmit={form.handleSubmit(async (values) => {
          await login(values.email, values.password);
          navigate('/app');
        })}
      >
        <p className="chip">Studio access</p>
        <h1 className="serif mt-4 text-3xl">Welcome back</h1>
        <p className="mt-2 text-sm text-[var(--muted)]">Your resumes and practice rooms are waiting.</p>
        <label className="mt-6 block text-sm">
          Email
          <input className="field mt-1" type="email" {...form.register('email')} />
        </label>
        <label className="mt-4 block text-sm">
          Password
          <input className="field mt-1" type="password" {...form.register('password')} />
        </label>
        <button className="btn btn-gold mt-6 w-full" type="submit">Enter studio</button>
        <p className="mt-4 text-sm text-[var(--muted)]">
          New here? <Link to="/register" className="text-[var(--gold)]">Create a room</Link>
        </p>
        {USE_MOCK && <p className="mt-3 text-xs text-[var(--muted)]">Mock mode — any matching form will sign you in as Maya.</p>}
      </form>
    </div>
  );
}

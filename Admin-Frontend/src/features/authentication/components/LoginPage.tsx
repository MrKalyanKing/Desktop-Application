import { useForm } from 'react-hook-form';
import { z } from 'zod';
import { zodResolver } from '@hookform/resolvers/zod';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../../../app/providers/AuthProvider';
import { Button } from '../../../components/ui/Button';
import { GlassCard } from '../../../components/ui/GlassCard';
import { USE_MOCK } from '../../../lib/config';

const schema = z.object({
  email: z.string().email(),
  password: z.string().min(8),
});

type FormValues = z.infer<typeof schema>;

export function LoginPage() {
  const { login } = useAuth();
  const navigate = useNavigate();
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: { email: 'admin@parakeet.ai', password: 'Admin123!' },
  });

  return (
    <div className="app-shell flex min-h-svh items-center justify-center p-4">
      <GlassCard strong className="w-full max-w-md p-8">
        <div className="mb-6">
          <div className="text-xs uppercase tracking-[0.22em] text-emerald-200/70">Parakeet AI</div>
          <h1 className="mt-2 text-2xl font-semibold">Interview Intelligence</h1>
          <p className="mt-2 text-sm text-[var(--lg-text-muted)]">
            Central dashboard for the KS Vision desktop assistant.
          </p>
        </div>
        <form
          className="space-y-4"
          onSubmit={form.handleSubmit(async (values) => {
            await login(values.email, values.password);
            navigate('/');
          })}
        >
          <label className="block text-sm">
            Email
            <input className="field mt-1" type="email" {...form.register('email')} />
          </label>
          <label className="block text-sm">
            Password
            <input className="field mt-1" type="password" {...form.register('password')} />
          </label>
          {form.formState.errors.email && (
            <p className="text-sm text-rose-300">{form.formState.errors.email.message}</p>
          )}
          <Button variant="primary" className="w-full" type="submit">
            Continue
          </Button>
        </form>
        <p className="mt-4 text-xs text-[var(--lg-text-muted)]">
          {USE_MOCK
            ? 'Running on mock data. Set VITE_USE_MOCK=false to use the NestJS API.'
            : 'Authenticated against /api/v1/auth. Refresh tokens stay in sessionStorage for this tab only.'}
        </p>
      </GlassCard>
    </div>
  );
}

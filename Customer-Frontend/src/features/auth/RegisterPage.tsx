import { useForm } from 'react-hook-form';
import { z } from 'zod';
import { zodResolver } from '@hookform/resolvers/zod';
import { Link, useNavigate } from 'react-router-dom';
import { useAuth } from '../../app/providers/AuthProvider';

const schema = z.object({
  firstName: z.string().min(1),
  lastName: z.string().min(1),
  email: z.string().email(),
  password: z.string().min(8),
});
type Values = z.infer<typeof schema>;

export function RegisterPage() {
  const { register: signup } = useAuth();
  const navigate = useNavigate();
  const form = useForm<Values>({ resolver: zodResolver(schema) });

  return (
    <div className="studio-bg flex min-h-svh items-center justify-center p-4">
      <form
        className="panel w-full max-w-md rounded-[32px] p-8"
        onSubmit={form.handleSubmit(async (values) => {
          await signup(values);
          navigate('/app');
        })}
      >
        <p className="chip">New studio</p>
        <h1 className="serif mt-4 text-3xl">Make it yours</h1>
        <div className="mt-6 grid gap-3 sm:grid-cols-2">
          <input className="field" placeholder="First name" {...form.register('firstName')} />
          <input className="field" placeholder="Last name" {...form.register('lastName')} />
        </div>
        <input className="field mt-3" type="email" placeholder="Email" {...form.register('email')} />
        <input className="field mt-3" type="password" placeholder="Password" {...form.register('password')} />
        <button className="btn btn-gold mt-6 w-full" type="submit">Create studio</button>
        <p className="mt-4 text-sm text-[var(--muted)]">
          Already practicing? <Link to="/login" className="text-[var(--gold)]">Sign in</Link>
        </p>
      </form>
    </div>
  );
}

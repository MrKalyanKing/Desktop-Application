import { NavLink, Outlet, useNavigate } from 'react-router-dom';
import {
  Activity,
  BarChart3,
  FileText,
  LayoutDashboard,
  Menu,
  MonitorSmartphone,
  Settings,
  Users,
  Video,
  X,
} from 'lucide-react';
import { useState } from 'react';
import { useAuth } from '../providers/AuthProvider';
import { USE_MOCK } from '../../lib/config';
import { Button } from '../../components/ui/Button';

const links = [
  { to: '/', label: 'Dashboard', icon: LayoutDashboard },
  { to: '/sessions', label: 'Sessions', icon: Video },
  { to: '/resumes', label: 'Resumes', icon: FileText },
  { to: '/candidates', label: 'Candidates', icon: Users },
  { to: '/analytics', label: 'Analytics', icon: BarChart3 },
  { to: '/reports', label: 'Reports', icon: Activity },
  { to: '/desktop', label: 'Desktop', icon: MonitorSmartphone },
  { to: '/settings', label: 'Settings', icon: Settings },
];

export function AppLayout() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);

  const nav = (
    <nav className="flex flex-col gap-1">
      {links.map((link) => (
        <NavLink
          key={link.to}
          to={link.to}
          end={link.to === '/'}
          onClick={() => setOpen(false)}
          className={({ isActive }) => `nav-link ${isActive ? 'is-active' : ''}`}
        >
          <link.icon size={16} />
          {link.label}
        </NavLink>
      ))}
    </nav>
  );

  return (
    <div className="app-shell">
      <div className="mx-auto flex min-h-svh max-w-[1440px] gap-4 p-3 md:p-5">
        <aside className="glass-strong hidden w-64 shrink-0 rounded-[28px] p-4 md:flex md:flex-col">
          <div className="mb-6 px-2">
            <div className="text-xs uppercase tracking-[0.2em] text-emerald-200/70">Parakeet AI</div>
            <div className="text-lg font-semibold">Interview Intelligence</div>
          </div>
          {nav}
          <div className="mt-auto pt-6 text-xs text-[var(--lg-text-muted)]">
            {USE_MOCK ? 'Mock data mode' : 'Live API'}
          </div>
        </aside>

        <div className="flex min-w-0 flex-1 flex-col gap-4">
          <header className="glass flex items-center justify-between rounded-[24px] px-4 py-3">
            <button className="glass-btn rounded-full p-2 md:hidden" onClick={() => setOpen(true)}>
              <Menu size={18} />
            </button>
            <div className="hidden text-sm text-[var(--lg-text-secondary)] md:block">
              Welcome back, {user?.firstName ?? 'Ava'}
            </div>
            <div className="flex items-center gap-3">
              <span className="badge badge-green">Org isolated</span>
              <Button
                onClick={() => {
                  logout();
                  navigate('/login');
                }}
              >
                Sign out
              </Button>
            </div>
          </header>
          <main className="min-h-0 flex-1 pb-8">
            <Outlet />
          </main>
        </div>
      </div>

      {open && (
        <div className="fixed inset-0 z-40 md:hidden">
          <button className="absolute inset-0 bg-black/50" onClick={() => setOpen(false)} />
          <aside className="glass-strong absolute left-3 top-3 bottom-3 w-[78%] max-w-xs rounded-[28px] p-4">
            <div className="mb-4 flex items-center justify-between">
              <span className="font-semibold">Menu</span>
              <button className="glass-btn rounded-full p-2" onClick={() => setOpen(false)}>
                <X size={16} />
              </button>
            </div>
            {nav}
          </aside>
        </div>
      )}
    </div>
  );
}

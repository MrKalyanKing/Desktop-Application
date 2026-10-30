import { NavLink, Outlet, useNavigate } from 'react-router-dom';
import { BarChart3, FileText, Home, Menu, Sparkles, UserRound, Video, X } from 'lucide-react';
import { useState } from 'react';
import { useAuth } from '../providers/AuthProvider';

const links = [
  { to: '/app', label: 'Home', icon: Home, end: true },
  { to: '/app/resumes', label: 'Resumes', icon: FileText },
  { to: '/app/interviews', label: 'Interviews', icon: Video },
  { to: '/app/practice', label: 'Practice', icon: Sparkles },
  { to: '/app/performance', label: 'Performance', icon: BarChart3 },
  { to: '/app/profile', label: 'Profile', icon: UserRound },
];

export function StudioLayout() {
  const { user, logout } = useAuth();
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);

  const nav = (
    <nav className="flex flex-col gap-1">
      {links.map((link) => (
        <NavLink
          key={link.to}
          to={link.to}
          end={link.end}
          onClick={() => setOpen(false)}
          className={({ isActive }) => `nav-item ${isActive ? 'is-active' : ''}`}
        >
          <link.icon size={16} />
          {link.label}
        </NavLink>
      ))}
    </nav>
  );

  return (
    <div className="studio-bg">
      <div className="mx-auto flex min-h-svh max-w-[1440px] gap-4 p-3 md:p-5">
        <aside className="panel hidden w-64 shrink-0 rounded-[28px] p-4 md:flex md:flex-col">
          <div className="mb-6 px-2">
            <div className="text-[11px] uppercase tracking-[0.28em] text-[var(--gold)]">Parakeet</div>
            <div className="serif text-xl">Studio</div>
          </div>
          {nav}
          <p className="mt-auto px-2 pt-6 text-xs text-[var(--muted)]">Your private practice room</p>
        </aside>
        <div className="flex min-w-0 flex-1 flex-col gap-4">
          <header className="panel flex items-center justify-between rounded-[24px] px-4 py-3">
            <button className="btn btn-ghost md:hidden" onClick={() => setOpen(true)}>
              <Menu size={16} />
            </button>
            <div className="hidden text-sm text-[var(--muted)] md:block">
              {user?.firstName ? `Hello, ${user.firstName}` : 'Welcome in'}
            </div>
            <button
              className="btn btn-ghost"
              onClick={() => {
                logout();
                navigate('/');
              }}
            >
              Sign out
            </button>
          </header>
          <main className="pb-10">
            <Outlet />
          </main>
        </div>
      </div>
      {open && (
        <div className="fixed inset-0 z-40 md:hidden">
          <button className="absolute inset-0 bg-black/50" onClick={() => setOpen(false)} />
          <aside className="panel absolute bottom-3 left-3 top-3 w-[78%] max-w-xs rounded-[28px] p-4">
            <div className="mb-4 flex items-center justify-between">
              <span className="serif text-lg">Studio</span>
              <button className="btn btn-ghost" onClick={() => setOpen(false)}><X size={16} /></button>
            </div>
            {nav}
          </aside>
        </div>
      )}
    </div>
  );
}

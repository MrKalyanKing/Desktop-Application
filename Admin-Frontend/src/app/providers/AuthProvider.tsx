import { createContext, useContext, useMemo, useState, type ReactNode } from 'react';
import { api, setAccessToken } from '../../services/api/client';
import { USE_MOCK } from '../../lib/config';
import type { AuthUser } from '../../types';

interface AuthContextValue {
  user: AuthUser | null;
  ready: boolean;
  login: (email: string, password: string) => Promise<void>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<AuthUser | null>(USE_MOCK ? null : null);
  const [ready] = useState(true);

  const value = useMemo<AuthContextValue>(
    () => ({
      user,
      ready,
      async login(email, password) {
        const result = await api.login(email, password);
        setUser(result.user);
      },
      logout() {
        api.logout();
        setAccessToken(null);
        setUser(null);
      },
    }),
    [user, ready],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('AuthProvider missing');
  return ctx;
}

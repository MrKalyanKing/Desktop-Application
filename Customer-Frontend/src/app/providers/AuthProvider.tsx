import { createContext, useContext, useMemo, useState, type ReactNode } from 'react';
import { api, setAccessToken } from '../../services/api/client';
import type { CustomerUser } from '../../types';

interface AuthContextValue {
  user: CustomerUser | null;
  login: (email: string, password: string) => Promise<void>;
  register: (body: { email: string; password: string; firstName: string; lastName: string }) => Promise<void>;
  logout: () => void;
}

const AuthContext = createContext<AuthContextValue | null>(null);

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<CustomerUser | null>(null);
  const value = useMemo<AuthContextValue>(
    () => ({
      user,
      async login(email, password) {
        const result = await api.login(email, password);
        setUser(result.user);
      },
      async register(body) {
        const result = await api.register(body);
        setUser(result.user);
      },
      logout() {
        api.logout();
        setAccessToken(null);
        setUser(null);
      },
    }),
    [user],
  );
  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const ctx = useContext(AuthContext);
  if (!ctx) throw new Error('AuthProvider missing');
  return ctx;
}

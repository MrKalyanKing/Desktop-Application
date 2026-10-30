import { Navigate, Outlet } from 'react-router-dom';
import { useAuth } from '../providers/AuthProvider';
import { USE_MOCK } from '../../lib/config';

export function RequireAuth() {
  const { user } = useAuth();
  if (!user && !USE_MOCK) return <Navigate to="/login" replace />;
  if (!user && USE_MOCK) return <Outlet />;
  return <Outlet />;
}

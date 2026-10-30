import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { AppLayout } from '../layouts/AppLayout';
import { RequireAuth } from './RequireAuth';
import { LoginPage } from '../../features/authentication/components/LoginPage';
import { DashboardPage } from '../../features/dashboard/components/DashboardPage';
import { SessionsPage } from '../../features/interview-sessions/components/SessionsPage';
import { SessionDetailPage } from '../../features/interview-sessions/components/SessionDetailPage';
import { ResumesPage } from '../../features/resumes/components/ResumesPage';
import { CandidatesPage } from '../../features/candidates/components/CandidatesPage';
import { AnalyticsPage } from '../../features/interview-analytics/components/AnalyticsPage';
import { ReportsPage } from '../../features/reports/components/ReportsPage';
import { DesktopPage } from '../../features/desktop-integration/components/DesktopPage';
import { SettingsPage } from '../../features/settings/components/SettingsPage';

const router = createBrowserRouter([
  { path: '/login', element: <LoginPage /> },
  {
    element: <RequireAuth />,
    children: [
      {
        element: <AppLayout />,
        children: [
          { path: '/', element: <DashboardPage /> },
          { path: '/sessions', element: <SessionsPage /> },
          { path: '/sessions/:id', element: <SessionDetailPage /> },
          { path: '/resumes', element: <ResumesPage /> },
          { path: '/candidates', element: <CandidatesPage /> },
          { path: '/analytics', element: <AnalyticsPage /> },
          { path: '/reports', element: <ReportsPage /> },
          { path: '/desktop', element: <DesktopPage /> },
          { path: '/settings', element: <SettingsPage /> },
        ],
      },
    ],
  },
]);

export function AppRouter() {
  return <RouterProvider router={router} />;
}

import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { StudioLayout } from '../layouts/StudioLayout';
import { RequireAuth } from './RequireAuth';
import { LandingPage } from '../../features/landing/LandingPage';
import { LoginPage } from '../../features/auth/LoginPage';
import { RegisterPage } from '../../features/auth/RegisterPage';
import { HomePage } from '../../features/home/HomePage';
import { ResumesPage } from '../../features/resumes/ResumesPage';
import { InterviewsPage } from '../../features/interviews/InterviewsPage';
import { InterviewRoomPage } from '../../features/interviews/InterviewRoomPage';
import { PracticePage } from '../../features/practice/PracticePage';
import { PerformancePage } from '../../features/performance/PerformancePage';
import { ProfilePage } from '../../features/profile/ProfilePage';

const router = createBrowserRouter([
  { path: '/', element: <LandingPage /> },
  { path: '/login', element: <LoginPage /> },
  { path: '/register', element: <RegisterPage /> },
  {
    element: <RequireAuth />,
    children: [
      {
        element: <StudioLayout />,
        children: [
          { path: '/app', element: <HomePage /> },
          { path: '/app/resumes', element: <ResumesPage /> },
          { path: '/app/interviews', element: <InterviewsPage /> },
          { path: '/app/interviews/:id', element: <InterviewRoomPage /> },
          { path: '/app/practice', element: <PracticePage /> },
          { path: '/app/performance', element: <PerformancePage /> },
          { path: '/app/profile', element: <ProfilePage /> },
        ],
      },
    ],
  },
]);

export function AppRouter() {
  return <RouterProvider router={router} />;
}

import { UserRole } from '../enums';

export interface AuthUser {
  id: string;
  email: string;
  organizationId: string;
  role: UserRole;
  deviceId?: string;
}

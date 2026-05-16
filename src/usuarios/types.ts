import { Role } from '../auth/types';

export interface Usuario {
  id: string;
  email: string;
  nombre: string;
  role: Role;
  sucursalId: string;
}

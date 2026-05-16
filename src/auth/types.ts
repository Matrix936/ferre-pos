export type Role = 'SUPERADMIN' | 'ADMIN' | 'USUARIO';

export interface User {
  id: string;
  email: string;
  nombre: string;
  role: Role;
  sucursalId: string; // Para el filtrado multisede
}

export interface AuthState {
  user: User | null;
  isAuthenticated: boolean;
  isLoading: boolean;
}

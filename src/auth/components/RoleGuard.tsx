import React from "react";
import { useAuth } from "../context/AuthContext";
import { Role } from "../types";

interface RoleGuardProps {
  children: React.ReactNode;
  allowedRoles: Role[];
}

export function RoleGuard({ children, allowedRoles }: RoleGuardProps) {
  const { user, isAuthenticated } = useAuth();

  // Si no hay usuario, no renderizamos nada (o podríamos redirigir al login)
  if (!isAuthenticated || !user) {
    return null;
  }

  // Ocultamos funciones que no corresponden al rol del usuario.
  if (!allowedRoles.includes(user.role)) {
    return null;
  }

  return <>{children}</>;
}

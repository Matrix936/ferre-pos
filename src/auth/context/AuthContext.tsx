import React, { createContext, useContext, useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { User, AuthState } from '../types';

interface AuthContextType extends AuthState {
  login: (user: User) => void;
  updateUser: (user: User) => void;
  logout: () => void;
}

const AuthContext = createContext<AuthContextType | undefined>(undefined);

export const AuthProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [state, setState] = useState<AuthState>({
    user: null,
    isAuthenticated: false,
    isLoading: true,
  });

  useEffect(() => {
    const verificarSesion = async () => {
      try {
        const user = await invoke<User | null>('get_sesion_actual');
        setState({
          user,
          isAuthenticated: Boolean(user),
          isLoading: false,
        });
      } catch (error) {
        console.error('Error al recuperar sesión actual:', error);
        setState({ user: null, isAuthenticated: false, isLoading: false });
      }
    };
    verificarSesion();
  }, []);

  const login = (user: User) => {
    setState({ user, isAuthenticated: true, isLoading: false });
  };

  const updateUser = (user: User) => {
    setState({ user, isAuthenticated: true, isLoading: false });
  };

  const logout = () => {
    setState({ user: null, isAuthenticated: false, isLoading: false });
  };

  return (
    <AuthContext.Provider value={{ ...state, login, updateUser, logout }}>
      {children}
    </AuthContext.Provider>
  );
};

// Hook personalizado para usar la seguridad fácilmente
export const useAuth = () => {
  const context = useContext(AuthContext);
  if (!context) throw new Error('useAuth debe usarse dentro de un AuthProvider');
  return context;
};

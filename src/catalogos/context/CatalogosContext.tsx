import React, { createContext, useCallback, useContext, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAuth } from '../../auth/context/AuthContext';
import { Categoria, Marca, Proveedor, UnidadMedida } from '../../inventario/types';
import { Sucursal } from '../../sucursales/types';
import { Usuario } from '../../usuarios/types';

interface CatalogosContextType {
  sucursales: Sucursal[];
  proveedores: Proveedor[];
  marcas: Marca[];
  categorias: Categoria[];
  unidades: UnidadMedida[];
  usuarios: Usuario[];
  isLoading: boolean;
  error: string | null;
  refreshCatalogos: () => Promise<void>;
}

const CatalogosContext = createContext<CatalogosContextType | undefined>(undefined);

export const CatalogosProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const { isAuthenticated } = useAuth();
  const [sucursales, setSucursales] = useState<Sucursal[]>([]);
  const [proveedores, setProveedores] = useState<Proveedor[]>([]);
  const [marcas, setMarcas] = useState<Marca[]>([]);
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [unidades, setUnidades] = useState<UnidadMedida[]>([]);
  const [usuarios, setUsuarios] = useState<Usuario[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refreshCatalogos = useCallback(async () => {
    if (!isAuthenticated) {
      setSucursales([]);
      setProveedores([]);
      setMarcas([]);
      setCategorias([]);
      setUnidades([]);
      setUsuarios([]);
      setError(null);
      return;
    }

    setIsLoading(true);
    setError(null);
    try {
      const [sucursalesData, proveedoresData, marcasData, categoriasData, unidadesData, usuariosData] = await Promise.all([
        invoke<Sucursal[]>('get_sucursales'),
        invoke<Proveedor[]>('get_proveedores'),
        invoke<Marca[]>('get_marcas'),
        invoke<Categoria[]>('get_categorias'),
        invoke<UnidadMedida[]>('get_unidades'),
        invoke<Usuario[]>('get_usuarios'),
      ]);
      setSucursales(sucursalesData);
      setProveedores(proveedoresData);
      setMarcas(marcasData);
      setCategorias(categoriasData);
      setUnidades(unidadesData);
      setUsuarios(usuariosData);
    } catch (err) {
      const message = String(err);
      setError(message);
      console.error('Error al cargar catálogos:', err);
    } finally {
      setIsLoading(false);
    }
  }, [isAuthenticated]);

  useEffect(() => {
    refreshCatalogos();
  }, [refreshCatalogos]);

  return (
    <CatalogosContext.Provider
      value={{
        sucursales,
        proveedores,
        marcas,
        categorias,
        unidades,
        usuarios,
        isLoading,
        error,
        refreshCatalogos,
      }}
    >
      {children}
    </CatalogosContext.Provider>
  );
};

export const useCatalogos = () => {
  const context = useContext(CatalogosContext);
  if (!context) throw new Error('useCatalogos debe usarse dentro de un CatalogosProvider');
  return context;
};

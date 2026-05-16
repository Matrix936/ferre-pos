import { 
  AppBar, 
  Toolbar, 
  Typography, 
  IconButton, 
  Box, 
  Avatar, 
  Menu, 
  MenuItem, 
  Tooltip,
  Divider,
  Chip
} from '@mui/material';
import { 
  Logout, 
  Settings, 
  Storefront, 
  Person,
  Menu as MenuIcon
} from '@mui/icons-material';
import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAuth } from '../../auth/context/AuthContext';
import { useConfig } from '../../config/context/ConfigContext';
import { useNavigate } from 'react-router-dom';
import { RoleGuard } from '../../auth/components/RoleGuard';
import { Sucursal } from '../../sucursales/types';

interface TopbarProps {
  onToggleSidebar: () => void;
}

export function Topbar({ onToggleSidebar }: TopbarProps) {
  const { user, logout } = useAuth();
  const { systemName } = useConfig();
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
  const [sucursales, setSucursales] = useState<Sucursal[]>([]);
  const navigate = useNavigate();

  useEffect(() => {
    const fetchSucursales = async () => {
      try {
        const data = await invoke<Sucursal[]>('get_sucursales');
        setSucursales(data);
      } catch (error) {
        console.error('Error al obtener sucursales para el topbar:', error);
      }
    };

    fetchSucursales();
  }, []);

  const sucursalNombre = useMemo(() => {
    const sucursal = sucursales.find((item) => item.id === user?.sucursalId);
    return sucursal?.nombre || 'Principal';
  }, [sucursales, user?.sucursalId]);
  
  const handleOpenMenu = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget);
  };

  const handleCloseMenu = () => {
    setAnchorEl(null);
  };

  const handleLogout = () => {
    handleCloseMenu();
    logout();
  };

  return (
    <AppBar 
      position="sticky" 
      elevation={0} 
      sx={{ 
        bgcolor: 'background.paper', 
        borderBottom: '1px solid',
        borderColor: 'divider',
        color: 'text.primary'
      }}
    >
      <Toolbar sx={{ justifyContent: 'space-between' }}>
        
        {/* Sección Izquierda: Logo y Sucursal */}
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
          <IconButton onClick={onToggleSidebar} edge="start" aria-label="toggle sidebar">
            <MenuIcon />
          </IconButton>
          <Typography variant="h6" sx={{ fontWeight: 700, color: 'primary.main', letterSpacing: -0.5 }}>
            {systemName}
          </Typography>
          <Divider orientation="vertical" flexItem sx={{ height: 24, my: 'auto' }} />
          <Chip 
            icon={<Storefront sx={{ fontSize: '1rem !important' }} />} 
            label={`Sucursal: ${sucursalNombre}`} 
            variant="outlined" 
            size="small"
            sx={{ borderRadius: '6px', fontWeight: 500 }}
          />
        </Box>

        {/* Sección Derecha: Usuario y Acciones */}
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <Box sx={{ mr: 2, textAlign: 'right', display: { xs: 'none', sm: 'block' } }}>
            <Typography variant="subtitle2" sx={{ lineHeight: 1, fontWeight: 600 }}>
              {user?.nombre}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              {user?.role}
            </Typography>
          </Box>

          <Tooltip title="Opciones de cuenta">
            <IconButton onClick={handleOpenMenu} sx={{ p: 0.5 }}>
              <Avatar 
                sx={{ 
                  width: 35, 
                  height: 35, 
                  bgcolor: 'primary.main',
                  fontSize: '0.9rem',
                  fontWeight: 600
                }}
              >
                {user?.nombre?.charAt(0).toUpperCase() || 'U'}
              </Avatar>
            </IconButton>
          </Tooltip>

          {/* Menú Desplegable */}
          <Menu
            anchorEl={anchorEl}
            open={Boolean(anchorEl)}
            onClose={handleCloseMenu}
            transformOrigin={{ horizontal: 'right', vertical: 'top' }}
            anchorOrigin={{ horizontal: 'right', vertical: 'bottom' }}
            slotProps={{
              paper: {
                elevation: 2,
                sx: { minWidth: 180, mt: 1.5, borderRadius: 2 }
              }
            }}
          >
            <MenuItem onClick={() => { handleCloseMenu(); navigate('/mi-perfil'); }} sx={{ gap: 1.5, py: 1.5 }}>
              <Person fontSize="small" color="action" /> Mi Perfil
            </MenuItem>
            <RoleGuard allowedRoles={["SUPERADMIN"]}>
              <MenuItem onClick={() => { handleCloseMenu(); navigate('/configuracion'); }} sx={{ gap: 1.5, py: 1.5 }}>
                <Settings fontSize="small" color="action" /> Configuración
              </MenuItem>
            </RoleGuard>
            <Divider />
            <MenuItem onClick={handleLogout} sx={{ gap: 1.5, py: 1.5, color: 'error.main' }}>
              <Logout fontSize="small" /> Cerrar Sesión
            </MenuItem>
          </Menu>
        </Box>

      </Toolbar>
    </AppBar>
  );
}

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
  Chip,
  Badge,
  Button,
  useTheme
} from '@mui/material';
import { invoke } from '@tauri-apps/api/core';
import { 
  Logout, 
  Settings, 
  Person,
  Menu as MenuIcon,
  Brightness4 as Brightness4Icon,
  Brightness7 as Brightness7Icon,
  DoneAll as DoneAllIcon,
  FiberManualRecord as FiberManualRecordIcon,
  Notifications as NotificationsIcon
} from '@mui/icons-material';
import { useContext, useEffect, useMemo, useState } from 'react';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { useNavigate } from 'react-router-dom';
import { RoleGuard } from '../../auth/components/RoleGuard';
import { ColorModeContext } from '../../theme';

interface TopbarProps {
  onToggleSidebar: () => void;
}

interface SyncStatus {
  pendientes: number;
  ventasPendientes?: number;
}

interface Notificacion {
  id: string;
  categoria: string;
  severidad: 'INFO' | 'WARNING' | 'CRITICAL';
  titulo: string;
  mensaje: string;
  leida: boolean;
  creadaAt: string;
}

const formatCurrentDateTime = (date: Date) => {
  const formattedDate = new Intl.DateTimeFormat('es-MX', {
    weekday: 'long',
    day: 'numeric',
    month: 'long',
  }).format(date);

  const formattedTime = new Intl.DateTimeFormat('es-MX', {
    hour: 'numeric',
    minute: '2-digit',
    second: '2-digit',
    hour12: true,
  }).format(date);

  return `${formattedDate.charAt(0).toUpperCase()}${formattedDate.slice(1)} - ${formattedTime.toUpperCase()}`;
};

export function Topbar({ onToggleSidebar }: TopbarProps) {
  const { user, logout } = useAuth();
  const theme = useTheme();
  const colorMode = useContext(ColorModeContext);
  const { sucursales } = useCatalogos();
  const [currentDateTime, setCurrentDateTime] = useState(() => formatCurrentDateTime(new Date()));
  const [syncStatus, setSyncStatus] = useState<SyncStatus>({ pendientes: 0 });
  const [notificaciones, setNotificaciones] = useState<Notificacion[]>([]);
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
  const [notificationsAnchorEl, setNotificationsAnchorEl] = useState<null | HTMLElement>(null);
  const navigate = useNavigate();

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      setCurrentDateTime(formatCurrentDateTime(new Date()));
    }, 1000);

    return () => window.clearInterval(intervalId);
  }, []);

  useEffect(() => {
    let active = true;

    const loadSyncStatus = async () => {
      try {
        const status = await invoke<SyncStatus>('get_sync_status');
        if (active) setSyncStatus(status);
      } catch (error) {
        console.error('Error al consultar sincronización:', error);
      }
    };

    loadSyncStatus();
    const intervalId = window.setInterval(loadSyncStatus, 15000);

    return () => {
      active = false;
      window.clearInterval(intervalId);
    };
  }, []);

  const loadNotificaciones = async () => {
    try {
      const data = await invoke<Notificacion[]>('get_notificaciones', { soloNoLeidas: false });
      setNotificaciones(data);
    } catch (error) {
      console.error('Error al consultar notificaciones:', error);
    }
  };

  useEffect(() => {
    loadNotificaciones();
    const intervalId = window.setInterval(loadNotificaciones, 15000);

    return () => window.clearInterval(intervalId);
  }, []);

  const sucursalNombre = useMemo(() => {
    const sucursal = sucursales.find((item) => item.id === user?.sucursalId);
    return sucursal?.nombre || 'Principal';
  }, [sucursales, user?.sucursalId]);

  const syncLabel = useMemo(() => {
    if (syncStatus.pendientes <= 0) return 'Sincronizado';
    if (syncStatus.ventasPendientes && syncStatus.ventasPendientes > 0) {
      return `${syncStatus.ventasPendientes} ${syncStatus.ventasPendientes === 1 ? 'venta' : 'ventas'}`;
    }
    return `${syncStatus.pendientes} ${syncStatus.pendientes === 1 ? 'pendiente' : 'pendientes'}`;
  }, [syncStatus]);

  const syncTooltip = useMemo(() => {
    if (syncStatus.pendientes <= 0) {
      return 'Todos los cambios locales están sincronizados con Supabase.';
    }

    const ventas = syncStatus.ventasPendientes ?? 0;
    const otros = Math.max(syncStatus.pendientes - ventas, 0);
    return `Pendientes por subir: ${syncStatus.pendientes}. Ventas: ${ventas}. Otros cambios: ${otros}.`;
  }, [syncStatus]);

  const unreadNotifications = useMemo(
    () => notificaciones.filter((notificacion) => !notificacion.leida).length,
    [notificaciones],
  );

  const severityColor = (severidad: Notificacion['severidad']) => {
    if (severidad === 'CRITICAL') return 'error.main';
    if (severidad === 'WARNING') return 'warning.main';
    return 'info.main';
  };
  
  const handleOpenMenu = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget);
  };

  const handleCloseMenu = () => {
    setAnchorEl(null);
  };

  const handleOpenNotifications = (event: React.MouseEvent<HTMLElement>) => {
    setNotificationsAnchorEl(event.currentTarget);
    loadNotificaciones();
  };

  const handleCloseNotifications = () => {
    setNotificationsAnchorEl(null);
  };

  const handleMarkNotificationRead = async (id: string) => {
    await invoke('marcar_notificacion_leida', { id });
    loadNotificaciones();
  };

  const handleMarkAllNotificationsRead = async () => {
    await invoke('marcar_todas_notificaciones_leidas');
    loadNotificaciones();
  };

  const handleLogout = async () => {
    handleCloseMenu();
    try {
      await logout();
    } catch (error) {
      window.alert(String(error));
    }
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
        
        {/* Sección Izquierda: Sucursal y sincronización */}
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.25, minWidth: 0 }}>
          <IconButton onClick={onToggleSidebar} edge="start" aria-label="toggle sidebar">
            <MenuIcon />
          </IconButton>
          <Tooltip title={`Sucursal activa: ${sucursalNombre}`}>
            <Box
              sx={{
                display: { xs: 'none', sm: 'flex' },
                alignItems: 'center',
                gap: 1,
                minHeight: 36,
                minWidth: 0,
                px: 1.25,
                border: '1px solid',
                borderColor: 'divider',
                borderRadius: '10px',
                bgcolor: 'background.paper',
                color: 'text.primary',
                overflow: 'hidden',
                transition: (muiTheme) => muiTheme.transitions.create(['background-color', 'border-color'], {
                  duration: 240,
                  easing: muiTheme.transitions.easing.easeInOut,
                }),
                '&:hover': {
                  borderColor: 'primary.main',
                  bgcolor: theme.palette.mode === 'dark' ? 'rgba(25, 118, 210, 0.12)' : 'rgba(25, 118, 210, 0.06)',
                },
              }}
            >
              <Box
                sx={{
                  width: 8,
                  height: 8,
                  borderRadius: '50%',
                  bgcolor: 'primary.main',
                  boxShadow: theme.palette.mode === 'dark'
                    ? '0 0 0 3px rgba(25, 118, 210, 0.18)'
                    : '0 0 0 3px rgba(25, 118, 210, 0.12)',
                  flex: '0 0 auto',
                }}
              />
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, minWidth: 0 }}>
                <Typography variant="body2" sx={{ fontWeight: 600, whiteSpace: 'nowrap' }}>
                  Sucursal
                </Typography>
                <Box
                  component="span"
                  sx={{
                    px: 0.75,
                    py: 0.25,
                    borderRadius: '6px',
                    bgcolor: theme.palette.mode === 'dark' ? 'rgba(25, 118, 210, 0.18)' : 'rgba(25, 118, 210, 0.1)',
                    color: 'primary.main',
                    fontSize: '0.65rem',
                    fontWeight: 800,
                    lineHeight: 1,
                    maxWidth: 150,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {sucursalNombre}
                </Box>
              </Box>
            </Box>
          </Tooltip>
          <Tooltip title={syncTooltip}>
            <Box
              sx={{
                display: { xs: 'none', md: 'flex' },
                alignItems: 'center',
                gap: 1,
                minHeight: 36,
                px: 1.25,
                border: '1px solid',
                borderColor: 'divider',
                borderRadius: '10px',
                bgcolor: 'background.paper',
                color: 'text.primary',
                overflow: 'hidden',
                transition: (muiTheme) => muiTheme.transitions.create(['background-color', 'border-color'], {
                  duration: 240,
                  easing: muiTheme.transitions.easing.easeInOut,
                }),
                '&:hover': {
                  borderColor: syncStatus.pendientes <= 0 ? 'success.main' : 'warning.main',
                  bgcolor: syncStatus.pendientes <= 0
                    ? theme.palette.mode === 'dark' ? 'rgba(46, 125, 50, 0.12)' : 'rgba(46, 125, 50, 0.06)'
                    : theme.palette.mode === 'dark' ? 'rgba(237, 108, 2, 0.14)' : 'rgba(237, 108, 2, 0.08)',
                },
              }}
            >
              <Box
                sx={{
                  width: 8,
                  height: 8,
                  borderRadius: '50%',
                  bgcolor: syncStatus.pendientes <= 0 ? 'success.main' : 'warning.main',
                  boxShadow: syncStatus.pendientes <= 0
                    ? '0 0 0 3px rgba(46, 125, 50, 0.12)'
                    : '0 0 0 3px rgba(237, 108, 2, 0.14)',
                  flex: '0 0 auto',
                }}
              />
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, minWidth: 0 }}>
                <Typography variant="body2" sx={{ fontWeight: 600, whiteSpace: 'nowrap' }}>
                  Sync
                </Typography>
                <Box
                  component="span"
                  sx={{
                    px: 0.75,
                    py: 0.25,
                    borderRadius: '6px',
                    bgcolor: syncStatus.pendientes <= 0
                      ? theme.palette.mode === 'dark' ? 'rgba(46, 125, 50, 0.18)' : 'rgba(46, 125, 50, 0.1)'
                      : theme.palette.mode === 'dark' ? 'rgba(237, 108, 2, 0.2)' : 'rgba(237, 108, 2, 0.12)',
                    color: syncStatus.pendientes <= 0 ? 'success.main' : 'warning.main',
                    fontSize: '0.65rem',
                    fontWeight: 800,
                    lineHeight: 1,
                    maxWidth: 100,
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    whiteSpace: 'nowrap',
                  }}
                >
                  {syncLabel}
                </Box>
              </Box>
            </Box>
          </Tooltip>
        </Box>

        {/* Sección Derecha: Usuario y Acciones */}
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>

          <Box 
            sx={{ 
              display: { xs: 'none', md: 'flex' }, 
              flexDirection: 'column', 
              alignItems: 'center', // Empuja el texto hacia la derecha para que rime con los iconos
              mr: 2 
            }}
          >
            <Typography variant="caption" color="text.secondary" sx={{ lineHeight: 1.1 }}>
              {currentDateTime.split(' - ')[0]} {/* Agarra solo la Fecha */}
            </Typography>
            <Typography variant="body2" color="text.primary" sx={{ fontWeight: 'bold', lineHeight: 1.1 }}>
              {currentDateTime.split(' - ')[1]} {/* Agarra solo la Hora */}
            </Typography>
          </Box>

          <Tooltip title={theme.palette.mode === 'dark' ? 'Cambiar a modo claro' : 'Cambiar a modo oscuro'}>
            <IconButton onClick={colorMode.toggleColorMode} color="inherit">
              {theme.palette.mode === 'dark' ? <Brightness7Icon /> : <Brightness4Icon />}
            </IconButton>
          </Tooltip>

          <Tooltip title="Centro de notificaciones">
            <IconButton onClick={handleOpenNotifications} color="inherit" aria-label="notificaciones">
              <Badge color="error" badgeContent={unreadNotifications} max={99}>
                <NotificationsIcon />
              </Badge>
            </IconButton>
          </Tooltip>

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

          <Menu
            anchorEl={notificationsAnchorEl}
            open={Boolean(notificationsAnchorEl)}
            onClose={handleCloseNotifications}
            transformOrigin={{ horizontal: 'right', vertical: 'top' }}
            anchorOrigin={{ horizontal: 'right', vertical: 'bottom' }}
            slotProps={{
              paper: {
                elevation: 3,
                sx: { width: 390, maxWidth: 'calc(100vw - 24px)', mt: 1.5, borderRadius: 2 }
              }
            }}
          >
            <Box sx={{ px: 2, py: 1.5, display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 1 }}>
              <Box>
                <Typography variant="subtitle1" sx={{ fontWeight: 800, lineHeight: 1.2 }}>
                  Notificaciones
                </Typography>
                <Typography variant="caption" color="text.secondary">
                  {unreadNotifications} pendientes por atender
                </Typography>
              </Box>
              <Button
                size="small"
                startIcon={<DoneAllIcon fontSize="small" />}
                onClick={handleMarkAllNotificationsRead}
                disabled={unreadNotifications === 0}
              >
                Leer todas
              </Button>
            </Box>
            <Divider />
            {notificaciones.length === 0 ? (
              <Box sx={{ px: 2, py: 3 }}>
                <Typography variant="body2" color="text.secondary">
                  Todo tranquilo por ahora.
                </Typography>
              </Box>
            ) : (
              notificaciones.slice(0, 8).map((notificacion) => (
                <MenuItem
                  key={notificacion.id}
                  onClick={() => handleMarkNotificationRead(notificacion.id)}
                  sx={{
                    alignItems: 'flex-start',
                    gap: 1.25,
                    py: 1.4,
                    whiteSpace: 'normal',
                    bgcolor: notificacion.leida ? 'transparent' : 'action.hover',
                  }}
                >
                  <FiberManualRecordIcon
                    sx={{
                      mt: 0.55,
                      fontSize: 12,
                      color: severityColor(notificacion.severidad),
                      flex: '0 0 auto',
                    }}
                  />
                  <Box sx={{ minWidth: 0 }}>
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 0.25, flexWrap: 'wrap' }}>
                      <Typography variant="body2" sx={{ fontWeight: 800, lineHeight: 1.25 }}>
                        {notificacion.titulo}
                      </Typography>
                      <Typography variant="caption" color="text.secondary" sx={{ textTransform: 'capitalize' }}>
                        {notificacion.categoria.toLowerCase()}
                      </Typography>
                    </Box>
                    <Typography variant="body2" color="text.secondary" sx={{ lineHeight: 1.35 }}>
                      {notificacion.mensaje}
                    </Typography>
                  </Box>
                </MenuItem>
              ))
            )}
          </Menu>

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
            <Box sx={{ px: 2, py: 1.5 }}>
              <Typography variant="subtitle1" sx={{ fontWeight: 700, lineHeight: 1.2 }}>
                {user?.nombre || 'Usuario'}
              </Typography>
              <Chip
                label={user?.role || 'Sin rol'}
                size="small"
                color="primary"
                variant="outlined"
                sx={{ mt: 0.75, height: 22, borderRadius: '6px', fontSize: '0.72rem' }}
              />
            </Box>
            <Divider />
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

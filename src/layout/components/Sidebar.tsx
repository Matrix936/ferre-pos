import { 
  Box, 
  List, 
  ListItem, 
  ListItemButton, 
  ListItemIcon, 
  ListItemText,
  Typography,
  Tooltip,
  alpha,
  keyframes
} from "@mui/material";
import { 
  Dashboard as DashboardIcon, 
  PointOfSale as SalesIcon, 
  Inventory as InventoryIcon, 
  Category as ProductosIcon,
  People as PeopleIcon,
  Store as StoreIcon,
  LocalShipping as ProveedoresIcon
  ,
  ShoppingCart as ComprasIcon
  ,
  AccountBalanceWallet as CajaIcon
  ,
  Badge as ClientesIcon,
  ReceiptLong as HistorialVentasIcon,
  RequestQuote as FacturacionIcon,
  SyncAlt as TraspasosIcon,
  ReportProblem as MermasIcon,
  Sell as MarcasIcon,
  SquareFoot as UnidadesIcon,
  Lock as LockIcon,
  LockOpen as LockOpenIcon
} from "@mui/icons-material";
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { NavLink } from "react-router-dom";
import { useAuth } from "../../auth/context/AuthContext";
import { RoleGuard } from "../../auth/components/RoleGuard";
import { useConfig } from "../../config/context/ConfigContext";
import logoDefecto from "../../images/logoDefecto.png";

const spin = keyframes`
  from { 
    transform: perspective(600px) rotateY(0deg); 
  }
  to { 
    transform: perspective(600px) rotateY(360deg); 
  }
`;

interface SidebarProps {
  isOpen: boolean;
  onClose: () => void;
}

interface CajaEstado {
  sesion: {
    estado: 'ABIERTA' | 'CERRADA';
  };
}

export function Sidebar({ isOpen, onClose }: SidebarProps) {
  const { user } = useAuth();
  const { systemName, logo } = useConfig();
  const [isCajaAbierta, setIsCajaAbierta] = useState(false);
  const displayLogo = logo || logoDefecto;

  useEffect(() => {
    if (!user?.id || !user?.sucursalId) {
      setIsCajaAbierta(false);
      return;
    }

    const fetchCajaActual = async () => {
      try {
        const data = await invoke<CajaEstado | null>('get_caja_actual', {
          usuarioId: user.id,
          sucursalId: user.sucursalId,
        });

        setIsCajaAbierta(data?.sesion.estado === 'ABIERTA');
      } catch (error) {
        console.error('Error al consultar estado de caja en el sidebar:', error);
        setIsCajaAbierta(false);
      }
    };

    fetchCajaActual();
    const intervalId = window.setInterval(fetchCajaActual, 5000);

    return () => window.clearInterval(intervalId);
  }, [user?.id, user?.sucursalId]);

  return (
    <Box 
      component="aside"
      sx={{ 
        width: isOpen ? 260 : 72,
        flexShrink: 0,
        bgcolor: 'background.paper', 
        borderRight: '1px solid',
        borderColor: 'divider',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
        height: '100%',
        zIndex: 10,
        overflow: 'hidden',
        transition: 'width 0.2s'
      }}
    >
      {/* Header del Sidebar */}
      <Box 
        sx={{ 
          height: 64, // Altura estándar del AppBar para que alinee con el Topbar
          display: 'flex', 
          alignItems: 'center', 
          justifyContent: 'center',
          borderBottom: '1px solid', 
          borderColor: 'divider',
          px: isOpen ? 2 : 1,
          py: 1,
          perspective: '1000px'
        }}
      >
        <Box 
          component="img" 
          src={displayLogo} 
          alt={systemName || "Ferre-POS"} 
          sx={{ 
            maxWidth: '100%', 
            maxHeight: '100%', 
            objectFit: 'contain',
            opacity: isOpen ? 1 : 0.85,
            animation: `${spin} 15s linear infinite`,
            transformStyle: 'preserve-3d',
            backfaceVisibility: 'visible'
          }} 
        />
      </Box>

      {/* Menú de Navegación */}
      <Box sx={{ flex: 1, py: 2, px: isOpen ? 1.5 : 1, overflowY: 'auto', overflowX: 'hidden' }}>
        <List
          sx={{
            display: 'flex',
            flexDirection: 'column',
            gap: 0.5,
            '& .MuiListItemButton-root': {
              justifyContent: isOpen ? 'flex-start' : 'center',
              px: isOpen ? 2 : 1,
              minHeight: 44,
            },
            '& .MuiListItemIcon-root': {
              minWidth: isOpen ? 40 : 0,
              justifyContent: 'center',
            },
            '& .MuiListItemText-root': {
              display: isOpen ? 'block' : 'none',
            },
          }}
        >
          
          <ListItem disablePadding>
            <Tooltip title={!isOpen ? 'Inicio' : ''} placement="right" arrow>
            <ListItemButton 
              component={NavLink}
              to="/"
              title={!isOpen ? 'Inicio' : undefined}
              onClick={onClose}
              sx={{ 
                borderRadius: '24px',
                mb: 0.5,
                '&.active': {
                  bgcolor: '#e8f0fe',
                  color: '#1a73e8',
                  '& .MuiListItemIcon-root': { color: '#1a73e8' },
                  '&:hover': { bgcolor: '#d2e3fc' }
                },
                '&:not(.active)': {
                  color: 'text.secondary',
                  '& .MuiListItemIcon-root': { color: 'text.secondary' },
                  '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                }
              }}
            >
              <ListItemIcon sx={{ minWidth: 40 }}><DashboardIcon /></ListItemIcon>
              <ListItemText 
                primary={
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    Inicio
                  </Typography>
                } 
              />
            </ListItemButton>
            </Tooltip>
          </ListItem>

          <ListItem disablePadding>
            <Tooltip title={!isOpen ? 'Ventas' : ''} placement="right" arrow>
            <ListItemButton 
              component={NavLink}
              to="/ventas"
              title={!isOpen ? 'Ventas' : undefined}
              onClick={onClose}
              sx={{ 
                borderRadius: '24px',
                mb: 0.5,
                '&.active': {
                  bgcolor: '#e8f0fe',
                  color: '#1a73e8',
                  '& .MuiListItemIcon-root': { color: '#1a73e8' },
                  '&:hover': { bgcolor: '#d2e3fc' }
                },
                '&:not(.active)': {
                  color: 'text.secondary',
                  '& .MuiListItemIcon-root': { color: 'text.secondary' },
                  '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                }
              }}
            >
              <ListItemIcon sx={{ minWidth: 40 }}><SalesIcon /></ListItemIcon>
              <ListItemText 
                primary={
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    Ventas
                  </Typography>
                } 
              />
            </ListItemButton>
            </Tooltip>
          </ListItem>

          <ListItem disablePadding>
            <Tooltip title={!isOpen ? 'Historial Ventas' : ''} placement="right" arrow>
            <ListItemButton
              component={NavLink}
              to="/ventas/historial"
              title={!isOpen ? 'Historial Ventas' : undefined}
              onClick={onClose}
              sx={{
                borderRadius: '24px',
                mb: 0.5,
                '&.active': {
                  bgcolor: '#e8f0fe',
                  color: '#1a73e8',
                  '& .MuiListItemIcon-root': { color: '#1a73e8' },
                  '&:hover': { bgcolor: '#d2e3fc' }
                },
                '&:not(.active)': {
                  color: 'text.secondary',
                  '& .MuiListItemIcon-root': { color: 'text.secondary' },
                  '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                }
              }}
            >
              <ListItemIcon sx={{ minWidth: 40 }}><HistorialVentasIcon /></ListItemIcon>
              <ListItemText
                primary={
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    Historial Ventas
                  </Typography>
                }
              />
            </ListItemButton>
            </Tooltip>
          </ListItem>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Facturación' : ''} placement="right" arrow>
              <ListItemButton
                component={NavLink}
                to="/facturacion"
                title={!isOpen ? 'Facturación' : undefined}
                onClick={onClose}
                sx={{
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><FacturacionIcon /></ListItemIcon>
                <ListItemText
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Facturación
                    </Typography>
                  }
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

          <ListItem disablePadding>
            <Tooltip title={!isOpen ? 'Caja' : ''} placement="right" arrow>
            <ListItemButton
              component={NavLink}
              to="/caja"
              title={!isOpen ? 'Caja' : undefined}
              onClick={onClose}
              sx={{
                borderRadius: '24px',
                mb: 0.5,
                '&.active': {
                  bgcolor: '#e8f0fe',
                  color: '#1a73e8',
                  '& .MuiListItemIcon-root': { color: '#1a73e8' },
                  '&:hover': { bgcolor: '#d2e3fc' }
                },
                '&:not(.active)': {
                  color: 'text.secondary',
                  '& .MuiListItemIcon-root': { color: 'text.secondary' },
                  '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                }
              }}
            >
              <ListItemIcon sx={{ minWidth: 40 }}><CajaIcon /></ListItemIcon>
              <ListItemText
                primary={
                  <Typography variant="body2" sx={{ fontWeight: 500 }}>
                    Caja
                  </Typography>
                }
              />
            </ListItemButton>
            </Tooltip>
          </ListItem>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Inventario' : ''} placement="right" arrow>
              <ListItemButton 
                component={NavLink}
                to="/inventario"
                title={!isOpen ? 'Inventario' : undefined}
                onClick={onClose}
                sx={{ 
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><InventoryIcon /></ListItemIcon>
                <ListItemText 
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Inventario
                    </Typography>
                  } 
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Productos' : ''} placement="right" arrow>
              <ListItemButton
                component={NavLink}
                to="/productos"
                title={!isOpen ? 'Productos' : undefined}
                onClick={onClose}
                sx={{
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><ProductosIcon /></ListItemIcon>
                <ListItemText
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Productos
                    </Typography>
                  }
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Proveedores' : ''} placement="right" arrow>
              <ListItemButton
                component={NavLink}
                to="/proveedores"
                title={!isOpen ? 'Proveedores' : undefined}
                onClick={onClose}
                sx={{
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><ProveedoresIcon /></ListItemIcon>
                <ListItemText
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Proveedores
                    </Typography>
                  }
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Marcas' : ''} placement="right" arrow>
              <ListItemButton
                component={NavLink}
                to="/marcas"
                title={!isOpen ? 'Marcas' : undefined}
                onClick={onClose}
                sx={{
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><MarcasIcon /></ListItemIcon>
                <ListItemText
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Marcas
                    </Typography>
                  }
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Unidades' : ''} placement="right" arrow>
              <ListItemButton
                component={NavLink}
                to="/unidades"
                title={!isOpen ? 'Unidades' : undefined}
                onClick={onClose}
                sx={{
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><UnidadesIcon /></ListItemIcon>
                <ListItemText
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Unidades
                    </Typography>
                  }
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Clientes' : ''} placement="right" arrow>
              <ListItemButton
                component={NavLink}
                to="/clientes"
                title={!isOpen ? 'Clientes' : undefined}
                onClick={onClose}
                sx={{
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><ClientesIcon /></ListItemIcon>
                <ListItemText
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Clientes
                    </Typography>
                  }
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Compras' : ''} placement="right" arrow>
              <ListItemButton
                component={NavLink}
                to="/compras"
                title={!isOpen ? 'Compras' : undefined}
                onClick={onClose}
                sx={{
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><ComprasIcon /></ListItemIcon>
                <ListItemText
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Compras
                    </Typography>
                  }
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Traspasos' : ''} placement="right" arrow>
              <ListItemButton
                component={NavLink}
                to="/traspasos"
                title={!isOpen ? 'Traspasos' : undefined}
                onClick={onClose}
                sx={{
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><TraspasosIcon /></ListItemIcon>
                <ListItemText
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Traspasos
                    </Typography>
                  }
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Mermas y Ajustes' : ''} placement="right" arrow>
              <ListItemButton
                component={NavLink}
                to="/inventario/mermas"
                title={!isOpen ? 'Mermas y Ajustes' : undefined}
                onClick={onClose}
                sx={{
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><MermasIcon /></ListItemIcon>
                <ListItemText
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Mermas y Ajustes
                    </Typography>
                  }
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Sucursales' : ''} placement="right" arrow>
              <ListItemButton 
                component={NavLink}
                to="/sucursales"
                title={!isOpen ? 'Sucursales' : undefined}
                onClick={onClose}
                sx={{ 
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><StoreIcon /></ListItemIcon>
                <ListItemText 
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Sucursales
                    </Typography>
                  } 
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <Tooltip title={!isOpen ? 'Usuarios' : ''} placement="right" arrow>
              <ListItemButton 
                component={NavLink}
                to="/usuarios"
                title={!isOpen ? 'Usuarios' : undefined}
                onClick={onClose}
                sx={{ 
                  borderRadius: '24px',
                  mb: 0.5,
                  '&.active': {
                    bgcolor: '#e8f0fe',
                    color: '#1a73e8',
                    '& .MuiListItemIcon-root': { color: '#1a73e8' },
                    '&:hover': { bgcolor: '#d2e3fc' }
                  },
                  '&:not(.active)': {
                    color: 'text.secondary',
                    '& .MuiListItemIcon-root': { color: 'text.secondary' },
                    '&:hover': { bgcolor: 'action.hover', color: 'text.primary' }
                  }
                }}
              >
                <ListItemIcon sx={{ minWidth: 40 }}><PeopleIcon /></ListItemIcon>
                <ListItemText 
                  primary={
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      Usuarios
                    </Typography>
                  } 
                />
              </ListItemButton>
              </Tooltip>
            </ListItem>
          </RoleGuard>

        </List>
      </Box>

      <Box
        sx={{
          mt: 'auto',
          mx: isOpen ? 1.5 : 1,
          mb: 1.5,
          px: isOpen ? 1.5 : 1,
          py: 1.25,
          borderTop: '1px solid',
          borderColor: 'divider',
          borderRadius: '8px',
          bgcolor: (theme) => alpha(isCajaAbierta ? theme.palette.success.light : theme.palette.error.light, 0.18),
          display: 'flex',
          alignItems: 'center',
          justifyContent: isOpen ? 'flex-start' : 'center',
          gap: 1.25
        }}
      >
        <Tooltip title={!isOpen ? (isCajaAbierta ? 'Caja Abierta' : 'Caja Cerrada') : ''} placement="right" arrow>
          {isCajaAbierta ? (
            <LockOpenIcon color="success" fontSize="small" />
          ) : (
            <LockIcon color="error" fontSize="small" />
          )}
        </Tooltip>
        <Typography
          variant="body2"
          sx={{
            display: isOpen ? 'block' : 'none',
            fontWeight: 700,
            color: isCajaAbierta ? 'success.main' : 'error.main'
          }}
        >
          {isCajaAbierta ? 'Caja Abierta' : 'Caja Cerrada'}
        </Typography>
      </Box>
    </Box>
  );
}

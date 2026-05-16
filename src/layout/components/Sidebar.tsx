import { 
  Box, 
  List, 
  ListItem, 
  ListItemButton, 
  ListItemIcon, 
  ListItemText,
  Typography,
  keyframes
} from "@mui/material";
import { 
  Dashboard as DashboardIcon, 
  PointOfSale as SalesIcon, 
  Inventory as InventoryIcon, 
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
  SyncAlt as TraspasosIcon,
  ReportProblem as MermasIcon
} from "@mui/icons-material";
import { NavLink } from "react-router-dom";
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

export function Sidebar({ isOpen, onClose }: SidebarProps) {
  const { systemName, logo } = useConfig();
  const displayLogo = logo || logoDefecto;

  return (
    <Box 
      component="aside"
      sx={{ 
        width: isOpen ? 260 : 0,
        bgcolor: 'background.paper', 
        borderRight: '1px solid',
        borderColor: 'divider',
        display: 'flex',
        flexDirection: 'column',
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
          px: 2,
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
            animation: `${spin} 15s linear infinite`,
            transformStyle: 'preserve-3d',
            backfaceVisibility: 'visible'
          }} 
        />
      </Box>

      {/* Menú de Navegación */}
      <Box sx={{ flex: 1, py: 2, px: 1.5, overflowY: 'auto' }}>
        <List sx={{ display: 'flex', flexDirection: 'column', gap: 0.5 }}>
          
          <ListItem disablePadding>
            <ListItemButton 
              component={NavLink}
              to="/"
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
          </ListItem>

          <ListItem disablePadding>
            <ListItemButton 
              component={NavLink}
              to="/ventas"
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
          </ListItem>

          <ListItem disablePadding>
            <ListItemButton
              component={NavLink}
              to="/ventas/historial"
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
          </ListItem>

          <ListItem disablePadding>
            <ListItemButton
              component={NavLink}
              to="/caja"
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
          </ListItem>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <ListItemButton 
                component={NavLink}
                to="/inventario"
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
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <ListItemButton
                component={NavLink}
                to="/proveedores"
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
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <ListItemButton
                component={NavLink}
                to="/clientes"
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
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <ListItemButton
                component={NavLink}
                to="/compras"
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
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <ListItemButton
                component={NavLink}
                to="/traspasos"
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
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <ListItemButton
                component={NavLink}
                to="/inventario/mermas"
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
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN"]}>
            <ListItem disablePadding>
              <ListItemButton 
                component={NavLink}
                to="/sucursales"
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
            </ListItem>
          </RoleGuard>

          <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
            <ListItem disablePadding>
              <ListItemButton 
                component={NavLink}
                to="/usuarios"
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
            </ListItem>
          </RoleGuard>

        </List>
      </Box>
    </Box>
  );
}

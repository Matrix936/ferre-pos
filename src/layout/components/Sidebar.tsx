import {
  alpha,
  Box,
  Divider,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  keyframes,
  Tooltip,
  Typography,
} from "@mui/material";
import {
  AccountBalanceWallet as CajaIcon,
  Badge as ClientesIcon,
  Class as CategoriasIcon,
  Category as ProductosIcon,
  Dashboard as DashboardIcon,
  Inventory as InventoryIcon,
  LocalOffer as PromocionesIcon,
  LocalShipping as ProveedoresIcon,
  People as PeopleIcon,
  PointOfSale as SalesIcon,
  ReceiptLong as HistorialVentasIcon,
  ReportProblem as MermasIcon,
  RequestQuote as FacturacionIcon,
  Sell as MarcasIcon,
  ShoppingCart as ComprasIcon,
  SquareFoot as UnidadesIcon,
  Store as StoreIcon,
  SyncAlt as TraspasosIcon,
} from "@mui/icons-material";
import { Theme } from "@mui/material/styles";
import { ReactNode, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { NavLink } from "react-router-dom";
import { Role } from "../../auth/types";
import { RoleGuard } from "../../auth/components/RoleGuard";
import { useAuth } from "../../auth/context/AuthContext";
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
    estado: "ABIERTA" | "CERRADA";
  };
}

interface NavItem {
  label: string;
  tooltip?: string;
  to: string;
  icon: ReactNode;
  allowedRoles?: Role[];
}

interface NavGroup {
  label: string;
  items: NavItem[];
}

const navGroups: NavGroup[] = [
  {
    label: "Operación",
    items: [
      { label: "Inicio", tooltip: "Inicio", to: "/", icon: <DashboardIcon /> },
      { label: "Ventas", tooltip: "Punto de venta", to: "/ventas", icon: <SalesIcon /> },
      { label: "Historial Ventas", tooltip: "Historial de ventas", to: "/ventas/historial", icon: <HistorialVentasIcon /> },
      { label: "Caja", tooltip: "Caja y corte", to: "/caja", icon: <CajaIcon /> },
    ],
  },
  {
    label: "Inventario",
    items: [
      { label: "Productos", tooltip: "Catálogo maestro de productos", to: "/productos", icon: <ProductosIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
      { label: "Inventario", tooltip: "Inventario por sucursal", to: "/inventario", icon: <InventoryIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
      { label: "Compras", tooltip: "Compras y entradas", to: "/compras", icon: <ComprasIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
      { label: "Traspasos", tooltip: "Traspasos entre sucursales", to: "/traspasos", icon: <TraspasosIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
      { label: "Mermas y Ajustes", tooltip: "Mermas y ajustes de inventario", to: "/inventario/mermas", icon: <MermasIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
    ],
  },
  {
    label: "Comercial",
    items: [
      { label: "Clientes", tooltip: "Clientes y crédito", to: "/clientes", icon: <ClientesIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
      { label: "Facturación", tooltip: "Facturación electrónica", to: "/facturacion", icon: <FacturacionIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
      { label: "Promociones", tooltip: "Promociones y descuentos", to: "/promociones", icon: <PromocionesIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
    ],
  },
  {
    label: "Catálogos",
    items: [
      { label: "Proveedores", tooltip: "Catálogo de proveedores", to: "/proveedores", icon: <ProveedoresIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
      { label: "Marcas", tooltip: "Catálogo de marcas", to: "/marcas", icon: <MarcasIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
      { label: "Categorías", tooltip: "Catálogo de categorías", to: "/categorias", icon: <CategoriasIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
      { label: "Unidades", tooltip: "Unidades de medida", to: "/unidades", icon: <UnidadesIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
    ],
  },
  {
    label: "Administración",
    items: [
      { label: "Usuarios", tooltip: "Gestión de usuarios", to: "/usuarios", icon: <PeopleIcon />, allowedRoles: ["SUPERADMIN", "ADMIN"] },
      { label: "Sucursales", tooltip: "Gestión de sucursales", to: "/sucursales", icon: <StoreIcon />, allowedRoles: ["SUPERADMIN"] },
    ],
  },
];

export function Sidebar({ isOpen, onClose }: SidebarProps) {
  const { user } = useAuth();
  const { systemName, logo, logoAnimationEnabled } = useConfig();
  const [isCajaAbierta, setIsCajaAbierta] = useState(false);
  const displayLogo = logo || logoDefecto;

  useEffect(() => {
    if (!user?.id || !user?.sucursalId) {
      setIsCajaAbierta(false);
      return;
    }

    const fetchCajaActual = async () => {
      try {
        const data = await invoke<CajaEstado | null>("get_caja_actual", {
          usuarioId: user.id,
          sucursalId: user.sucursalId,
        });

        setIsCajaAbierta(data?.sesion.estado === "ABIERTA");
      } catch (error) {
        console.error("Error al consultar estado de caja en el sidebar:", error);
        setIsCajaAbierta(false);
      }
    };

    fetchCajaActual();
    const intervalId = window.setInterval(fetchCajaActual, 5000);

    return () => window.clearInterval(intervalId);
  }, [user?.id, user?.sucursalId]);

  const navItemSx = {
    position: "relative",
    borderRadius: "10px",
    mb: 0.25,
    minHeight: 42,
    overflow: "hidden",
    "&::before": {
      content: '""',
      position: "absolute",
      left: 5,
      top: 10,
      bottom: 10,
      width: 3,
      borderRadius: 999,
      bgcolor: "primary.main",
      opacity: 0,
      transform: "scaleY(0.4)",
      transition: (theme: Theme) => theme.transitions.create(["opacity", "transform"], {
        duration: 180,
        easing: theme.transitions.easing.easeInOut,
      }),
    },
    "&.active": {
      bgcolor: (theme: Theme) => alpha(theme.palette.primary.main, theme.palette.mode === "dark" ? 0.22 : 0.1),
      color: "primary.main",
      "&::before": {
        opacity: 1,
        transform: "scaleY(1)",
      },
      "& .MuiListItemIcon-root": { color: "primary.main" },
      "&:hover": {
        bgcolor: (theme: Theme) => alpha(theme.palette.primary.main, theme.palette.mode === "dark" ? 0.28 : 0.16),
      },
    },
    "&:not(.active)": {
      color: "text.secondary",
      "& .MuiListItemIcon-root": { color: "text.secondary" },
      "&:hover": {
        bgcolor: "action.hover",
        color: "text.primary",
        "& .MuiListItemIcon-root": { color: "text.primary" },
      },
    },
  };

  const renderNavItem = (item: NavItem) => {
    const content = (
      <ListItem disablePadding key={item.to}>
        <Tooltip title={!isOpen ? item.tooltip || item.label : ""} placement="right" arrow>
          <ListItemButton
            component={NavLink}
            to={item.to}
            title={!isOpen ? item.tooltip || item.label : undefined}
            onClick={onClose}
            sx={navItemSx}
          >
            <ListItemIcon>{item.icon}</ListItemIcon>
            <ListItemText
              primary={
                <Typography variant="body2" sx={{ fontWeight: 600 }}>
                  {item.label}
                </Typography>
              }
            />
          </ListItemButton>
        </Tooltip>
      </ListItem>
    );

    return item.allowedRoles ? (
      <RoleGuard key={item.to} allowedRoles={item.allowedRoles}>
        {content}
      </RoleGuard>
    ) : content;
  };

  return (
    <Box
      component="aside"
      sx={{
        width: isOpen ? 260 : 72,
        flexShrink: 0,
        bgcolor: "background.paper",
        borderRight: "1px solid",
        borderColor: "divider",
        display: "flex",
        flexDirection: "column",
        justifyContent: "space-between",
        height: "100%",
        zIndex: 10,
        overflow: "hidden",
        transition: (theme) => theme.transitions.create("width", {
          duration: 280,
          easing: theme.transitions.easing.easeInOut,
        }),
        willChange: "width",
      }}
    >
      <Box
        sx={{
          height: 64,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          borderBottom: "1px solid",
          borderColor: "divider",
          px: isOpen ? 2 : 1,
          py: 1,
          transition: (theme) => theme.transitions.create("padding", {
            duration: 260,
            easing: theme.transitions.easing.easeInOut,
          }),
        }}
      >
        <Box
          component="img"
          src={displayLogo}
          alt={systemName || "Ferre-POS"}
          sx={{
            width: "auto",
            height: "auto",
            maxWidth: isOpen ? 180 : 44,
            maxHeight: 44,
            objectFit: "contain",
            opacity: isOpen ? 1 : 0.85,
            animation: logoAnimationEnabled ? `${spin} 15s linear infinite` : "none",
            transformStyle: "preserve-3d",
            backfaceVisibility: "visible",
            transition: (theme) => theme.transitions.create(["opacity", "transform"], {
              duration: 260,
              easing: theme.transitions.easing.easeInOut,
            }),
          }}
        />
      </Box>

      <Box
        sx={{
          flex: 1,
          py: 1.5,
          px: isOpen ? 1.25 : 1,
          overflowY: "auto",
          overflowX: "hidden",
          transition: (theme) => theme.transitions.create("padding", {
            duration: 260,
            easing: theme.transitions.easing.easeInOut,
          }),
        }}
      >
        <List
          sx={{
            display: "flex",
            flexDirection: "column",
            gap: 0.5,
            "& .MuiListItemButton-root": {
              justifyContent: isOpen ? "flex-start" : "center",
              px: isOpen ? 2 : 1,
              transition: (theme) => theme.transitions.create(["padding", "background-color", "color"], {
                duration: 240,
                easing: theme.transitions.easing.easeInOut,
              }),
            },
            "& .MuiListItemIcon-root": {
              minWidth: isOpen ? 38 : 0,
              justifyContent: "center",
              transition: (theme) => theme.transitions.create(["min-width", "color"], {
                duration: 260,
                easing: theme.transitions.easing.easeInOut,
              }),
            },
            "& .MuiListItemText-root": {
              m: 0,
              opacity: isOpen ? 1 : 0,
              maxWidth: isOpen ? 176 : 0,
              transform: isOpen ? "translateX(0)" : "translateX(-8px)",
              overflow: "hidden",
              whiteSpace: "nowrap",
              pointerEvents: isOpen ? "auto" : "none",
              transition: (theme) => theme.transitions.create(["opacity", "max-width", "transform"], {
                duration: isOpen ? 260 : 180,
                easing: theme.transitions.easing.easeInOut,
              }),
              "& .MuiTypography-root": {
                overflow: "hidden",
                textOverflow: "ellipsis",
                whiteSpace: "nowrap",
              },
            },
          }}
        >
          {navGroups.map((group, groupIndex) => (
            // CORREGIDO: Cambiamos component="li" por un Box neutral para evitar el nesting de <li>
            <Box key={group.label} sx={{ mb: 1 }}> 
              {groupIndex > 0 && <Divider sx={{ my: 1, mx: isOpen ? 1 : 1.5 }} />}
              <Typography
                variant="caption"
                sx={{
                  display: "block",
                  height: isOpen ? 24 : 8,
                  px: isOpen ? 1.5 : 0,
                  mb: isOpen ? 0.25 : 0,
                  opacity: isOpen ? 1 : 0,
                  color: "text.disabled",
                  fontWeight: 800,
                  textTransform: "uppercase",
                  overflow: "hidden",
                  whiteSpace: "nowrap",
                  transition: (theme) => theme.transitions.create(["height", "opacity", "margin", "padding"], {
                    duration: 220,
                    easing: theme.transitions.easing.easeInOut,
                  }),
                }}
              >
                {group.label}
              </Typography>
              {/* Aquí adentro tus items ya se renderizan como <li> válidos sin chocar */}
              {group.items.map(renderNavItem)}
            </Box>
          ))}
        </List>
      </Box>

      <Box component="div" sx={{ mt: "auto", p: 1.5 }}>
        <Tooltip title={!isOpen ? (isCajaAbierta ? "Caja abierta - Ir a caja" : "Caja cerrada - Abrir caja") : ""} placement="right" arrow>
          <ListItem disablePadding>
            <ListItemButton
              component={NavLink}
              to="/caja"
              onClick={onClose}
              sx={{
                minHeight: 44,
                borderRadius: "10px",
                border: "1px solid",
                borderColor: "divider",
                color: "text.primary",
                justifyContent: isOpen ? "flex-start" : "center",
                px: isOpen ? 1.5 : 1,
                overflow: "hidden",
                transition: (theme) => theme.transitions.create(["padding", "background-color", "border-color"], {
                  duration: 240,
                  easing: theme.transitions.easing.easeInOut,
                }),
                "&:hover": {
                  borderColor: isCajaAbierta ? "success.main" : "error.main",
                  bgcolor: (theme) => alpha(isCajaAbierta ? theme.palette.success.main : theme.palette.error.main, 0.08),
                },
              }}
            >
              <ListItemIcon
                sx={{
                  minWidth: isOpen ? 34 : 0,
                  justifyContent: "center",
                  transition: (theme) => theme.transitions.create("min-width", {
                    duration: 240,
                    easing: theme.transitions.easing.easeInOut,
                  }),
                }}
              >
                <Box
                  sx={{
                    width: 8,
                    height: 8,
                    borderRadius: "50%",
                    bgcolor: isCajaAbierta ? "success.main" : "error.main",
                    boxShadow: (theme) => `0 0 0 3px ${alpha(isCajaAbierta ? theme.palette.success.main : theme.palette.error.main, 0.12)}`,
                  }}
                />
              </ListItemIcon>
              <ListItemText
                primary={
                  <Box sx={{ display: "flex", alignItems: "center", justifyContent: "space-between", gap: 1 }}>
                    <Typography variant="body2" sx={{ fontWeight: 600 }}>
                      Caja
                    </Typography>
                    <Box
                      component="span"
                      sx={{
                        px: 0.75,
                        py: 0.25,
                        borderRadius: "6px",
                        bgcolor: (theme) => alpha(isCajaAbierta ? theme.palette.success.main : theme.palette.error.main, 0.12),
                        color: isCajaAbierta ? "success.main" : "error.main",
                        fontSize: "0.65rem",
                        fontWeight: 800,
                        lineHeight: 1,
                        opacity: isOpen ? 1 : 0,
                        maxWidth: isOpen ? 72 : 0,
                        overflow: "hidden",
                        whiteSpace: "nowrap",
                        transition: (theme) => theme.transitions.create(["opacity", "max-width"], {
                          duration: isOpen ? 240 : 160,
                          easing: theme.transitions.easing.easeInOut,
                        }),
                      }}
                    >
                      {isCajaAbierta ? "ABIERTA" : "CERRADA"}
                    </Box>
                  </Box>
                }
                sx={{
                  m: 0,
                  opacity: isOpen ? 1 : 0,
                  maxWidth: isOpen ? 176 : 0,
                  overflow: "hidden",
                  whiteSpace: "nowrap",
                  transition: (theme) => theme.transitions.create(["opacity", "max-width"], {
                    duration: isOpen ? 240 : 160,
                    easing: theme.transitions.easing.easeInOut,
                  }),
                }}
              />
            </ListItemButton>
          </ListItem>
        </Tooltip>
      </Box>
    </Box>
  );
}

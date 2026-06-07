import { lazy, Suspense } from "react";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { Box, CircularProgress, Typography } from "@mui/material";
import { AuthProvider, useAuth } from "./auth/context/AuthContext";
import { CatalogosProvider } from "./catalogos/context/CatalogosContext";
import { ConfigProvider } from "./config/context/ConfigContext";
import { LoginForm } from "./auth/components/LoginForm";
import { DashboardLayout } from "./layout/components/DashboardLayout";
import { RoleGuard } from "./auth/components/RoleGuard";

const DashboardView = lazy(() => import("./dashboard/components/DashboardView").then((module) => ({ default: module.DashboardView })));
const MiPerfilView = lazy(() => import("./perfil/components/MiPerfilView").then((module) => ({ default: module.MiPerfilView })));
const NuevaVenta = lazy(() => import("./ventas/components/NuevaVenta").then((module) => ({ default: module.NuevaVenta })));
const HistorialVentasView = lazy(() => import("./ventas/components/HistorialVentas").then((module) => ({ default: module.HistorialVentasView })));
const FacturacionHub = lazy(() => import("./facturacion/components/FacturacionHub").then((module) => ({ default: module.FacturacionHub })));
const CajaView = lazy(() => import("./caja/components/Caja").then((module) => ({ default: module.CajaView })));
const ClientesView = lazy(() => import("./clientes/components/Clientes").then((module) => ({ default: module.ClientesView })));
const NuevoTraspasoView = lazy(() => import("./traspasos/components/NuevoTraspaso").then((module) => ({ default: module.NuevoTraspasoView })));
const InventarioView = lazy(() => import("./inventario/components/InventarioView").then((module) => ({ default: module.InventarioView })));
const ProductosView = lazy(() => import("./productos/components/ProductosView").then((module) => ({ default: module.ProductosView })));
const PromocionesView = lazy(() => import("./promociones/components/PromocionesView").then((module) => ({ default: module.PromocionesView })));
const RentabilidadView = lazy(() => import("./indicadores/components/RentabilidadView").then((module) => ({ default: module.RentabilidadView })));
const IndicadorVentasView = lazy(() => import("./indicadores/components/IndicadorVentasView").then((module) => ({ default: module.IndicadorVentasView })));
const IndicadorInventarioView = lazy(() => import("./indicadores/components/IndicadorInventarioView").then((module) => ({ default: module.IndicadorInventarioView })));
const IndicadorFinancieroView = lazy(() => import("./indicadores/components/IndicadorFinancieroView").then((module) => ({ default: module.IndicadorFinancieroView })));
const MermasAjustesView = lazy(() => import("./inventario/components/MermasAjustes").then((module) => ({ default: module.MermasAjustesView })));
const ProveedoresView = lazy(() => import("./proveedores/components/ProveedoresView").then((module) => ({ default: module.ProveedoresView })));
const MarcasView = lazy(() => import("./catalogos/components/MarcasView").then((module) => ({ default: module.MarcasView })));
const CategoriasView = lazy(() => import("./catalogos/components/CategoriasView").then((module) => ({ default: module.CategoriasView })));
const UnidadesView = lazy(() => import("./catalogos/components/UnidadesView").then((module) => ({ default: module.UnidadesView })));
const NuevaCompra = lazy(() => import("./compras/components/NuevaCompra").then((module) => ({ default: module.NuevaCompra })));
const SucursalesView = lazy(() => import("./sucursales/components/SucursalesView").then((module) => ({ default: module.SucursalesView })));
const UsuariosView = lazy(() => import("./usuarios/components/UsuariosView").then((module) => ({ default: module.UsuariosView })));
const ConfiguracionView = lazy(() => import("./config/components/ConfiguracionView").then((module) => ({ default: module.ConfiguracionView })));

function RouteFallback() {
  return (
    <Box sx={{ minHeight: 220, display: "grid", placeItems: "center", color: "text.secondary" }}>
      <Box sx={{ display: "flex", alignItems: "center", gap: 1.5 }}>
        <CircularProgress size={20} />
        <Typography variant="body2">Cargando...</Typography>
      </Box>
    </Box>
  );
}

// Componente interno para manejar la lógica de rutas
function Root() {
  const { isAuthenticated } = useAuth();

  if (!isAuthenticated) {
    return <LoginForm />;
  }

  return (
    <DashboardLayout>
      <Suspense fallback={<RouteFallback />}>
        <Routes>
          <Route path="/" element={<DashboardView />} />
          <Route path="/mi-perfil" element={<MiPerfilView />} />
          <Route path="/ventas" element={<NuevaVenta />} />
          <Route path="/ventas/historial" element={<HistorialVentasView />} />
          <Route path="/facturacion" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <FacturacionHub />
            </RoleGuard>
          } />
          <Route path="/caja" element={<CajaView />} />
          <Route path="/clientes" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <ClientesView />
            </RoleGuard>
          } />
          <Route path="/traspasos" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <NuevoTraspasoView />
            </RoleGuard>
          } />
          <Route path="/inventario" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <InventarioView />
            </RoleGuard>
          } />
          <Route path="/productos" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <ProductosView />
            </RoleGuard>
          } />
          <Route path="/promociones" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <PromocionesView />
            </RoleGuard>
          } />
          <Route path="/indicadores/rentabilidad" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <RentabilidadView />
            </RoleGuard>
          } />
          <Route path="/indicadores/ventas" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <IndicadorVentasView />
            </RoleGuard>
          } />
          <Route path="/indicadores/inventario" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <IndicadorInventarioView />
            </RoleGuard>
          } />
          <Route path="/indicadores/financiero" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <IndicadorFinancieroView />
            </RoleGuard>
          } />
          <Route path="/inventario/mermas" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <MermasAjustesView />
            </RoleGuard>
          } />
          <Route path="/proveedores" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <ProveedoresView />
            </RoleGuard>
          } />
          <Route path="/marcas" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <MarcasView />
            </RoleGuard>
          } />
          <Route path="/categorias" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <CategoriasView />
            </RoleGuard>
          } />
          <Route path="/unidades" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <UnidadesView />
            </RoleGuard>
          } />
          <Route path="/compras" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <NuevaCompra />
            </RoleGuard>
          } />
          <Route path="/sucursales" element={
            <RoleGuard allowedRoles={["SUPERADMIN"]}>
              <SucursalesView />
            </RoleGuard>
          } />
          <Route path="/usuarios" element={
            <RoleGuard allowedRoles={["SUPERADMIN", "ADMIN"]}>
              <UsuariosView />
            </RoleGuard>
          } />
          <Route path="/configuracion" element={
            <RoleGuard allowedRoles={["SUPERADMIN"]}>
              <ConfiguracionView />
            </RoleGuard>
          } />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Suspense>
    </DashboardLayout>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <ConfigProvider>
        <AuthProvider>
          <CatalogosProvider>
            <Root />
          </CatalogosProvider>
        </AuthProvider>
      </ConfigProvider>
    </BrowserRouter>
  );
}

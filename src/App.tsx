import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import { AuthProvider, useAuth } from "./auth/context/AuthContext";
import { CatalogosProvider } from "./catalogos/context/CatalogosContext";
import { ConfigProvider } from "./config/context/ConfigContext";
import { LoginForm } from "./auth/components/LoginForm";
import { DashboardLayout } from "./layout/components/DashboardLayout";
import { DashboardView } from "./dashboard/components/DashboardView";
import { ConfiguracionView } from "./config/components/ConfiguracionView";
import { UsuariosView } from "./usuarios/components/UsuariosView";
import { SucursalesView } from "./sucursales/components/SucursalesView";
import { RoleGuard } from "./auth/components/RoleGuard";
import { MiPerfilView } from "./perfil/components/MiPerfilView";
import { InventarioView } from "./inventario/components/InventarioView";
import { ProductosView } from "./productos/components/ProductosView";
import { PromocionesView } from "./promociones/components/PromocionesView";
import { MermasAjustesView } from "./inventario/components/MermasAjustes";
import { ProveedoresView } from "./proveedores/components/ProveedoresView";
import { MarcasView } from "./catalogos/components/MarcasView";
import { CategoriasView } from "./catalogos/components/CategoriasView";
import { UnidadesView } from "./catalogos/components/UnidadesView";
import { NuevaCompra } from "./compras/components/NuevaCompra";
import { NuevaVenta } from "./ventas/components/NuevaVenta";
import { HistorialVentasView } from "./ventas/components/HistorialVentas";
import { FacturacionHub } from "./facturacion/components/FacturacionHub";
import { CajaView } from "./caja/components/Caja";
import { ClientesView } from "./clientes/components/Clientes";
import { NuevoTraspasoView } from "./traspasos/components/NuevoTraspaso";

// Componente interno para manejar la lógica de rutas
function Root() {
  const { isAuthenticated } = useAuth();

  if (!isAuthenticated) {
    return <LoginForm />;
  }

  return (
    <DashboardLayout>
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

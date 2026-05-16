export interface ProductoInventario {
  id: string;
  codigoBarras: string;
  codigoProveedor: string;
  proveedorId: string;
  claveProducto: string;
  descripcion: string;
  marca: string;
  categoria: string;
  unidad: string;
  precioCosto: number;
  precioVenta: number;
  sucursalId: string;
  stock: number;
  stockMinimo: number;
}

export interface ProductoPayload {
  id: string;
  codigoBarras: string;
  codigoProveedor: string;
  proveedorId: string;
  claveProducto: string;
  descripcion: string;
  marca: string;
  categoria: string;
  unidad: string;
  precioCosto: number;
  precioVenta: number;
}

export interface InventarioSucursalPayload {
  sucursalId: string;
  stock: number;
  stockMinimo: number;
}

export interface Proveedor {
  id: string;
  nombre: string;
  contactoNombre: string;
  telefono: string;
  email: string;
  direccion: string;
}

export interface CompraDetallePayload {
  id: string;
  productoId: string;
  cantidad: number;
  precioCostoPactado: number;
}

export interface RegistrarCompraPayload {
  id: string;
  proveedorId: string;
  sucursalId: string;
  fecha: string;
  detalles: CompraDetallePayload[];
}

export interface VentaDetallePayload {
  id: string;
  productoId: string;
  cantidad: number;
  precioVentaPactado: number;
}

export interface RegistrarVentaPayload {
  id: string;
  usuarioId: string;
  sucursalId: string;
  fecha: string;
  metodoPago: string;
  clienteId?: string;
  detalles: VentaDetallePayload[];
}

export interface Cliente {
  id: string;
  nombre: string;
  telefono: string;
  direccion: string;
  limiteCredito: number;
  saldoDeudor: number;
}

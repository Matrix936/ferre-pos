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
  satClaveProdServ: string;
  satClaveUnidad: string;
  sucursalId: string;
  stock: number;
  stockMinimo: number;
  costoPromedio: number;
  precioOriginal?: number | null;
  precioDescontado?: number | null;
  nombrePromo?: string | null;
  promocionId?: string | null;
  promoTipoDescuento?: string | null;
  promoValor?: number | null;
  precio1?: number;
  precio2?: number;
  precio3?: number;
  precio4?: number;
  mayoreoApartir?: number;
  aGranel?: boolean;
  noEnCatalogo?: boolean;
  ventasNegativas?: boolean;
  caducidad?: string | null;
  fotos?: string;
  descripcionCatalogo?: string;
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
  satClaveProdServ: string;
  satClaveUnidad: string;
  precio1?: number;
  precio2?: number;
  precio3?: number;
  precio4?: number;
  mayoreoApartir?: number;
  aGranel?: boolean;
  noEnCatalogo?: boolean;
  ventasNegativas?: boolean;
  caducidad?: string | null;
  fotos?: string;
  descripcionCatalogo?: string;
}

export type ProductoCatalogo = ProductoPayload;

export interface ProductoCatalogoPage {
  rows: ProductoCatalogo[];
  total: number;
}

export interface ProductoInventarioPage {
  rows: ProductoInventario[];
  total: number;
}

export interface InventarioSucursalPayload {
  sucursalId: string;
  stock: number;
  stockMinimo: number;
  precioVenta: number;
  costoPromedio: number;
}

export interface Proveedor {
  id: string;
  nombre: string;
  contactoNombre: string;
  telefono: string;
  email: string;
  direccion: string;
}

export interface Marca {
  id: string;
  nombre: string;
}

export interface Categoria {
  id: string;
  nombre: string;
}

export interface UnidadMedida {
  id: string;
  nombre: string;
  claveSat: string;
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
  tipoPrecioVendido?: string;
  precioOriginal?: number;
  descuentoAplicado?: number;
}

export interface RegistrarVentaPayload {
  id: string;
  usuarioId: string;
  sucursalId: string;
  fecha: string;
  metodoPago: string;
  clienteId?: string;
  efectivoRecibido?: number;
  cambioEntregado?: number;
  clienteRapidoNombre?: string;
  clienteRapidoTelefono?: string;
  clienteRapidoDomicilio?: string;
  requiereFactura?: boolean;
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

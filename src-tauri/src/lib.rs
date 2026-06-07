use bcrypt::{hash, verify, DEFAULT_COST};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::types::{Value, ValueRef};
use rusqlite::{params, params_from_iter, Connection, Error as SqliteError, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

type DbPool = Pool<SqliteConnectionManager>;
type AppResult<T> = Result<T, String>;

const BACKUP_TABLES: &[&str] = &[
    "sucursales",
    "empresa_config_fiscal",
    "usuarios",
    "proveedores",
    "marcas",
    "categorias",
    "unidades",
    "productos",
    "inventario_sucursal",
    "clientes",
    "clientes_datos_fiscales",
    "compras",
    "detalle_compras",
    "ventas",
    "detalle_ventas",
    "creditos_abonos",
    "cajas_sesiones",
    "caja_movimientos",
    "traspasos",
    "detalle_traspasos",
    "mermas_ajustes",
    "movimientos_inventario",
    "facturas_emitidas",
    "promociones",
    "promocion_sucursales",
];

const LOCAL_ONLY_BACKUP_TABLES: &[&str] = &["notificaciones"];

const SYNC_TABLES: &[&str] = &[
    "sucursales",
    "empresa_config_fiscal",
    "usuarios",
    "proveedores",
    "marcas",
    "categorias",
    "unidades",
    "productos",
    "inventario_sucursal",
    "clientes",
    "clientes_datos_fiscales",
    "compras",
    "detalle_compras",
    "ventas",
    "detalle_ventas",
    "creditos_abonos",
    "cajas_sesiones",
    "caja_movimientos",
    "traspasos",
    "detalle_traspasos",
    "mermas_ajustes",
    "movimientos_inventario",
    "facturas_emitidas",
    "promociones",
    "promocion_sucursales",
];

const PULL_TABLES: &[&str] = &[
    "sucursales",
    "empresa_config_fiscal",
    "usuarios",
    "proveedores",
    "marcas",
    "categorias",
    "unidades",
    "productos",
    "inventario_sucursal",
    "clientes",
    "clientes_datos_fiscales",
    "promociones",
    "promocion_sucursales",
    "compras",
    "detalle_compras",
    "ventas",
    "detalle_ventas",
    "creditos_abonos",
    "cajas_sesiones",
    "caja_movimientos",
    "traspasos",
    "detalle_traspasos",
    "mermas_ajustes",
    "movimientos_inventario",
    "facturas_emitidas",
];

const UUID_SYNC_TABLES: &[&str] = &[
    "compras",
    "detalle_compras",
    "ventas",
    "detalle_ventas",
    "creditos_abonos",
    "cajas_sesiones",
    "caja_movimientos",
    "traspasos",
    "detalle_traspasos",
    "mermas_ajustes",
    "facturas_emitidas",
];

const SOFT_DELETE_TABLES: &[&str] = &[
    "sucursales",
    "usuarios",
    "proveedores",
    "marcas",
    "categorias",
    "unidades",
    "productos",
    "inventario_sucursal",
    "clientes",
    "promociones",
];

const AUTO_SYNC_BATCH_SIZE: usize = 50;
#[derive(Debug)]
enum AppError {
    Db(String),
    Pool(String),
    Auth(String),
    Validation(String),
    Conflict(String),
    Crypto(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Db(message) => write!(f, "Error de base de datos: {message}"),
            AppError::Pool(message) => write!(f, "No se pudo obtener conexión a la base de datos: {message}"),
            AppError::Auth(message) => write!(f, "{message}"),
            AppError::Validation(message) => write!(f, "{message}"),
            AppError::Conflict(message) => write!(f, "{message}"),
            AppError::Crypto(message) => write!(f, "Error de seguridad al procesar contraseña: {message}"),
        }
    }
}

impl From<SqliteError> for AppError {
    fn from(error: SqliteError) -> Self {
        match error {
            SqliteError::QueryReturnedNoRows => AppError::Auth("Credenciales inválidas o usuario no encontrado.".to_string()),
            SqliteError::SqliteFailure(_, Some(message)) => AppError::Db(message),
            other => AppError::Db(other.to_string()),
        }
    }
}

impl From<r2d2::Error> for AppError {
    fn from(error: r2d2::Error) -> Self {
        AppError::Pool(error.to_string())
    }
}

impl From<bcrypt::BcryptError> for AppError {
    fn from(error: bcrypt::BcryptError) -> Self {
        AppError::Crypto(error.to_string())
    }
}

fn to_command_error(error: AppError) -> String {
    error.to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Usuario {
    id: String,
    email: String,
    nombre: String,
    role: String,
    sucursal_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Sucursal {
    id: String,
    nombre: String,
    direccion: String,
    telefono: String,
    #[serde(default)]
    codigo_postal: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Proveedor {
    id: String,
    nombre: String,
    contacto_nombre: String,
    telefono: String,
    email: String,
    direccion: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Marca {
    id: String,
    nombre: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Categoria {
    id: String,
    nombre: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UnidadMedida {
    id: String,
    nombre: String,
    clave_sat: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Producto {
    id: String,
    codigo_barras: String,
    codigo_proveedor: String,
    proveedor_id: String,
    clave_producto: String,
    descripcion: String,
    marca: String,
    categoria: String,
    unidad: String,
    precio_costo: f64,
    #[serde(default)]
    costo_promedio: f64,
    precio_venta: f64,
    #[serde(default)]
    sat_clave_prod_serv: String,
    #[serde(default)]
    sat_clave_unidad: String,
    #[serde(default)]
    precio_1: f64,
    #[serde(default)]
    precio_2: f64,
    #[serde(default)]
    precio_3: f64,
    #[serde(default)]
    precio_4: f64,
    #[serde(default)]
    mayoreo_apartir: f64,
    #[serde(default)]
    a_granel: bool,
    #[serde(default)]
    no_en_catalogo: bool,
    #[serde(default)]
    ventas_negativas: bool,
    #[serde(default)]
    caducidad: Option<String>,
    #[serde(default)]
    fotos: String,
    #[serde(default)]
    descripcion_catalogo: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventarioSucursalInput {
    sucursal_id: String,
    stock: f64,
    stock_minimo: f64,
    #[serde(default)]
    precio_venta: f64,
    #[serde(default)]
    costo_promedio: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct LegacyArticuloImportRow {
    id: Option<i64>,
    clave: Option<String>,
    descripcion_articulo: Option<String>,
    unidad: Option<String>,
    codigo_barra: Option<String>,
    existencia_stock: Option<f64>,
    caducidad: Option<String>,
    provedor: Option<i64>,
    #[serde(default)]
    proveedor_nombre: Option<String>,
    categoria: Option<String>,
    marca: Option<i64>,
    #[serde(default)]
    marca_nombre: Option<String>,
    fotos: Option<String>,
    descripcion_catalogo: Option<String>,
    precio_compra: Option<f64>,
    precio_venta: Option<f64>,
    precio_1: Option<f64>,
    precio_2: Option<f64>,
    precio_3: Option<f64>,
    precio_4: Option<f64>,
    mayoreo_apartir: Option<f64>,
    cant_min_stock: Option<f64>,
    a_granel: Option<String>,
    no_en_catalogo: Option<String>,
    ventas_negativas: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportarArticulosLegacyInput {
    sucursal_id: String,
    proveedor_default_id: String,
    rows: Vec<LegacyArticuloImportRow>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportarArticulosLegacyResult {
    total_leidos: usize,
    productos_upsertados: usize,
    inventario_upsertado: usize,
    catalogos_actualizados: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportarDatosUniversalInput {
    destino: String,
    rows: Vec<HashMap<String, JsonValue>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportarDatosUniversalResult {
    destino: String,
    total_leidos: usize,
    registros_upsertados: usize,
    omitidos: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportarCsvProductosMapeadoInput {
    file_path: String,
    sucursal_id: String,
    column_indexes: HashMap<String, usize>,
    #[serde(default)]
    delimiter: String,
    #[serde(default)]
    foreign_key_map: HashMap<String, HashMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalizarCsvImportacionInput {
    file_path: String,
    column_indexes: HashMap<String, usize>,
    #[serde(default)]
    delimiter: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CsvImportIssue {
    fila: usize,
    motivo: String,
    codigo: String,
    descripcion: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalizarCsvImportacionResult {
    total_filas: usize,
    unique_values: HashMap<String, Vec<String>>,
    preview_rows: Vec<HashMap<String, String>>,
    warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CsvArchivoSeleccionado {
    file_path: String,
    file_name: String,
    headers: Vec<String>,
    delimiter: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportarCsvProductosMapeadoResult {
    total_leidos: usize,
    productos_upsertados: usize,
    inventario_upsertado: usize,
    filas_omitidas: usize,
    errores: Vec<CsvImportIssue>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductoCatalogoPage {
    rows: Vec<Producto>,
    total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductoInventarioPage {
    rows: Vec<ProductoConStock>,
    total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HistorialVentasPage {
    rows: Vec<HistorialVenta>,
    total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FacturasEmitidasPage {
    rows: Vec<FacturaEmitida>,
    total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HistorialTraspasosPage {
    rows: Vec<HistorialTraspaso>,
    total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HistorialMermasPage {
    rows: Vec<HistorialMerma>,
    total: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductosBajoStockPage {
    rows: Vec<ProductoBajoStock>,
    total: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProductoConStock {
    id: String,
    codigo_barras: String,
    codigo_proveedor: String,
    proveedor_id: String,
    clave_producto: String,
    descripcion: String,
    marca: String,
    categoria: String,
    unidad: String,
    precio_costo: f64,
    costo_promedio: f64,
    precio_venta: f64,
    sat_clave_prod_serv: String,
    sat_clave_unidad: String,
    sucursal_id: String,
    stock: f64,
    stock_minimo: f64,
    precio_original: Option<f64>,
    precio_descontado: Option<f64>,
    nombre_promo: Option<String>,
    promocion_id: Option<String>,
    #[serde(default)]
    promo_tipo_descuento: Option<String>,
    #[serde(default)]
    promo_valor: Option<f64>,
    #[serde(default)]
    precio_1: f64,
    #[serde(default)]
    precio_2: f64,
    #[serde(default)]
    precio_3: f64,
    #[serde(default)]
    precio_4: f64,
    #[serde(default)]
    mayoreo_apartir: f64,
    #[serde(default)]
    a_granel: bool,
    #[serde(default)]
    no_en_catalogo: bool,
    #[serde(default)]
    ventas_negativas: bool,
    #[serde(default)]
    caducidad: Option<String>,
    #[serde(default)]
    fotos: String,
    #[serde(default)]
    descripcion_catalogo: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProductoPromocionPrecio {
    id: String,
    codigo_barras: String,
    codigo_proveedor: String,
    clave_producto: String,
    descripcion: String,
    marca: String,
    categoria: String,
    unidad: String,
    precio_costo: f64,
    precio_venta: f64,
    precio_costo_min: f64,
    precio_costo_max: f64,
    precio_venta_min: f64,
    precio_venta_max: f64,
    sucursales_con_precio: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Promocion {
    id: String,
    nombre: String,
    tipo_descuento: String,
    valor: f64,
    fecha_inicio: String,
    fecha_fin: String,
    activo: bool,
    producto_id: Option<String>,
    categoria_id: Option<String>,
    marca: Option<String>,
    eliminado: bool,
    updated_at: String,
    sucursal_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromocionInput {
    id: String,
    nombre: String,
    tipo_descuento: String,
    valor: f64,
    fecha_inicio: String,
    fecha_fin: String,
    activo: bool,
    producto_id: Option<String>,
    categoria_id: Option<String>,
    marca: Option<String>,
    sucursal_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Compra {
    id: String,
    proveedor_id: String,
    sucursal_id: String,
    fecha: String,
    total: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DetalleCompra {
    id: String,
    compra_id: String,
    producto_id: String,
    cantidad: f64,
    precio_costo_pactado: f64,
    #[serde(default)]
    costo_promedio_resultante: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompraDetalleInput {
    id: String,
    producto_id: String,
    cantidad: f64,
    precio_costo_pactado: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrarCompraInput {
    id: String,
    proveedor_id: String,
    sucursal_id: String,
    fecha: String,
    detalles: Vec<CompraDetalleInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VentaDetalleInput {
    id: String,
    producto_id: String,
    cantidad: f64,
    precio_venta_pactado: f64,
    #[serde(default)]
    tipo_precio_vendido: Option<String>,
    #[serde(default)]
    precio_original: Option<f64>,
    #[serde(default)]
    descuento_aplicado: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrarVentaInput {
    id: String,
    usuario_id: String,
    sucursal_id: String,
    fecha: String,
    metodo_pago: String,
    cliente_id: Option<String>,
    #[serde(default)]
    cliente_rapido_nombre: Option<String>,
    #[serde(default)]
    cliente_rapido_telefono: Option<String>,
    #[serde(default)]
    cliente_rapido_domicilio: Option<String>,
    #[serde(default)]
    requiere_factura: bool,
    efectivo_recibido: Option<f64>,
    cambio_entregado: Option<f64>,
    detalles: Vec<VentaDetalleInput>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Cliente {
    id: String,
    nombre: String,
    telefono: String,
    direccion: String,
    limite_credito: f64,
    saldo_deudor: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistorialVenta {
    id: String,
    fecha: String,
    total: f64,
    metodo_pago: String,
    efectivo_recibido: Option<f64>,
    cambio_entregado: Option<f64>,
    estado: String,
    sucursal_id: String,
    sucursal_nombre: String,
    usuario_id: String,
    usuario_nombre: String,
    cliente_id: Option<String>,
    cliente_nombre: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistorialVentaDetalle {
    id: String,
    venta_id: String,
    producto_id: String,
    descripcion: String,
    marca: String,
    cantidad: f64,
    precio_venta_pactado: f64,
    #[serde(default)]
    costo_unitario_pactado: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraspasoDetalleInput {
    id: String,
    producto_id: String,
    cantidad: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrarTraspasoInput {
    id: String,
    sucursal_origen_id: String,
    sucursal_destino_id: String,
    usuario_id: String,
    fecha: String,
    detalles: Vec<TraspasoDetalleInput>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecibirTraspasoInput {
    traspaso_id: String,
    usuario_recibio_id: String,
    fecha_recepcion: String,
    observaciones_recepcion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistorialTraspaso {
    id: String,
    sucursal_origen_id: String,
    sucursal_origen_nombre: String,
    sucursal_destino_id: String,
    sucursal_destino_nombre: String,
    usuario_id: String,
    usuario_nombre: String,
    fecha: String,
    estado: String,
    usuario_recibio_id: Option<String>,
    usuario_recibio_nombre: Option<String>,
    fecha_recepcion: Option<String>,
    observaciones_recepcion: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrarMermaAjusteInput {
    id: String,
    producto_id: String,
    sucursal_id: String,
    usuario_id: String,
    cantidad: f64,
    tipo_movimiento: String,
    motivo: String,
    fecha: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HistorialMerma {
    id: String,
    producto_id: String,
    producto_descripcion: String,
    marca: String,
    sucursal_id: String,
    sucursal_nombre: String,
    usuario_id: String,
    usuario_nombre: String,
    cantidad: f64,
    tipo_movimiento: String,
    motivo: String,
    fecha: String,
    costo_unitario: f64,
    costo_total_perdido: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorialVentasFiltro {
    fecha_inicio: Option<String>,
    fecha_fin: Option<String>,
    sucursal_id: Option<String>,
    usuario_id: Option<String>,
    folio: Option<String>,
    estado: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbonoCreditoInput {
    id: String,
    cliente_id: String,
    monto: f64,
    fecha: String,
    usuario_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CajaSesion {
    id: String,
    usuario_id: String,
    sucursal_id: String,
    fecha_apertura: String,
    monto_inicial: f64,
    fecha_cierre: Option<String>,
    monto_final_real: Option<f64>,
    monto_esperado: f64,
    estado: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CajaEstado {
    sesion: CajaSesion,
    ventas_efectivo: f64,
    ingresos: f64,
    egresos: f64,
    monto_esperado_actual: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbrirCajaInput {
    id: String,
    usuario_id: String,
    sucursal_id: String,
    fecha_apertura: String,
    monto_inicial: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovimientoCajaInput {
    id: String,
    sesion_id: String,
    tipo: String,
    monto: f64,
    motivo: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CerrarCajaInput {
    sesion_id: String,
    fecha_cierre: String,
    monto_final_real: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketProductoInput {
    descripcion: String,
    marca: Option<String>,
    cantidad: f64,
    precio_unitario: f64,
    importe: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketPayloadInput {
    folio: String,
    fecha: String,
    cajero: String,
    sucursal: String,
    logo_bytes: Option<Vec<u8>>,
    empresa_nombre: Option<String>,
    rfc: Option<String>,
    regimen_fiscal: Option<String>,
    codigo_postal: Option<String>,
    metodo_pago: String,
    estado: Option<String>,
    productos: Vec<TicketProductoInput>,
    subtotal: f64,
    descuento: f64,
    total: f64,
    recibido: Option<f64>,
    cambio: Option<f64>,
    mensaje: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PerifericosConfig {
    impresora_tickets: String,
    impresora_etiquetas: String,
    updated_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerifericosConfigInput {
    impresora_tickets: String,
    impresora_etiquetas: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SilentPrintInput {
    printer_name: String,
    contenido: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardFiltroInput {
    sucursal_id: Option<String>,
    fecha_inicio: Option<String>,
    fecha_fin: Option<String>,
    marca: Option<String>,
    categoria: Option<String>,
    proveedor_id: Option<String>,
    metodo_pago: Option<String>,
    usuario_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    total_vendido: f64,
    utilidad_neta: f64,
    transacciones: i64,
    ticket_promedio: f64,
    margen_porcentaje: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProductoBajoStock {
    producto_id: String,
    descripcion: String,
    marca: String,
    sucursal_id: String,
    sucursal_nombre: String,
    stock: f64,
    stock_minimo: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProductoMasVendido {
    producto_id: String,
    descripcion: String,
    marca: String,
    unidades_vendidas: f64,
}

fn query_productos_bajo_stock(
    conn: &Connection,
    user: &Usuario,
    sucursal_id: Option<String>,
    page: i64,
    page_size: i64,
) -> AppResult<ProductosBajoStockPage> {
    let sid = scoped_sucursal_for_read(user, sucursal_id);
    let (page, page_size) = normalize_page_args(page, page_size);
    let offset = page * page_size;
    let mut where_sql = String::from(
        "
        WHERE i.stock_minimo > 0
          AND i.stock <= i.stock_minimo
          AND p.eliminado = 0
          AND i.eliminado = 0
          AND s.eliminado = 0
        ",
    );
    let mut params_vec: Vec<String> = Vec::new();
    if let Some(value) = sid {
        where_sql.push_str(" AND i.sucursal_id = ?");
        params_vec.push(value);
    }

    let count_sql = format!(
        "
        SELECT COUNT(*)
        FROM inventario_sucursal i
        INNER JOIN productos p ON p.id = i.producto_id
        INNER JOIN sucursales s ON s.id = i.sucursal_id
        {where_sql}
        "
    );
    let total: i64 = conn
        .query_row(&count_sql, params_from_iter(params_vec.iter()), |row| row.get(0))
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut select_params = params_vec.clone();
    select_params.push(page_size.to_string());
    select_params.push(offset.to_string());
    let select_sql = format!(
        "
        SELECT p.id, p.descripcion, p.marca, s.id, s.nombre, i.stock, i.stock_minimo
        FROM inventario_sucursal i
        INNER JOIN productos p ON p.id = i.producto_id
        INNER JOIN sucursales s ON s.id = i.sucursal_id
        {where_sql}
        ORDER BY (i.stock - i.stock_minimo) ASC, p.descripcion
        LIMIT ? OFFSET ?
        "
    );
    let mut stmt = conn
        .prepare(&select_sql)
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let iter = stmt
        .query_map(params_from_iter(select_params.iter()), |row| {
            Ok(ProductoBajoStock {
                producto_id: row.get(0)?,
                descripcion: row.get(1)?,
                marca: row.get(2)?,
                sucursal_id: row.get(3)?,
                sucursal_nombre: row.get(4)?,
                stock: row.get(5)?,
                stock_minimo: row.get(6)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut rows = Vec::new();
    for item in iter {
        rows.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }

    Ok(ProductosBajoStockPage { rows, total })
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProductoRentabilidad {
    producto_id: String,
    descripcion: String,
    marca: String,
    unidades: f64,
    venta_total: f64,
    costo_total: f64,
    utilidad: f64,
    margen_porcentaje: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RentabilidadResumen {
    venta_total: f64,
    costo_total: f64,
    utilidad_bruta: f64,
    margen_porcentaje: f64,
    productos: Vec<ProductoRentabilidad>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetodoPagoResumen {
    metodo_pago: String,
    total: f64,
    transacciones: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IndicadorVentasResumen {
    total_vendido: f64,
    transacciones: i64,
    ticket_promedio: f64,
    ventas_canceladas: i64,
    ventas_credito: f64,
    ventas_contado: f64,
    metodos: Vec<MetodoPagoResumen>,
    productos_mas_vendidos: Vec<ProductoMasVendido>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IndicadorInventarioResumen {
    productos_en_inventario: i64,
    valor_inventario: f64,
    stock_total: f64,
    stock_bajo: i64,
    sin_stock: i64,
    sobre_stock: i64,
    bajo_stock: Vec<ProductoBajoStock>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IndicadorFinancieroResumen {
    ingresos_caja: f64,
    egresos_caja: f64,
    ventas_efectivo: f64,
    ventas_tarjeta: f64,
    ventas_transferencia: f64,
    ventas_credito: f64,
    compras: f64,
    cuentas_por_cobrar: f64,
    flujo_neto_estimado: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClienteDatosFiscales {
    cliente_id: String,
    rfc: String,
    razon_social: String,
    regimen_fiscal: String,
    codigo_postal: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FacturaEmitida {
    id: String,
    venta_id: String,
    uuid: Option<String>,
    rfc_receptor: String,
    monto_total: f64,
    estado: String,
    fecha_emision: String,
    pdf_path: Option<String>,
    xml_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfdiEmisor {
    rfc: String,
    nombre: String,
    regimen_fiscal: String,
    lugar_expedicion: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfdiReceptor {
    cliente_id: String,
    rfc: String,
    nombre: String,
    regimen_fiscal: String,
    domicilio_fiscal_receptor: String,
    uso_cfdi: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfdiImpuestoTraslado {
    base: f64,
    impuesto: String,
    tipo_factor: String,
    tasa_o_cuota: String,
    importe: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfdiConcepto {
    producto_id: String,
    clave_prod_serv: String,
    no_identificacion: String,
    cantidad: f64,
    clave_unidad: String,
    unidad: String,
    descripcion: String,
    valor_unitario: f64,
    importe: f64,
    objeto_imp: String,
    impuestos: Vec<CfdiImpuestoTraslado>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FacturaPayload {
    version: String,
    serie: String,
    folio: String,
    fecha: String,
    moneda: String,
    tipo_de_comprobante: String,
    exportacion: String,
    metodo_pago: String,
    forma_pago: String,
    subtotal: f64,
    total_impuestos_trasladados: f64,
    total: f64,
    emisor: CfdiEmisor,
    receptor: CfdiReceptor,
    conceptos: Vec<CfdiConcepto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActualizarEstadoFacturaInput {
    factura_id: String,
    uuid: String,
    pdf_path: Option<String>,
    xml_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EmpresaConfigFiscal {
    rfc: String,
    razon_social: String,
    regimen_fiscal: String,
    registro_patronal: Option<String>,
    actualizado_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SupabaseConfig {
    url: String,
    anon_key: String,
    is_connected: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupLocal {
    version: String,
    generado_at: String,
    tablas: HashMap<String, Vec<JsonValue>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncMigrationStatus {
    tablas: Vec<String>,
    tablas_con_uuid: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncUploadResult {
    total_registros: usize,
    por_tabla: HashMap<String, usize>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncTableStatus {
    tabla: String,
    pendientes: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pendientes: i64,
    ventas_pendientes: i64,
    tablas_pendientes: Vec<SyncTableStatus>,
    ultimo_intento_at: Option<String>,
    ultimo_exito_at: Option<String>,
    ultimo_error_at: Option<String>,
    ultimo_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Notificacion {
    id: String,
    categoria: String,
    severidad: String,
    titulo: String,
    mensaje: String,
    entidad_tipo: Option<String>,
    entidad_id: Option<String>,
    event_key: String,
    leida: bool,
    creada_at: String,
}

pub struct SesionActual(Mutex<Option<Usuario>>);
pub struct DbState(DbPool);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerfilUpdate {
    nombres: String,
    apellido_paterno: String,
    apellido_materno: String,
    email: String,
    password_actual: String,
    nueva_password: Option<String>,
}

fn get_conn(state_db: &tauri::State<DbState>) -> Result<r2d2::PooledConnection<SqliteConnectionManager>, AppError> {
    Ok(state_db.0.get()?)
}

fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

fn build_full_name(nombres: &str, apellido_paterno: &str, apellido_materno: &str) -> String {
    [nombres.trim(), apellido_paterno.trim(), apellido_materno.trim()]
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_valid_role(role: &str) -> bool {
    matches!(role, "SUPERADMIN" | "ADMIN" | "USUARIO")
}

fn is_bcrypt_hash(value: &str) -> bool {
    value.starts_with("$2a$") || value.starts_with("$2b$") || value.starts_with("$2y$")
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, AppError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns_iter = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for col in columns_iter {
        if col? == column {
            return Ok(true);
        }
    }

    Ok(false)
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns_iter = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut columns = Vec::new();
    for col in columns_iter {
        columns.push(col?);
    }
    Ok(columns)
}

fn generate_uuid_like() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_nanos();
    format!(
        "{:08x}-{:04x}-4{:03x}-a{:03x}-{:012x}",
        (nanos & 0xffff_ffff) as u64,
        ((nanos >> 32) & 0xffff) as u64,
        ((nanos >> 48) & 0x0fff) as u64,
        ((nanos >> 60) & 0x0fff) as u64,
        ((nanos >> 72) & 0xffff_ffff_ffff) as u64
    )
}

fn inventario_costo_promedio(
    tx: &Transaction<'_>,
    producto_id: &str,
    sucursal_id: &str,
) -> Result<(f64, f64), AppError> {
    let costo_producto: f64 = tx
        .query_row(
            "SELECT COALESCE(NULLIF(costo_promedio, 0), precio_costo, 0) FROM productos WHERE id = ?1",
            [producto_id],
            |row| row.get(0),
        )
        .optional()?
        .unwrap_or(0.0);

    let inventario: Option<(f64, f64)> = tx
        .query_row(
            "
            SELECT stock, COALESCE(NULLIF(costo_promedio, 0), ?3)
            FROM inventario_sucursal
            WHERE producto_id = ?1 AND sucursal_id = ?2
            ",
            params![producto_id, sucursal_id, costo_producto],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;

    Ok(inventario.unwrap_or((0.0, costo_producto)))
}

fn insertar_movimiento_inventario(
    tx: &Transaction<'_>,
    producto_id: &str,
    sucursal_id: &str,
    tipo: &str,
    referencia_tipo: &str,
    referencia_id: &str,
    cantidad: f64,
    costo_unitario: Option<f64>,
    usuario_id: Option<&str>,
    fecha: &str,
) -> Result<(), AppError> {
    tx.execute(
        "
        INSERT INTO movimientos_inventario (
            uuid, producto_id, sucursal_id, tipo, referencia_tipo, referencia_id,
            cantidad, costo_unitario, usuario_id, fecha, sincronizado, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, datetime('now'))
        ",
        params![
            generate_uuid_like(),
            producto_id,
            sucursal_id,
            tipo,
            referencia_tipo,
            referencia_id,
            cantidad,
            costo_unitario,
            usuario_id,
            fecha
        ],
    )?;
    Ok(())
}

fn resolve_valid_producto_proveedor_id(conn: &Connection, proveedor_id: &str) -> Result<String, AppError> {
    let candidate = proveedor_id.trim();
    if candidate.is_empty() || candidate.eq_ignore_ascii_case("null") {
        return Err(AppError::Validation(
            "Error: Debes seleccionar un proveedor válido de la lista para poder registrar el producto.".to_string(),
        ));
    }

    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM proveedores WHERE id = ?1 AND eliminado = 0",
        [candidate],
        |row| row.get(0),
    )?;

    if exists > 0 {
        Ok(candidate.to_string())
    } else {
        Err(AppError::Validation(
            "Error: Debes seleccionar un proveedor válido de la lista para poder registrar el producto.".to_string(),
        ))
    }
}

fn ensure_sync_uuids(conn: &Connection) -> Result<(), AppError> {
    for table in UUID_SYNC_TABLES {
        if !table_has_column(conn, table, "sync_uuid")? {
            continue;
        }

        let mut stmt = conn.prepare(&format!("SELECT id FROM {table} WHERE sync_uuid IS NULL OR sync_uuid = ''"))?;
        let ids_iter = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut ids = Vec::new();
        for id in ids_iter {
            ids.push(id?);
        }
        drop(stmt);

        for id in ids {
            conn.execute(
                &format!("UPDATE {table} SET sync_uuid = ?1 WHERE id = ?2 AND (sync_uuid IS NULL OR sync_uuid = '')"),
                params![generate_uuid_like(), id],
            )?;
        }
    }

    Ok(())
}

fn verify_password_and_migrate(
    conn: &Connection,
    user_id: &str,
    clave: &str,
    stored_password: &str,
) -> Result<bool, AppError> {
    if is_bcrypt_hash(stored_password) {
        return verify(clave, stored_password).map_err(AppError::from);
    }

    if stored_password == clave {
        let migrated_hash = hash(clave, DEFAULT_COST)?;
        conn.execute(
            "UPDATE usuarios SET password_hash = ?1 WHERE id = ?2",
            params![migrated_hash, user_id],
        )?;
        return Ok(true);
    }

    Ok(false)
}

fn validate_usuario(usuario: &Usuario, require_admin_role: bool) -> Result<(), AppError> {
    if usuario.id.trim().is_empty() {
        return Err(AppError::Validation("El usuario necesita un identificador interno.".to_string()));
    }

    if usuario.nombre.trim().is_empty() {
        return Err(AppError::Validation("El usuario necesita nombre.".to_string()));
    }

    if usuario.email.trim().is_empty() {
        return Err(AppError::Validation("El usuario necesita correo electrónico.".to_string()));
    }

    if !is_valid_role(&usuario.role) {
        return Err(AppError::Validation("El rol del usuario no es válido.".to_string()));
    }

    if require_admin_role && usuario.role != "SUPERADMIN" && usuario.role != "ADMIN" {
        return Err(AppError::Validation(
            "El primer usuario debe ser Administrador o Super Administrador.".to_string(),
        ));
    }

    if usuario.sucursal_id.trim().is_empty() {
        return Err(AppError::Validation("El usuario debe estar asociado a una sucursal.".to_string()));
    }

    Ok(())
}

fn validate_perfil_update(perfil: &PerfilUpdate) -> Result<(), AppError> {
    if perfil.nombres.trim().is_empty() {
        return Err(AppError::Validation("Ingresa tus nombres.".to_string()));
    }

    if perfil.apellido_paterno.trim().is_empty() {
        return Err(AppError::Validation("Ingresa tu apellido paterno.".to_string()));
    }

    if perfil.email.trim().is_empty() {
        return Err(AppError::Validation("Ingresa tu correo electrónico.".to_string()));
    }

    if perfil.password_actual.trim().is_empty() {
        return Err(AppError::Validation("Ingresa tu contraseña actual para guardar cambios.".to_string()));
    }

    if let Some(nueva_password) = &perfil.nueva_password {
        if !nueva_password.trim().is_empty() && nueva_password.trim().len() < 4 {
            return Err(AppError::Validation("La nueva contraseña debe tener al menos 4 caracteres.".to_string()));
        }
    }

    Ok(())
}

fn validate_sucursal(sucursal: &Sucursal) -> Result<(), AppError> {
    if sucursal.id.trim().is_empty() {
        return Err(AppError::Validation("La sucursal necesita un identificador interno.".to_string()));
    }

    if sucursal.nombre.trim().is_empty() {
        return Err(AppError::Validation("La sucursal necesita nombre.".to_string()));
    }

    if sucursal.direccion.trim().is_empty() {
        return Err(AppError::Validation("La sucursal necesita dirección.".to_string()));
    }

    if !sucursal.codigo_postal.trim().is_empty() && sucursal.codigo_postal.trim().len() != 5 {
        return Err(AppError::Validation("El código postal de la sucursal debe tener 5 dígitos.".to_string()));
    }

    Ok(())
}

fn validate_proveedor(proveedor: &Proveedor) -> Result<(), AppError> {
    if proveedor.id.trim().is_empty() {
        return Err(AppError::Validation("El proveedor necesita identificador interno.".to_string()));
    }

    if proveedor.nombre.trim().is_empty() {
        return Err(AppError::Validation("El proveedor necesita nombre.".to_string()));
    }

    Ok(())
}

fn validate_marca(marca: &Marca) -> Result<(), AppError> {
    if marca.id.trim().is_empty() {
        return Err(AppError::Validation("La marca necesita identificador interno.".to_string()));
    }
    if marca.nombre.trim().is_empty() {
        return Err(AppError::Validation("La marca necesita nombre.".to_string()));
    }
    Ok(())
}

fn validate_categoria(categoria: &Categoria) -> Result<(), AppError> {
    if categoria.id.trim().is_empty() {
        return Err(AppError::Validation("La categoría necesita identificador interno.".to_string()));
    }
    if categoria.nombre.trim().is_empty() {
        return Err(AppError::Validation("La categoría necesita nombre.".to_string()));
    }
    Ok(())
}

fn validate_unidad(unidad: &UnidadMedida) -> Result<(), AppError> {
    if unidad.id.trim().is_empty() {
        return Err(AppError::Validation("La unidad necesita identificador interno.".to_string()));
    }
    if unidad.nombre.trim().is_empty() {
        return Err(AppError::Validation("La unidad necesita nombre.".to_string()));
    }
    if !unidad.clave_sat.trim().is_empty() {
        validate_exact_len(
            &normalize_upper_trim(&unidad.clave_sat),
            3,
            "La Clave de Unidad SAT debe tener exactamente 3 caracteres (Ej: H87).",
        )?;
    }
    Ok(())
}

fn normalize_upper_trim(value: &str) -> String {
    value.trim().to_uppercase()
}

fn normalize_title_trim(value: &str) -> String {
    normalize_spaces(value).to_uppercase()
}

fn normalize_spaces(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_email_trim(value: &str) -> String {
    value.trim().to_lowercase()
}

fn normalize_plain_trim(value: &str) -> String {
    normalize_spaces(value)
}

fn legacy_catalog_id(prefix: &str, raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_uppercase());
        } else if ch == ' ' || ch == '-' || ch == '_' || ch == '/' {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    let out = out.trim_matches('_');
    if out.is_empty() {
        format!("{prefix}-GEN")
    } else {
        format!("{prefix}-{out}")
    }
}

fn optional_normalized_name(value: Option<&String>) -> Option<String> {
    value
        .map(|v| normalize_title_trim(v))
        .filter(|v| !v.is_empty())
}

fn find_catalog_by_legacy_id(
    tx: &Transaction<'_>,
    table: &str,
    prefixes: &[&str],
    legacy_id: i64,
) -> Result<Option<(String, String)>, AppError> {
    for prefix in prefixes {
        let candidate_id = format!("{prefix}-{legacy_id}");
        let found = tx
            .query_row(
                &format!("SELECT id, TRIM(COALESCE(nombre, '')) FROM {table} WHERE id = ?1 AND eliminado = 0"),
                [&candidate_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((id, name)) = found {
            let normalized_name = normalize_title_trim(&name);
            if !normalized_name.is_empty() {
                return Ok(Some((id, normalized_name)));
            }
        }
    }
    Ok(None)
}

fn find_proveedor_id_by_legacy_id(
    tx: &Transaction<'_>,
    prefixes: &[&str],
    legacy_id: i64,
) -> Result<Option<String>, AppError> {
    for prefix in prefixes {
        let candidate_id = format!("{prefix}-{legacy_id}");
        let found = tx
            .query_row(
                "SELECT id FROM proveedores WHERE id = ?1 AND eliminado = 0",
                [&candidate_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if found.is_some() {
            return Ok(found);
        }
    }
    Ok(None)
}

fn legacy_truthy(value: Option<&String>) -> i64 {
    let normalized = value
        .map(|v| normalize_upper_trim(v))
        .unwrap_or_default()
        .replace('Í', "I");
    matches!(
        normalized.as_str(),
        "1" | "SI" | "S" | "TRUE" | "VERDADERO" | "YES" | "Y" | "X"
    ) as i64
}

fn normalize_import_key(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_lowercase())
        .collect()
}

fn json_value_text(row: &HashMap<String, JsonValue>, aliases: &[&str]) -> String {
    for alias in aliases {
        if let Some(value) = row.get(*alias) {
            let text = match value {
                JsonValue::String(text) => text.trim().to_string(),
                JsonValue::Number(number) => number.to_string(),
                JsonValue::Bool(flag) => flag.to_string(),
                _ => String::new(),
            };
            if !text.is_empty() {
                return text;
            }
        }
    }
    for alias in aliases {
        let normalized_alias = normalize_import_key(alias);
        for (key, value) in row {
            if normalize_import_key(key) == normalized_alias {
                let text = match value {
                    JsonValue::String(text) => text.trim().to_string(),
                    JsonValue::Number(number) => number.to_string(),
                    JsonValue::Bool(flag) => flag.to_string(),
                    _ => String::new(),
                };
                if !text.is_empty() {
                    return text;
                }
            }
        }
    }
    String::new()
}

fn json_value_number(row: &HashMap<String, JsonValue>, aliases: &[&str]) -> f64 {
    let text = json_value_text(row, aliases);
    if text.is_empty() {
        return 0.0;
    }
    let mut clean = text.replace('$', "").replace(' ', "");
    let last_comma = clean.rfind(',');
    let last_dot = clean.rfind('.');
    if last_comma > last_dot {
        clean = clean.replace('.', "").replace(',', ".");
    } else {
        clean = clean.replace(',', "");
    }
    clean.parse::<f64>().unwrap_or(0.0)
}

fn validate_exact_len(value: &str, expected: usize, message: &str) -> Result<(), AppError> {
    if value.chars().count() != expected {
        return Err(AppError::Validation(message.to_string()));
    }
    Ok(())
}

fn validate_digits_exact(value: &str, expected: usize, message: &str) -> Result<(), AppError> {
    validate_exact_len(value, expected, message)?;
    if !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(AppError::Validation(message.to_string()));
    }
    Ok(())
}

fn validate_rfc_sat(rfc: &str) -> Result<(), AppError> {
    let len = rfc.chars().count();
    if !(12..=13).contains(&len) {
        return Err(AppError::Validation("El RFC debe tener entre 12 y 13 caracteres.".to_string()));
    }
    Ok(())
}

fn validate_regimen_fiscal_sat(regimen_fiscal: &str) -> Result<(), AppError> {
    validate_digits_exact(
        regimen_fiscal,
        3,
        "El Régimen Fiscal debe tener exactamente 3 dígitos.",
    )
}

fn validate_codigo_postal_sat(codigo_postal: &str) -> Result<(), AppError> {
    validate_digits_exact(
        codigo_postal,
        5,
        "El Código Postal fiscal debe tener exactamente 5 dígitos.",
    )
}

fn validate_uuid_sat_like(uuid: &str) -> Result<(), AppError> {
    let uuid = uuid.trim();
    if uuid.len() < 10 || uuid.len() > 64 {
        return Err(AppError::Validation("El UUID de la factura no tiene una longitud válida.".to_string()));
    }
    if !uuid.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-') {
        return Err(AppError::Validation("El UUID de la factura contiene caracteres inválidos.".to_string()));
    }
    Ok(())
}

fn validate_producto_sat_claves(producto: &Producto) -> Result<(), AppError> {
    validate_digits_exact(
        producto.sat_clave_prod_serv.trim(),
        8,
        "La Clave de Producto/Servicio SAT debe tener exactamente 8 dígitos (Ej: 27111700).",
    )?;
    validate_exact_len(
        producto.sat_clave_unidad.trim(),
        3,
        "La Clave de Unidad SAT debe tener exactamente 3 caracteres (Ej: H87).",
    )?;
    Ok(())
}

fn sanitize_producto(mut producto: Producto) -> Producto {
    producto.codigo_barras = normalize_upper_trim(&producto.codigo_barras);
    producto.codigo_proveedor = normalize_upper_trim(&producto.codigo_proveedor);
    producto.proveedor_id = producto.proveedor_id.trim().to_string();
    producto.clave_producto = normalize_upper_trim(&producto.clave_producto);
    producto.descripcion = normalize_title_trim(&producto.descripcion);
    producto.marca = normalize_title_trim(&producto.marca);
    producto.categoria = normalize_title_trim(&producto.categoria);
    producto.unidad = normalize_title_trim(&producto.unidad);
    producto.sat_clave_prod_serv = normalize_upper_trim(&producto.sat_clave_prod_serv);
    producto.sat_clave_unidad = normalize_upper_trim(&producto.sat_clave_unidad);
    producto.precio_1 = round_money(producto.precio_1.max(0.0));
    producto.precio_2 = round_money(producto.precio_2.max(0.0));
    producto.precio_3 = round_money(producto.precio_3.max(0.0));
    producto.precio_4 = round_money(producto.precio_4.max(0.0));
    producto.mayoreo_apartir = producto.mayoreo_apartir.max(0.0);
    producto.fotos = normalize_plain_trim(&producto.fotos);
    producto.descripcion_catalogo = normalize_title_trim(&producto.descripcion_catalogo);
    producto.caducidad = producto
        .caducidad
        .as_deref()
        .map(normalize_plain_trim)
        .filter(|value| !value.is_empty());
    producto
}

fn validate_producto(producto: &Producto) -> Result<(), AppError> {
    if producto.id.trim().is_empty() {
        return Err(AppError::Validation("El producto necesita identificador interno.".to_string()));
    }

    if producto.descripcion.trim().is_empty() {
        return Err(AppError::Validation("El producto necesita descripción.".to_string()));
    }

    let proveedor_id = producto.proveedor_id.trim();
    if proveedor_id.is_empty() || proveedor_id.eq_ignore_ascii_case("null") {
        return Err(AppError::Validation(
            "Error: Debes seleccionar un proveedor válido de la lista para poder registrar el producto.".to_string(),
        ));
    }

    if producto.marca.trim().is_empty() {
        return Err(AppError::Validation("Selecciona una marca válida de la lista.".to_string()));
    }
    if producto.categoria.trim().is_empty() {
        return Err(AppError::Validation("Selecciona una categoría válida de la lista.".to_string()));
    }
    if producto.unidad.trim().is_empty() {
        return Err(AppError::Validation("Selecciona una unidad válida de la lista.".to_string()));
    }

    if !producto.precio_costo.is_finite()
        || !producto.costo_promedio.is_finite()
        || !producto.precio_venta.is_finite()
    {
        return Err(AppError::Validation("Los precios deben ser números válidos.".to_string()));
    }

    if producto.precio_costo < 0.0 || producto.precio_venta < 0.0 {
        return Err(AppError::Validation("Los precios no pueden ser negativos.".to_string()));
    }

    validate_producto_sat_claves(producto)?;

    Ok(())
}

fn validate_inventario_input(inventario: &InventarioSucursalInput) -> Result<(), AppError> {
    if inventario.sucursal_id.trim().is_empty() {
        return Err(AppError::Validation("La sucursal del inventario es obligatoria.".to_string()));
    }

    if inventario.stock < 0.0 || inventario.stock_minimo < 0.0 {
        return Err(AppError::Validation("Stock y stock mínimo no pueden ser negativos.".to_string()));
    }

    Ok(())
}

fn validate_registrar_compra_input(compra: &RegistrarCompraInput) -> Result<(), AppError> {
    if compra.id.trim().is_empty() {
        return Err(AppError::Validation("La compra necesita identificador interno.".to_string()));
    }

    if compra.proveedor_id.trim().is_empty() {
        return Err(AppError::Validation("Selecciona un proveedor.".to_string()));
    }

    if compra.sucursal_id.trim().is_empty() {
        return Err(AppError::Validation("Falta la sucursal de la compra.".to_string()));
    }

    if compra.fecha.trim().is_empty() {
        return Err(AppError::Validation("La compra necesita fecha.".to_string()));
    }

    if compra.detalles.is_empty() {
        return Err(AppError::Validation("Agrega al menos un producto al detalle de compra.".to_string()));
    }

    for detalle in &compra.detalles {
        if detalle.id.trim().is_empty() {
            return Err(AppError::Validation("Un detalle de compra no tiene identificador.".to_string()));
        }
        if detalle.producto_id.trim().is_empty() {
            return Err(AppError::Validation("Un detalle de compra no tiene producto.".to_string()));
        }
        if !detalle.cantidad.is_finite() || detalle.cantidad <= 0.0 {
            return Err(AppError::Validation("La cantidad debe ser mayor que cero.".to_string()));
        }
        if !detalle.precio_costo_pactado.is_finite() || detalle.precio_costo_pactado <= 0.0 {
            return Err(AppError::Validation("El precio costo pactado debe ser mayor que cero.".to_string()));
        }
    }

    Ok(())
}

fn validate_registrar_venta_input(venta: &RegistrarVentaInput) -> Result<(), AppError> {
    if venta.id.trim().is_empty() {
        return Err(AppError::Validation("La venta necesita identificador interno.".to_string()));
    }
    if venta.usuario_id.trim().is_empty() {
        return Err(AppError::Validation("La venta necesita usuario.".to_string()));
    }
    if venta.sucursal_id.trim().is_empty() {
        return Err(AppError::Validation("La venta necesita sucursal.".to_string()));
    }
    if venta.fecha.trim().is_empty() {
        return Err(AppError::Validation("La venta necesita fecha.".to_string()));
    }
    if venta.metodo_pago.trim().is_empty() {
        return Err(AppError::Validation("Selecciona un método de pago.".to_string()));
    }
    if !matches!(
        venta.metodo_pago.as_str(),
        "EFECTIVO" | "TARJETA" | "TRANSFERENCIA" | "CREDITO"
    ) {
        return Err(AppError::Validation("Método de pago inválido.".to_string()));
    }
    if venta.metodo_pago == "CREDITO"
        && venta
            .cliente_id
            .as_ref()
            .map(|id| id.trim().is_empty())
            .unwrap_or(true)
    {
        return Err(AppError::Validation("Selecciona un cliente para venta a crédito.".to_string()));
    }
    if venta.detalles.is_empty() {
        return Err(AppError::Validation("Agrega al menos un producto al carrito.".to_string()));
    }

    for detalle in &venta.detalles {
        if detalle.id.trim().is_empty() {
            return Err(AppError::Validation("Un detalle de venta no tiene identificador.".to_string()));
        }
        if detalle.producto_id.trim().is_empty() {
            return Err(AppError::Validation("Un detalle de venta no tiene producto.".to_string()));
        }
        if detalle.cantidad <= 0.0 {
            return Err(AppError::Validation("La cantidad de venta debe ser mayor a cero.".to_string()));
        }
        if detalle.precio_venta_pactado < 0.0 {
            return Err(AppError::Validation("El precio de venta pactado no puede ser negativo.".to_string()));
        }
    }

    Ok(())
}

fn consolidar_detalles_venta(detalles: &[VentaDetalleInput]) -> Result<Vec<VentaDetalleInput>, AppError> {
    let mut index_by_producto: HashMap<String, usize> = HashMap::new();
    let mut consolidados: Vec<VentaDetalleInput> = Vec::new();

    for detalle in detalles {
        let producto_id = detalle.producto_id.trim().to_string();
        if let Some(index) = index_by_producto.get(&producto_id).copied() {
            let existente = &mut consolidados[index];
            if (existente.precio_venta_pactado - detalle.precio_venta_pactado).abs() > f64::EPSILON {
                return Err(AppError::Validation(
                    "No se puede repetir un producto con precios de venta distintos.".to_string(),
                ));
            }
            existente.cantidad += detalle.cantidad;
        } else {
            index_by_producto.insert(producto_id.clone(), consolidados.len());
            consolidados.push(VentaDetalleInput {
                id: detalle.id.clone(),
                producto_id,
                cantidad: detalle.cantidad,
                precio_venta_pactado: detalle.precio_venta_pactado,
                tipo_precio_vendido: detalle.tipo_precio_vendido.clone(),
                precio_original: detalle.precio_original,
                descuento_aplicado: detalle.descuento_aplicado,
            });
        }
    }

    Ok(consolidados)
}

fn consolidar_detalles_compra(detalles: &[CompraDetalleInput]) -> Vec<CompraDetalleInput> {
    let mut index_by_producto: HashMap<String, usize> = HashMap::new();
    let mut consolidados: Vec<CompraDetalleInput> = Vec::new();

    for detalle in detalles {
        let producto_id = detalle.producto_id.trim().to_string();
        if let Some(index) = index_by_producto.get(&producto_id).copied() {
            let existente = &mut consolidados[index];
            let total_cantidad = existente.cantidad + detalle.cantidad;
            let total_costo = (existente.cantidad * existente.precio_costo_pactado)
                + (detalle.cantidad * detalle.precio_costo_pactado);
            existente.cantidad = total_cantidad;
            existente.precio_costo_pactado = if total_cantidad > 0.0 {
                round_money(total_costo / total_cantidad)
            } else {
                detalle.precio_costo_pactado
            };
        } else {
            index_by_producto.insert(producto_id.clone(), consolidados.len());
            consolidados.push(CompraDetalleInput {
                id: detalle.id.trim().to_string(),
                producto_id,
                cantidad: detalle.cantidad,
                precio_costo_pactado: detalle.precio_costo_pactado,
            });
        }
    }

    consolidados
}

fn consolidar_detalles_traspaso(detalles: &[TraspasoDetalleInput]) -> Vec<TraspasoDetalleInput> {
    let mut index_by_producto: HashMap<String, usize> = HashMap::new();
    let mut consolidados: Vec<TraspasoDetalleInput> = Vec::new();

    for detalle in detalles {
        let producto_id = detalle.producto_id.trim().to_string();
        if let Some(index) = index_by_producto.get(&producto_id).copied() {
            consolidados[index].cantidad += detalle.cantidad;
        } else {
            index_by_producto.insert(producto_id.clone(), consolidados.len());
            consolidados.push(TraspasoDetalleInput {
                id: detalle.id.clone(),
                producto_id,
                cantidad: detalle.cantidad,
            });
        }
    }

    consolidados
}

fn validate_cliente(cliente: &Cliente) -> Result<(), AppError> {
    if cliente.id.trim().is_empty() {
        return Err(AppError::Validation("El cliente necesita identificador interno.".to_string()));
    }
    if cliente.nombre.trim().is_empty() {
        return Err(AppError::Validation("El cliente necesita nombre.".to_string()));
    }
    if !cliente.limite_credito.is_finite() || !cliente.saldo_deudor.is_finite() {
        return Err(AppError::Validation("Límite de crédito y saldo deben ser números válidos.".to_string()));
    }
    if cliente.limite_credito < 0.0 || cliente.saldo_deudor < 0.0 {
        return Err(AppError::Validation("Límite de crédito y saldo no pueden ser negativos.".to_string()));
    }
    Ok(())
}

fn validate_cliente_datos_fiscales(input: &ClienteDatosFiscales) -> Result<(), AppError> {
    if input.cliente_id.trim().is_empty()
        || input.rfc.trim().is_empty()
        || input.razon_social.trim().is_empty()
        || input.regimen_fiscal.trim().is_empty()
        || input.codigo_postal.trim().is_empty()
    {
        return Err(AppError::Validation("Completa todos los datos fiscales del cliente.".to_string()));
    }

    validate_rfc_sat(&normalize_upper_trim(&input.rfc))?;
    validate_regimen_fiscal_sat(&input.regimen_fiscal.trim())?;
    validate_codigo_postal_sat(&input.codigo_postal.trim())?;

    Ok(())
}

fn validate_abono_credito(input: &AbonoCreditoInput) -> Result<(), AppError> {
    if input.id.trim().is_empty()
        || input.cliente_id.trim().is_empty()
        || input.fecha.trim().is_empty()
        || input.usuario_id.trim().is_empty()
    {
        return Err(AppError::Validation("Datos incompletos para registrar abono.".to_string()));
    }
    if !input.monto.is_finite() || input.monto <= 0.0 {
        return Err(AppError::Validation("El abono debe ser mayor que cero.".to_string()));
    }
    Ok(())
}

fn validate_registrar_traspaso_input(input: &RegistrarTraspasoInput) -> Result<(), AppError> {
    if input.id.trim().is_empty()
        || input.sucursal_origen_id.trim().is_empty()
        || input.sucursal_destino_id.trim().is_empty()
        || input.usuario_id.trim().is_empty()
        || input.fecha.trim().is_empty()
    {
        return Err(AppError::Validation("Datos incompletos para registrar traspaso.".to_string()));
    }
    if input.sucursal_origen_id == input.sucursal_destino_id {
        return Err(AppError::Validation("La sucursal origen y destino no pueden ser la misma.".to_string()));
    }
    if input.detalles.is_empty() {
        return Err(AppError::Validation("Agrega al menos un producto al traspaso.".to_string()));
    }
    for detalle in &input.detalles {
        if detalle.id.trim().is_empty() || detalle.producto_id.trim().is_empty() {
            return Err(AppError::Validation("Un detalle de traspaso está incompleto.".to_string()));
        }
        if !detalle.cantidad.is_finite() || detalle.cantidad <= 0.0 {
            return Err(AppError::Validation("La cantidad de traspaso debe ser mayor que cero.".to_string()));
        }
    }
    Ok(())
}

fn validate_registrar_merma_ajuste_input(input: &RegistrarMermaAjusteInput) -> Result<(), AppError> {
    if input.id.trim().is_empty()
        || input.producto_id.trim().is_empty()
        || input.sucursal_id.trim().is_empty()
        || input.usuario_id.trim().is_empty()
        || input.motivo.trim().is_empty()
        || input.fecha.trim().is_empty()
    {
        return Err(AppError::Validation("Datos incompletos para registrar merma/ajuste.".to_string()));
    }
    if !matches!(
        input.tipo_movimiento.as_str(),
        "MERMA" | "AJUSTE" | "AJUSTE_ENTRADA" | "AJUSTE_SALIDA"
    ) {
        return Err(AppError::Validation(
            "Tipo de movimiento inválido. Usa MERMA, AJUSTE_ENTRADA o AJUSTE_SALIDA.".to_string(),
        ));
    }
    if !input.cantidad.is_finite() || input.cantidad <= 0.0 {
        return Err(AppError::Validation("La cantidad debe ser mayor que cero.".to_string()));
    }
    Ok(())
}

fn validate_abrir_caja_input(input: &AbrirCajaInput) -> Result<(), AppError> {
    if input.id.trim().is_empty()
        || input.usuario_id.trim().is_empty()
        || input.sucursal_id.trim().is_empty()
        || input.fecha_apertura.trim().is_empty()
    {
        return Err(AppError::Validation("Datos incompletos para abrir caja.".to_string()));
    }
    if !input.monto_inicial.is_finite() || input.monto_inicial < 0.0 {
        return Err(AppError::Validation("El fondo inicial no puede ser negativo.".to_string()));
    }
    Ok(())
}

fn normalize_money(value: f64, field_name: &str, allow_zero: bool) -> Result<f64, AppError> {
    if !value.is_finite() {
        return Err(AppError::Validation(format!("{field_name} debe ser un número válido.")));
    }
    if value < 0.0 || (!allow_zero && value <= 0.0) {
        let message = if allow_zero {
            format!("{field_name} no puede ser negativo.")
        } else {
            format!("{field_name} debe ser mayor que cero.")
        };
        return Err(AppError::Validation(message));
    }
    Ok((value * 100.0).round() / 100.0)
}

fn validate_movimiento_caja_input(input: &MovimientoCajaInput) -> Result<(), AppError> {
    if input.id.trim().is_empty() || input.sesion_id.trim().is_empty() {
        return Err(AppError::Validation("Datos incompletos para el movimiento de caja.".to_string()));
    }
    if input.tipo != "INGRESO" && input.tipo != "EGRESO" {
        return Err(AppError::Validation("El tipo de movimiento debe ser INGRESO o EGRESO.".to_string()));
    }
    if !input.monto.is_finite() || input.monto <= 0.0 {
        return Err(AppError::Validation("El monto del movimiento debe ser mayor que cero.".to_string()));
    }
    if input.motivo.trim().is_empty() {
        return Err(AppError::Validation("El motivo del movimiento es obligatorio.".to_string()));
    }
    Ok(())
}

fn validate_cerrar_caja_input(input: &CerrarCajaInput) -> Result<(), AppError> {
    if input.sesion_id.trim().is_empty() || input.fecha_cierre.trim().is_empty() {
        return Err(AppError::Validation("Datos incompletos para cerrar caja.".to_string()));
    }
    if !input.monto_final_real.is_finite() || input.monto_final_real < 0.0 {
        return Err(AppError::Validation("El monto final real no puede ser negativo.".to_string()));
    }
    Ok(())
}

fn normalize_filter(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn current_session_user(state_sesion: &tauri::State<SesionActual>) -> AppResult<Usuario> {
    state_sesion
        .0
        .lock()
        .map_err(|_| "No se pudo leer la sesión actual.".to_string())?
        .clone()
        .ok_or_else(|| "No hay una sesión activa.".to_string())
}

fn is_superadmin(user: &Usuario) -> bool {
    user.role.eq_ignore_ascii_case("SUPERADMIN")
}

fn is_admin_or_superadmin_role(role: &str) -> bool {
    matches!(role.to_ascii_uppercase().as_str(), "ADMIN" | "SUPERADMIN")
}

fn ensure_admin_or_superadmin(user: &Usuario) -> AppResult<()> {
    if is_admin_or_superadmin_role(&user.role) {
        Ok(())
    } else {
        Err("No tienes permisos para realizar esta operación.".to_string())
    }
}

fn ensure_unique_catalog_name(
    conn: &Connection,
    table: &str,
    label: &str,
    current_id: Option<&str>,
    name: &str,
) -> Result<(), AppError> {
    let table = match table {
        "proveedores" => "proveedores",
        "marcas" => "marcas",
        "categorias" => "categorias",
        "unidades" => "unidades",
        _ => return Err(AppError::Validation("Catálogo inválido.".to_string())),
    };
    let normalized = name.trim();
    if normalized.is_empty() {
        return Err(AppError::Validation(format!("El {label} necesita nombre.")));
    }

    let count: i64 = if let Some(id) = current_id {
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM {table} WHERE eliminado = 0 AND nombre = ?1 COLLATE NOCASE AND id <> ?2"
            ),
            params![normalized, id],
            |row| row.get(0),
        )?
    } else {
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM {table} WHERE eliminado = 0 AND nombre = ?1 COLLATE NOCASE"
            ),
            [normalized],
            |row| row.get(0),
        )?
    };

    if count > 0 {
        return Err(AppError::Validation(format!(
            "Ya existe un {label} activo con ese nombre."
        )));
    }
    Ok(())
}

fn ensure_catalog_value_exists(
    conn: &Connection,
    table: &str,
    label: &str,
    name: &str,
) -> Result<String, AppError> {
    let table = match table {
        "marcas" => "marcas",
        "categorias" => "categorias",
        "unidades" => "unidades",
        _ => return Err(AppError::Validation("Catálogo inválido.".to_string())),
    };
    let normalized = name.trim();
    if normalized.is_empty() {
        return Err(AppError::Validation(format!(
            "Selecciona una {label} válida de la lista."
        )));
    }

    let stored: Option<String> = conn
        .query_row(
            &format!("SELECT nombre FROM {table} WHERE eliminado = 0 AND nombre = ?1 COLLATE NOCASE LIMIT 1"),
            [normalized],
            |row| row.get(0),
        )
        .optional()?;

    stored.ok_or_else(|| {
        AppError::Validation(format!(
            "Selecciona una {label} válida de la lista."
        ))
    })
}

fn ensure_superadmin(user: &Usuario) -> AppResult<()> {
    if is_superadmin(user) {
        Ok(())
    } else {
        Err("Esta operación requiere permisos de SUPERADMIN.".to_string())
    }
}

fn require_admin_or_superadmin(state_sesion: &tauri::State<SesionActual>) -> AppResult<Usuario> {
    let user = current_session_user(state_sesion)?;
    ensure_admin_or_superadmin(&user)?;
    Ok(user)
}

fn require_superadmin(state_sesion: &tauri::State<SesionActual>) -> AppResult<Usuario> {
    let user = current_session_user(state_sesion)?;
    ensure_superadmin(&user)?;
    Ok(user)
}

fn require_superadmin_or_initial_setup(
    conn: &Connection,
    state_sesion: &tauri::State<SesionActual>,
) -> AppResult<()> {
    let sesion = state_sesion
        .0
        .lock()
        .map_err(|_| "No se pudo leer la sesión actual.".to_string())?;

    if let Some(user) = sesion.as_ref() {
        return ensure_superadmin(user);
    }

    let user_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM usuarios WHERE eliminado = 0", [], |row| row.get(0))
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if user_count == 0 {
        Ok(())
    } else {
        Err("Esta operación requiere una sesión activa de SUPERADMIN.".to_string())
    }
}

fn scoped_sucursal_for_read(user: &Usuario, requested_sucursal_id: Option<String>) -> Option<String> {
    if is_superadmin(user) {
        requested_sucursal_id
    } else {
        Some(user.sucursal_id.clone())
    }
}

fn ensure_can_read_sucursal(user: &Usuario, sucursal_id: &str) -> AppResult<()> {
    if is_superadmin(user) || user.sucursal_id == sucursal_id {
        Ok(())
    } else {
        Err("No tienes permisos para consultar información de otra sucursal.".to_string())
    }
}

fn get_active_usuario_by_id(conn: &Connection, id: &str) -> AppResult<Usuario> {
    conn.query_row(
        "SELECT id, email, nombre, role, sucursal_id FROM usuarios WHERE id = ?1 AND eliminado = 0",
        [id],
        |row| {
            Ok(Usuario {
                id: row.get(0)?,
                email: row.get(1)?,
                nombre: row.get(2)?,
                role: row.get(3)?,
                sucursal_id: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(AppError::from)
    .map_err(to_command_error)?
    .ok_or_else(|| "No se encontró el usuario indicado.".to_string())
}

fn count_active_superadmins(conn: &Connection) -> AppResult<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM usuarios WHERE role = 'SUPERADMIN' AND eliminado = 0",
        [],
        |row| row.get(0),
    )
    .map_err(AppError::from)
    .map_err(to_command_error)
}

fn ensure_can_manage_usuario(
    conn: &Connection,
    actor: &Usuario,
    target: Option<&Usuario>,
    nuevo_usuario: Option<&Usuario>,
) -> AppResult<()> {
    if let Some(target_user) = target {
        if actor.id == target_user.id {
            return Err("Operación inválida: No puedes eliminar ni modificar tu propia cuenta.".to_string());
        }
    }

    if let Some(new_user) = nuevo_usuario {
        if !is_superadmin(actor) {
            if new_user.role != "USUARIO" || new_user.sucursal_id != actor.sucursal_id {
                return Err("Operación inválida: Un administrador solo puede crear o modificar usuarios de rol USUARIO en su propia sucursal.".to_string());
            }
        }

        if let Some(target_user) = target {
            if target_user.role == "SUPERADMIN" && new_user.role != "SUPERADMIN" && count_active_superadmins(conn)? <= 1 {
                return Err("Operación inválida: El sistema no puede quedarse sin un Superadmin activo.".to_string());
            }
        }
    } else if let Some(target_user) = target {
        if target_user.role == "SUPERADMIN" && count_active_superadmins(conn)? <= 1 {
            return Err("Operación inválida: El sistema no puede quedarse sin un Superadmin activo.".to_string());
        }
    }

    if !is_superadmin(actor) {
        if let Some(target_user) = target {
            if target_user.sucursal_id != actor.sucursal_id {
                return Err("Operación inválida: Solo puedes administrar usuarios de tu sucursal.".to_string());
            }
            if target_user.role != "USUARIO" {
                return Err("Operación inválida: Un administrador solo puede modificar usuarios operativos.".to_string());
            }
        } else if nuevo_usuario.is_none() {
            return Err("Operación inválida: No se encontró el usuario indicado.".to_string());
        }
    }

    Ok(())
}

fn map_write_error(error: SqliteError, entity: &str) -> AppError {
    match error {
        SqliteError::SqliteFailure(_, Some(message)) if message.contains("UNIQUE") => {
            AppError::Conflict(format!("Ya existe un registro de {entity} con esos datos."))
        }
        SqliteError::SqliteFailure(_, Some(message)) if message.contains("FOREIGN KEY") => {
            AppError::Conflict("La operación viola la relación entre sucursales y usuarios.".to_string())
        }
        other => AppError::from(other),
    }
}

fn init_db(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA busy_timeout = 5000;
        PRAGMA foreign_keys = ON;
        "
    )?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sucursales (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL,
            direccion TEXT NOT NULL,
            telefono TEXT NOT NULL DEFAULT '',
            codigo_postal TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS empresa_config_fiscal (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            rfc TEXT NOT NULL DEFAULT '',
            razon_social TEXT NOT NULL DEFAULT '',
            regimen_fiscal TEXT NOT NULL DEFAULT '',
            registro_patronal TEXT NULL,
            actualizado_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS supabase_config (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            url TEXT NOT NULL DEFAULT '',
            anon_key TEXT NOT NULL DEFAULT '',
            is_connected INTEGER NOT NULL DEFAULT 0 CHECK(is_connected IN (0, 1))
        );

        CREATE TABLE IF NOT EXISTS sync_runtime_status (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            ultimo_intento_at TEXT NULL,
            ultimo_exito_at TEXT NULL,
            ultimo_error_at TEXT NULL,
            ultimo_error TEXT NULL
        );

        CREATE TABLE IF NOT EXISTS perifericos_config (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            impresora_tickets TEXT NOT NULL DEFAULT '',
            impresora_etiquetas TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS notificaciones (
            id TEXT PRIMARY KEY,
            categoria TEXT NOT NULL,
            severidad TEXT NOT NULL CHECK(severidad IN ('INFO', 'WARNING', 'CRITICAL')),
            titulo TEXT NOT NULL,
            mensaje TEXT NOT NULL,
            entidad_tipo TEXT NULL,
            entidad_id TEXT NULL,
            event_key TEXT NOT NULL UNIQUE,
            leida INTEGER NOT NULL DEFAULT 0 CHECK(leida IN (0, 1)),
            creada_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS usuarios (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            nombre TEXT NOT NULL,
            role TEXT NOT NULL CHECK(role IN ('SUPERADMIN', 'ADMIN', 'USUARIO')),
            sucursal_id TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS proveedores (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL,
            contacto_nombre TEXT NOT NULL DEFAULT '',
            telefono TEXT NOT NULL DEFAULT '',
            email TEXT NOT NULL DEFAULT '',
            direccion TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS marcas (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS categorias (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS unidades (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE,
            clave_sat TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS productos (
            id TEXT PRIMARY KEY,
            codigo_barras TEXT UNIQUE,
            codigo_proveedor TEXT NOT NULL DEFAULT '',
            proveedor_id TEXT NOT NULL,
            clave_producto TEXT NOT NULL DEFAULT '',
            descripcion TEXT NOT NULL,
            marca TEXT NOT NULL DEFAULT '',
            categoria TEXT NOT NULL DEFAULT '',
            unidad TEXT NOT NULL DEFAULT '',
            precio_costo REAL NOT NULL DEFAULT 0,
            costo_promedio REAL NOT NULL DEFAULT 0,
            precio_venta REAL NOT NULL DEFAULT 0,
            sat_clave_prod_serv TEXT NOT NULL DEFAULT '',
            sat_clave_unidad TEXT NOT NULL DEFAULT '',
            precio_1 REAL NOT NULL DEFAULT 0,
            precio_2 REAL NOT NULL DEFAULT 0,
            precio_3 REAL NOT NULL DEFAULT 0,
            precio_4 REAL NOT NULL DEFAULT 0,
            mayoreo_apartir REAL NOT NULL DEFAULT 0,
            a_granel INTEGER NOT NULL DEFAULT 0 CHECK(a_granel IN (0, 1)),
            no_en_catalogo INTEGER NOT NULL DEFAULT 0 CHECK(no_en_catalogo IN (0, 1)),
            ventas_negativas INTEGER NOT NULL DEFAULT 0 CHECK(ventas_negativas IN (0, 1)),
            caducidad TEXT NULL,
            fotos TEXT NOT NULL DEFAULT '',
            descripcion_catalogo TEXT NOT NULL DEFAULT '',
            FOREIGN KEY (proveedor_id) REFERENCES proveedores(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS inventario_sucursal (
            producto_id TEXT NOT NULL,
            sucursal_id TEXT NOT NULL,
            stock REAL NOT NULL DEFAULT 0,
            stock_minimo REAL NOT NULL DEFAULT 0,
            costo_promedio REAL NOT NULL DEFAULT 0,
            precio_venta REAL NOT NULL DEFAULT 0,
            PRIMARY KEY (producto_id, sucursal_id),
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE CASCADE,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS promociones (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL,
            tipo_descuento TEXT NOT NULL CHECK(tipo_descuento IN ('PORCENTAJE', 'MONTO_FIJO')),
            valor REAL NOT NULL,
            fecha_inicio TEXT NOT NULL,
            fecha_fin TEXT NOT NULL,
            activo INTEGER NOT NULL DEFAULT 1 CHECK(activo IN (0, 1)),
            producto_id TEXT NULL,
            categoria_id TEXT NULL,
            marca TEXT NULL,
            eliminado INTEGER NOT NULL DEFAULT 0 CHECK(eliminado IN (0, 1)),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS promocion_sucursales (
            promocion_id TEXT NOT NULL,
            sucursal_id TEXT NOT NULL,
            eliminado INTEGER NOT NULL DEFAULT 0 CHECK(eliminado IN (0, 1)),
            sincronizado INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (promocion_id, sucursal_id),
            FOREIGN KEY (promocion_id) REFERENCES promociones(id) ON UPDATE CASCADE ON DELETE CASCADE,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS compras (
            id TEXT PRIMARY KEY,
            proveedor_id TEXT NOT NULL,
            sucursal_id TEXT NOT NULL,
            fecha TEXT NOT NULL,
            total REAL NOT NULL DEFAULT 0,
            FOREIGN KEY (proveedor_id) REFERENCES proveedores(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS detalle_compras (
            id TEXT PRIMARY KEY,
            compra_id TEXT NOT NULL,
            producto_id TEXT NOT NULL,
            cantidad REAL NOT NULL DEFAULT 0,
            precio_costo_pactado REAL NOT NULL DEFAULT 0,
            costo_promedio_resultante REAL NULL,
            FOREIGN KEY (compra_id) REFERENCES compras(id) ON UPDATE CASCADE ON DELETE CASCADE,
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS ventas (
            id TEXT PRIMARY KEY,
            usuario_id TEXT NOT NULL,
            sucursal_id TEXT NOT NULL,
            fecha TEXT NOT NULL,
            total REAL NOT NULL DEFAULT 0,
            metodo_pago TEXT NOT NULL,
            efectivo_recibido REAL NULL,
            cambio_entregado REAL NULL,
            cliente_id TEXT NULL,
            cliente_rapido_nombre TEXT NULL,
            cliente_rapido_telefono TEXT NULL,
            cliente_rapido_domicilio TEXT NULL,
            requiere_factura INTEGER NOT NULL DEFAULT 0 CHECK(requiere_factura IN (0, 1)),
            usuario_autorizo_cancelacion_id TEXT NULL,
            motivo_cancelacion TEXT NULL,
            fecha_cancelacion TEXT NULL,
            estado TEXT NOT NULL DEFAULT 'COMPLETADA' CHECK(estado IN ('COMPLETADA', 'CANCELADA')),
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (cliente_id) REFERENCES clientes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (usuario_autorizo_cancelacion_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS detalle_ventas (
            id TEXT PRIMARY KEY,
            venta_id TEXT NOT NULL,
            producto_id TEXT NOT NULL,
            cantidad REAL NOT NULL DEFAULT 0,
            precio_venta_pactado REAL NOT NULL DEFAULT 0,
            costo_unitario_pactado REAL NOT NULL DEFAULT 0,
            tipo_precio_vendido TEXT NOT NULL DEFAULT 'MOSTRADOR',
            precio_original REAL NOT NULL DEFAULT 0,
            descuento_aplicado REAL NOT NULL DEFAULT 0,
            FOREIGN KEY (venta_id) REFERENCES ventas(id) ON UPDATE CASCADE ON DELETE CASCADE,
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS clientes (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL,
            telefono TEXT NOT NULL DEFAULT '',
            direccion TEXT NOT NULL DEFAULT '',
            limite_credito REAL NOT NULL DEFAULT 0,
            saldo_deudor REAL NOT NULL DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS clientes_datos_fiscales (
            cliente_id TEXT PRIMARY KEY,
            rfc TEXT NOT NULL UNIQUE,
            razon_social TEXT NOT NULL,
            regimen_fiscal TEXT NOT NULL,
            codigo_postal TEXT NOT NULL,
            FOREIGN KEY (cliente_id) REFERENCES clientes(id) ON UPDATE CASCADE ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS creditos_abonos (
            id TEXT PRIMARY KEY,
            cliente_id TEXT NOT NULL,
            monto REAL NOT NULL DEFAULT 0,
            fecha TEXT NOT NULL,
            usuario_id TEXT NOT NULL,
            FOREIGN KEY (cliente_id) REFERENCES clientes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS cajas_sesiones (
            id TEXT PRIMARY KEY,
            usuario_id TEXT NOT NULL,
            sucursal_id TEXT NOT NULL,
            fecha_apertura TEXT NOT NULL,
            monto_inicial REAL NOT NULL DEFAULT 0,
            fecha_cierre TEXT NULL,
            monto_final_real REAL NULL,
            monto_esperado REAL NOT NULL DEFAULT 0,
            estado TEXT NOT NULL CHECK(estado IN ('ABIERTA', 'CERRADA')),
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS caja_movimientos (
            id TEXT PRIMARY KEY,
            sesion_id TEXT NOT NULL,
            tipo TEXT NOT NULL CHECK(tipo IN ('INGRESO', 'EGRESO')),
            monto REAL NOT NULL DEFAULT 0,
            motivo TEXT NOT NULL DEFAULT '',
            FOREIGN KEY (sesion_id) REFERENCES cajas_sesiones(id) ON UPDATE CASCADE ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS traspasos (
            id TEXT PRIMARY KEY,
            sucursal_origen_id TEXT NOT NULL,
            sucursal_destino_id TEXT NOT NULL,
            usuario_id TEXT NOT NULL,
            fecha TEXT NOT NULL,
            estado TEXT NOT NULL DEFAULT 'EN_TRANSITO' CHECK(estado IN ('EN_TRANSITO', 'RECIBIDO', 'RECHAZADO', 'CANCELADO')),
            usuario_recibio_id TEXT NULL,
            fecha_recepcion TEXT NULL,
            observaciones_recepcion TEXT NULL,
            FOREIGN KEY (sucursal_origen_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (sucursal_destino_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (usuario_recibio_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS detalle_traspasos (
            id TEXT PRIMARY KEY,
            traspaso_id TEXT NOT NULL,
            producto_id TEXT NOT NULL,
            cantidad REAL NOT NULL DEFAULT 0,
            FOREIGN KEY (traspaso_id) REFERENCES traspasos(id) ON UPDATE CASCADE ON DELETE CASCADE,
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS mermas_ajustes (
            id TEXT PRIMARY KEY,
            producto_id TEXT NOT NULL,
            sucursal_id TEXT NOT NULL,
            usuario_id TEXT NOT NULL,
            cantidad REAL NOT NULL DEFAULT 0,
            tipo_movimiento TEXT NOT NULL CHECK(tipo_movimiento IN ('MERMA', 'AJUSTE', 'AJUSTE_ENTRADA', 'AJUSTE_SALIDA')),
            motivo TEXT NOT NULL,
            fecha TEXT NOT NULL,
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS facturas_emitidas (
            id TEXT PRIMARY KEY,
            venta_id TEXT NOT NULL UNIQUE,
            uuid TEXT NULL,
            rfc_receptor TEXT NOT NULL,
            monto_total REAL NOT NULL DEFAULT 0,
            estado TEXT NOT NULL DEFAULT 'PENDIENTE' CHECK(estado IN ('PENDIENTE', 'TIMBRADA', 'CANCELADA')),
            fecha_emision TEXT NOT NULL,
            pdf_path TEXT NULL,
            xml_path TEXT NULL,
            FOREIGN KEY (venta_id) REFERENCES ventas(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS movimientos_inventario (
            uuid TEXT PRIMARY KEY,
            producto_id TEXT NOT NULL,
            sucursal_id TEXT NOT NULL,
            tipo TEXT NOT NULL CHECK(tipo IN ('COMPRA', 'VENTA', 'CANCELACION_VENTA', 'TRASPASO_SALIDA', 'TRASPASO_ENTRADA', 'TRASPASO_RECHAZO', 'MERMA', 'AJUSTE_ENTRADA', 'AJUSTE_SALIDA')),
            referencia_tipo TEXT NOT NULL,
            referencia_id TEXT NOT NULL,
            cantidad REAL NOT NULL,
            costo_unitario REAL NULL,
            usuario_id TEXT NULL,
            fecha TEXT NOT NULL,
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_productos_descripcion ON productos(descripcion);
        CREATE INDEX IF NOT EXISTS idx_marcas_nombre ON marcas(nombre);
        CREATE INDEX IF NOT EXISTS idx_categorias_nombre ON categorias(nombre);
        CREATE INDEX IF NOT EXISTS idx_unidades_nombre ON unidades(nombre);
        CREATE INDEX IF NOT EXISTS idx_productos_codigo_barras ON productos(codigo_barras);
        CREATE INDEX IF NOT EXISTS idx_productos_clave_producto ON productos(clave_producto);
        CREATE INDEX IF NOT EXISTS idx_productos_codigo_proveedor ON productos(codigo_proveedor);
        CREATE INDEX IF NOT EXISTS idx_productos_descripcion_nocase ON productos(descripcion COLLATE NOCASE);
        CREATE INDEX IF NOT EXISTS idx_productos_marca_nocase ON productos(marca COLLATE NOCASE);
        CREATE INDEX IF NOT EXISTS idx_productos_codigo_barras_nocase ON productos(codigo_barras COLLATE NOCASE);
        CREATE INDEX IF NOT EXISTS idx_productos_codigo_proveedor_nocase ON productos(codigo_proveedor COLLATE NOCASE);
        CREATE INDEX IF NOT EXISTS idx_productos_clave_producto_nocase ON productos(clave_producto COLLATE NOCASE);
        CREATE INDEX IF NOT EXISTS idx_inventario_sucursal_id ON inventario_sucursal(sucursal_id);
        CREATE INDEX IF NOT EXISTS idx_promociones_producto ON promociones(producto_id);
        CREATE INDEX IF NOT EXISTS idx_promociones_categoria ON promociones(categoria_id);
        CREATE INDEX IF NOT EXISTS idx_promociones_vigencia ON promociones(activo, fecha_inicio, fecha_fin, eliminado);
        CREATE INDEX IF NOT EXISTS idx_promocion_sucursales_sucursal ON promocion_sucursales(sucursal_id);
        CREATE INDEX IF NOT EXISTS idx_compras_sucursal_fecha ON compras(sucursal_id, fecha);
        CREATE INDEX IF NOT EXISTS idx_detalle_compras_compra ON detalle_compras(compra_id);
        CREATE INDEX IF NOT EXISTS idx_ventas_sucursal_fecha ON ventas(sucursal_id, fecha);
        CREATE INDEX IF NOT EXISTS idx_ventas_cancelacion_usuario ON ventas(usuario_autorizo_cancelacion_id);
        CREATE INDEX IF NOT EXISTS idx_detalle_ventas_venta ON detalle_ventas(venta_id);
        CREATE INDEX IF NOT EXISTS idx_clientes_nombre ON clientes(nombre);
        CREATE INDEX IF NOT EXISTS idx_abonos_cliente_fecha ON creditos_abonos(cliente_id, fecha);
        CREATE INDEX IF NOT EXISTS idx_cajas_sesiones_usuario_estado ON cajas_sesiones(usuario_id, sucursal_id, estado);
        CREATE INDEX IF NOT EXISTS idx_caja_movimientos_sesion ON caja_movimientos(sesion_id);
        CREATE INDEX IF NOT EXISTS idx_traspasos_fecha ON traspasos(fecha);
        CREATE INDEX IF NOT EXISTS idx_detalle_traspasos_traspaso ON detalle_traspasos(traspaso_id);
        CREATE INDEX IF NOT EXISTS idx_mermas_fecha ON mermas_ajustes(fecha);
        CREATE INDEX IF NOT EXISTS idx_mermas_sucursal ON mermas_ajustes(sucursal_id);
        CREATE INDEX IF NOT EXISTS idx_movimientos_inventario_producto_sucursal ON movimientos_inventario(producto_id, sucursal_id, fecha);
        CREATE INDEX IF NOT EXISTS idx_movimientos_inventario_referencia ON movimientos_inventario(referencia_tipo, referencia_id);
        CREATE INDEX IF NOT EXISTS idx_clientes_datos_fiscales_rfc ON clientes_datos_fiscales(rfc);
        CREATE INDEX IF NOT EXISTS idx_facturas_emitidas_venta ON facturas_emitidas(venta_id);
        CREATE INDEX IF NOT EXISTS idx_facturas_emitidas_estado_fecha ON facturas_emitidas(estado, fecha_emision);
        CREATE INDEX IF NOT EXISTS idx_notificaciones_leida_fecha ON notificaciones(leida, creada_at);
        CREATE INDEX IF NOT EXISTS idx_notificaciones_categoria ON notificaciones(categoria);
        ",
    )?;

    migrate_user_role_schema(conn)?;
    migrate_sucursales_add_codigo_postal(conn)?;
    migrate_productos_add_proveedor(conn)?;
    migrate_productos_add_sat_claves(conn)?;
    migrate_ventas_add_estado_cliente(conn)?;
    migrate_ventas_add_pago_efectivo(conn)?;
    migrate_ventas_add_cancelacion_auditoria(conn)?;
    migrate_facturacion_cfdi40(conn)?;
    migrate_supabase_config(conn)?;
    migrate_sync_runtime_status(conn)?;
    migrate_perifericos_config(conn)?;
    migrate_notificaciones(conn)?;
    migrate_marcas_unidades(conn)?;
    migrate_inventario_empresarial(conn)?;
    migrate_promociones(conn)?;
    migrate_legacy_business_fields(conn)?;
    migrate_add_sincronizacion_fields(conn)?;
    migrate_performance_indexes(conn)?;
    Ok(())
}

fn add_column_if_missing(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<(), AppError> {
    if !table_has_column(conn, table, column)? {
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"), [])?;
    }
    Ok(())
}

fn migrate_legacy_business_fields(conn: &Connection) -> Result<(), AppError> {
    for (column, definition) in [
        ("precio_1", "REAL NOT NULL DEFAULT 0"),
        ("precio_2", "REAL NOT NULL DEFAULT 0"),
        ("precio_3", "REAL NOT NULL DEFAULT 0"),
        ("precio_4", "REAL NOT NULL DEFAULT 0"),
        ("mayoreo_apartir", "REAL NOT NULL DEFAULT 0"),
        ("a_granel", "INTEGER NOT NULL DEFAULT 0"),
        ("no_en_catalogo", "INTEGER NOT NULL DEFAULT 0"),
        ("ventas_negativas", "INTEGER NOT NULL DEFAULT 0"),
        ("caducidad", "TEXT NULL"),
        ("fotos", "TEXT NOT NULL DEFAULT ''"),
        ("descripcion_catalogo", "TEXT NOT NULL DEFAULT ''"),
    ] {
        add_column_if_missing(conn, "productos", column, definition)?;
    }
    for (column, definition) in [
        ("cliente_rapido_nombre", "TEXT NULL"),
        ("cliente_rapido_telefono", "TEXT NULL"),
        ("cliente_rapido_domicilio", "TEXT NULL"),
        ("requiere_factura", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        add_column_if_missing(conn, "ventas", column, definition)?;
    }
    for (column, definition) in [
        ("tipo_precio_vendido", "TEXT NOT NULL DEFAULT 'MOSTRADOR'"),
        ("precio_original", "REAL NOT NULL DEFAULT 0"),
        ("descuento_aplicado", "REAL NOT NULL DEFAULT 0"),
    ] {
        add_column_if_missing(conn, "detalle_ventas", column, definition)?;
    }
    Ok(())
}

fn migrate_promociones(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS promociones (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL,
            tipo_descuento TEXT NOT NULL CHECK(tipo_descuento IN ('PORCENTAJE', 'MONTO_FIJO')),
            valor REAL NOT NULL,
            fecha_inicio TEXT NOT NULL,
            fecha_fin TEXT NOT NULL,
            activo INTEGER NOT NULL DEFAULT 1 CHECK(activo IN (0, 1)),
            producto_id TEXT NULL,
            categoria_id TEXT NULL,
            marca TEXT NULL,
            eliminado INTEGER NOT NULL DEFAULT 0 CHECK(eliminado IN (0, 1)),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS promocion_sucursales (
            promocion_id TEXT NOT NULL,
            sucursal_id TEXT NOT NULL,
            eliminado INTEGER NOT NULL DEFAULT 0 CHECK(eliminado IN (0, 1)),
            sincronizado INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (promocion_id, sucursal_id),
            FOREIGN KEY (promocion_id) REFERENCES promociones(id) ON UPDATE CASCADE ON DELETE CASCADE,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_promociones_producto ON promociones(producto_id);
        CREATE INDEX IF NOT EXISTS idx_promociones_categoria ON promociones(categoria_id);
        CREATE INDEX IF NOT EXISTS idx_promociones_vigencia ON promociones(activo, fecha_inicio, fecha_fin, eliminado);
        CREATE INDEX IF NOT EXISTS idx_promocion_sucursales_sucursal ON promocion_sucursales(sucursal_id);
        ",
    )?;
    if !table_has_column(conn, "promocion_sucursales", "eliminado")? {
        conn.execute(
            "ALTER TABLE promocion_sucursales ADD COLUMN eliminado INTEGER NOT NULL DEFAULT 0 CHECK(eliminado IN (0, 1))",
            [],
        )?;
    }
    if !table_has_column(conn, "promociones", "marca")? {
        conn.execute("ALTER TABLE promociones ADD COLUMN marca TEXT NULL", [])?;
    }
    Ok(())
}

fn migrate_marcas_unidades(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS marcas (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS categorias (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS unidades (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE,
            clave_sat TEXT NOT NULL DEFAULT ''
        );
        CREATE INDEX IF NOT EXISTS idx_marcas_nombre ON marcas(nombre);
        CREATE INDEX IF NOT EXISTS idx_categorias_nombre ON categorias(nombre);
        CREATE INDEX IF NOT EXISTS idx_unidades_nombre ON unidades(nombre);
        ",
    )?;

    let mut stmt = conn.prepare("SELECT DISTINCT TRIM(marca) FROM productos WHERE TRIM(marca) <> ''")?;
    let marcas = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut marcas_existentes = Vec::new();
    for marca in marcas {
        marcas_existentes.push(marca?);
    }
    drop(stmt);
    for marca in marcas_existentes {
        conn.execute(
            "
            INSERT OR IGNORE INTO marcas (id, nombre)
            VALUES (?1, ?2)
            ",
            params![format!("MARCA-{}", normalize_upper_trim(&marca).replace(' ', "-")), marca],
        )?;
    }

    let mut stmt = conn.prepare("SELECT DISTINCT TRIM(categoria) FROM productos WHERE TRIM(categoria) <> ''")?;
    let categorias = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut categorias_existentes = Vec::new();
    for categoria in categorias {
        categorias_existentes.push(categoria?);
    }
    drop(stmt);
    for categoria in categorias_existentes {
        conn.execute(
            "
            INSERT OR IGNORE INTO categorias (id, nombre)
            VALUES (?1, ?2)
            ",
            params![format!("CATEGORIA-{}", normalize_upper_trim(&categoria).replace(' ', "-")), categoria],
        )?;
    }

    let mut stmt = conn.prepare(
        "SELECT DISTINCT TRIM(unidad), TRIM(sat_clave_unidad) FROM productos WHERE TRIM(unidad) <> ''",
    )?;
    let unidades = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
    let mut unidades_existentes = Vec::new();
    for unidad in unidades {
        unidades_existentes.push(unidad?);
    }
    drop(stmt);
    for (unidad, clave_sat) in unidades_existentes {
        conn.execute(
            "
            INSERT OR IGNORE INTO unidades (id, nombre, clave_sat)
            VALUES (?1, ?2, ?3)
            ",
            params![
                format!("UNIDAD-{}", normalize_upper_trim(&unidad).replace(' ', "-")),
                unidad,
                normalize_upper_trim(&clave_sat)
            ],
        )?;
    }

    Ok(())
}

fn migrate_inventario_empresarial(conn: &Connection) -> Result<(), AppError> {
    if !table_has_column(conn, "productos", "costo_promedio")? {
        conn.execute("ALTER TABLE productos ADD COLUMN costo_promedio REAL NOT NULL DEFAULT 0", [])?;
        conn.execute(
            "UPDATE productos SET costo_promedio = precio_costo WHERE costo_promedio = 0 AND precio_costo > 0",
            [],
        )?;
    }
    if !table_has_column(conn, "inventario_sucursal", "costo_promedio")? {
        conn.execute(
            "ALTER TABLE inventario_sucursal ADD COLUMN costo_promedio REAL NOT NULL DEFAULT 0",
            [],
        )?;
        conn.execute(
            "
            UPDATE inventario_sucursal
            SET costo_promedio = COALESCE((SELECT NULLIF(p.costo_promedio, 0) FROM productos p WHERE p.id = producto_id),
                                          (SELECT p.precio_costo FROM productos p WHERE p.id = producto_id),
                                          0)
            WHERE costo_promedio = 0
            ",
            [],
        )?;
    }
    if !table_has_column(conn, "inventario_sucursal", "precio_venta")? {
        conn.execute(
            "ALTER TABLE inventario_sucursal ADD COLUMN precio_venta REAL NOT NULL DEFAULT 0",
            [],
        )?;
        conn.execute(
            "
            UPDATE inventario_sucursal
            SET precio_venta = COALESCE((SELECT p.precio_venta FROM productos p WHERE p.id = producto_id), 0)
            WHERE precio_venta = 0
            ",
            [],
        )?;
    }
    if !table_has_column(conn, "detalle_compras", "costo_promedio_resultante")? {
        conn.execute(
            "ALTER TABLE detalle_compras ADD COLUMN costo_promedio_resultante REAL NULL",
            [],
        )?;
    }
    if !table_has_column(conn, "detalle_ventas", "costo_unitario_pactado")? {
        conn.execute(
            "ALTER TABLE detalle_ventas ADD COLUMN costo_unitario_pactado REAL NOT NULL DEFAULT 0",
            [],
        )?;
        conn.execute(
            "
            UPDATE detalle_ventas
            SET costo_unitario_pactado = COALESCE((SELECT NULLIF(p.costo_promedio, 0) FROM productos p WHERE p.id = detalle_ventas.producto_id),
                                                  (SELECT p.precio_costo FROM productos p WHERE p.id = detalle_ventas.producto_id),
                                                  0)
            WHERE costo_unitario_pactado = 0
            ",
            [],
        )?;
    }
    for (column, definition) in [
        ("estado", "TEXT NOT NULL DEFAULT 'RECIBIDO'"),
        ("usuario_recibio_id", "TEXT NULL"),
        ("fecha_recepcion", "TEXT NULL"),
        ("observaciones_recepcion", "TEXT NULL"),
    ] {
        if !table_has_column(conn, "traspasos", column)? {
            conn.execute(&format!("ALTER TABLE traspasos ADD COLUMN {column} {definition}"), [])?;
        }
    }
    conn.execute(
        "
        UPDATE traspasos
        SET estado = 'RECIBIDO',
            fecha_recepcion = COALESCE(fecha_recepcion, fecha)
        WHERE estado IS NULL OR estado = ''
        ",
        [],
    )?;
    let mermas_schema: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'mermas_ajustes'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    if mermas_schema
        .as_deref()
        .is_some_and(|sql| sql.contains("CHECK(tipo_movimiento IN ('MERMA', 'AJUSTE'))"))
    {
        conn.execute_batch(
            "
            ALTER TABLE mermas_ajustes RENAME TO mermas_ajustes_old;
            CREATE TABLE mermas_ajustes (
                id TEXT PRIMARY KEY,
                producto_id TEXT NOT NULL,
                sucursal_id TEXT NOT NULL,
                usuario_id TEXT NOT NULL,
                cantidad REAL NOT NULL DEFAULT 0,
                tipo_movimiento TEXT NOT NULL CHECK(tipo_movimiento IN ('MERMA', 'AJUSTE', 'AJUSTE_ENTRADA', 'AJUSTE_SALIDA')),
                motivo TEXT NOT NULL,
                fecha TEXT NOT NULL,
                FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
                FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT
            );
            INSERT INTO mermas_ajustes (id, producto_id, sucursal_id, usuario_id, cantidad, tipo_movimiento, motivo, fecha)
            SELECT id, producto_id, sucursal_id, usuario_id, cantidad, tipo_movimiento, motivo, fecha
            FROM mermas_ajustes_old;
            DROP TABLE mermas_ajustes_old;
            CREATE INDEX IF NOT EXISTS idx_mermas_fecha ON mermas_ajustes(fecha);
            CREATE INDEX IF NOT EXISTS idx_mermas_sucursal ON mermas_ajustes(sucursal_id);
            ",
        )?;
    }
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS movimientos_inventario (
            uuid TEXT PRIMARY KEY,
            producto_id TEXT NOT NULL,
            sucursal_id TEXT NOT NULL,
            tipo TEXT NOT NULL CHECK(tipo IN ('COMPRA', 'VENTA', 'CANCELACION_VENTA', 'TRASPASO_SALIDA', 'TRASPASO_ENTRADA', 'TRASPASO_RECHAZO', 'MERMA', 'AJUSTE_ENTRADA', 'AJUSTE_SALIDA')),
            referencia_tipo TEXT NOT NULL,
            referencia_id TEXT NOT NULL,
            cantidad REAL NOT NULL,
            costo_unitario REAL NULL,
            usuario_id TEXT NULL,
            fecha TEXT NOT NULL,
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );
        CREATE INDEX IF NOT EXISTS idx_movimientos_inventario_producto_sucursal ON movimientos_inventario(producto_id, sucursal_id, fecha);
        CREATE INDEX IF NOT EXISTS idx_movimientos_inventario_referencia ON movimientos_inventario(referencia_tipo, referencia_id);
        ",
    )?;
    Ok(())
}

fn migrate_notificaciones(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS notificaciones (
            id TEXT PRIMARY KEY,
            categoria TEXT NOT NULL,
            severidad TEXT NOT NULL CHECK(severidad IN ('INFO', 'WARNING', 'CRITICAL')),
            titulo TEXT NOT NULL,
            mensaje TEXT NOT NULL,
            entidad_tipo TEXT NULL,
            entidad_id TEXT NULL,
            event_key TEXT NOT NULL UNIQUE,
            leida INTEGER NOT NULL DEFAULT 0 CHECK(leida IN (0, 1)),
            creada_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        CREATE INDEX IF NOT EXISTS idx_notificaciones_leida_fecha ON notificaciones(leida, creada_at);
        CREATE INDEX IF NOT EXISTS idx_notificaciones_categoria ON notificaciones(categoria);
        ",
    )?;
    Ok(())
}

fn migrate_add_sincronizacion_fields(conn: &Connection) -> Result<(), AppError> {
    for table in SYNC_TABLES {
        if !table_has_column(conn, table, "sincronizado")? {
            conn.execute(
                &format!("ALTER TABLE {table} ADD COLUMN sincronizado INTEGER NOT NULL DEFAULT 0"),
                [],
            )?;
        }

        if !table_has_column(conn, table, "updated_at")? {
            conn.execute(
                &format!("ALTER TABLE {table} ADD COLUMN updated_at TEXT NOT NULL DEFAULT (datetime('now'))"),
                [],
            )?;
        }
    }

    for table in SOFT_DELETE_TABLES {
        if !table_has_column(conn, table, "eliminado")? {
            conn.execute(
                &format!("ALTER TABLE {table} ADD COLUMN eliminado INTEGER NOT NULL DEFAULT 0"),
                [],
            )?;
        }
        conn.execute(
            &format!("CREATE INDEX IF NOT EXISTS idx_{table}_eliminado ON {table}(eliminado)"),
            [],
        )?;
    }

    for table in UUID_SYNC_TABLES {
        if !table_has_column(conn, table, "sync_uuid")? {
            conn.execute(&format!("ALTER TABLE {table} ADD COLUMN sync_uuid TEXT NULL"), [])?;
            conn.execute(
                &format!("CREATE UNIQUE INDEX IF NOT EXISTS idx_{table}_sync_uuid ON {table}(sync_uuid) WHERE sync_uuid IS NOT NULL"),
                [],
            )?;
        }
    }
    ensure_sync_uuids(conn)?;

    for table in SYNC_TABLES {
        conn.execute(
            &format!("CREATE INDEX IF NOT EXISTS idx_{table}_sync_pending ON {table}(sincronizado, updated_at)"),
            [],
        )?;
    }

    conn.execute(
        "
        CREATE UNIQUE INDEX IF NOT EXISTS idx_cajas_sesiones_abierta_unica
        ON cajas_sesiones(usuario_id, sucursal_id)
        WHERE estado = 'ABIERTA'
        ",
        [],
    )?;

    create_sync_dirty_triggers(conn)?;

    Ok(())
}

fn sync_dirty_predicate(table: &str) -> Option<&'static str> {
    match table {
        "inventario_sucursal" => Some("producto_id = NEW.producto_id AND sucursal_id = NEW.sucursal_id"),
        "promocion_sucursales" => Some("promocion_id = NEW.promocion_id AND sucursal_id = NEW.sucursal_id"),
        "clientes_datos_fiscales" => Some("cliente_id = NEW.cliente_id"),
        "empresa_config_fiscal" => Some("id = NEW.id"),
        "movimientos_inventario" => Some("uuid = NEW.uuid"),
        _ => Some("id = NEW.id"),
    }
}

fn create_sync_dirty_triggers(conn: &Connection) -> Result<(), AppError> {
    for table in SYNC_TABLES {
        let Some(predicate) = sync_dirty_predicate(table) else {
            continue;
        };
        conn.execute_batch(&format!(
            "
            DROP TRIGGER IF EXISTS trg_{table}_mark_dirty;
            CREATE TRIGGER trg_{table}_mark_dirty
            AFTER UPDATE ON {table}
            WHEN NEW.sincronizado = OLD.sincronizado
            BEGIN
                UPDATE {table}
                SET sincronizado = 0,
                    updated_at = datetime('now')
                WHERE {predicate};
            END;
            "
        ))?;
    }

    Ok(())
}

fn migrate_performance_indexes(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_ventas_estado_fecha ON ventas(estado, fecha);
        CREATE INDEX IF NOT EXISTS idx_ventas_estado_sucursal_fecha ON ventas(estado, sucursal_id, fecha);
        CREATE INDEX IF NOT EXISTS idx_ventas_estado_metodo_fecha ON ventas(estado, metodo_pago, fecha);
        CREATE INDEX IF NOT EXISTS idx_ventas_estado_usuario_fecha ON ventas(estado, usuario_id, fecha);
        CREATE INDEX IF NOT EXISTS idx_detalle_ventas_producto_venta ON detalle_ventas(producto_id, venta_id);

        CREATE INDEX IF NOT EXISTS idx_inventario_bajo_stock_scope
            ON inventario_sucursal(sucursal_id, eliminado, stock_minimo, stock);
        CREATE INDEX IF NOT EXISTS idx_inventario_producto_sucursal_eliminado
            ON inventario_sucursal(producto_id, sucursal_id, eliminado);

        CREATE INDEX IF NOT EXISTS idx_traspasos_estado_fecha ON traspasos(estado, fecha);
        CREATE INDEX IF NOT EXISTS idx_traspasos_origen_estado_fecha ON traspasos(sucursal_origen_id, estado, fecha);
        CREATE INDEX IF NOT EXISTS idx_traspasos_destino_estado_fecha ON traspasos(sucursal_destino_id, estado, fecha);

        CREATE INDEX IF NOT EXISTS idx_mermas_sucursal_fecha ON mermas_ajustes(sucursal_id, fecha);
        CREATE INDEX IF NOT EXISTS idx_mermas_producto_fecha ON mermas_ajustes(producto_id, fecha);

        CREATE INDEX IF NOT EXISTS idx_facturas_emitidas_fecha ON facturas_emitidas(fecha_emision);
        CREATE INDEX IF NOT EXISTS idx_facturas_emitidas_venta_fecha ON facturas_emitidas(venta_id, fecha_emision);

        CREATE INDEX IF NOT EXISTS idx_caja_movimientos_sesion_tipo_fecha ON caja_movimientos(sesion_id, tipo, fecha);
        CREATE INDEX IF NOT EXISTS idx_compras_fecha ON compras(fecha);
        ",
    )?;
    Ok(())
}

fn migrate_supabase_config(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS supabase_config (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            url TEXT NOT NULL DEFAULT '',
            anon_key TEXT NOT NULL DEFAULT '',
            is_connected INTEGER NOT NULL DEFAULT 0 CHECK(is_connected IN (0, 1))
        );
        ",
    )?;
    Ok(())
}

fn migrate_sync_runtime_status(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sync_runtime_status (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            ultimo_intento_at TEXT NULL,
            ultimo_exito_at TEXT NULL,
            ultimo_error_at TEXT NULL,
            ultimo_error TEXT NULL
        );
        INSERT OR IGNORE INTO sync_runtime_status (id) VALUES (1);
        ",
    )?;
    Ok(())
}

fn migrate_perifericos_config(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS perifericos_config (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            impresora_tickets TEXT NOT NULL DEFAULT '',
            impresora_etiquetas TEXT NOT NULL DEFAULT '',
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        ",
    )?;
    Ok(())
}

fn migrate_productos_add_proveedor(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare("PRAGMA table_info(productos)")?;
    let columns_iter = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut has_proveedor_id = false;
    for col in columns_iter {
        if col? == "proveedor_id" {
            has_proveedor_id = true;
            break;
        }
    }

    if has_proveedor_id {
        return Ok(());
    }

    conn.execute(
        "ALTER TABLE productos ADD COLUMN proveedor_id TEXT NOT NULL DEFAULT ''",
        [],
    )?;
    Ok(())
}

fn migrate_sucursales_add_codigo_postal(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare("PRAGMA table_info(sucursales)")?;
    let columns_iter = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut has_codigo_postal = false;

    for col in columns_iter {
        if col? == "codigo_postal" {
            has_codigo_postal = true;
            break;
        }
    }

    if !has_codigo_postal {
        conn.execute("ALTER TABLE sucursales ADD COLUMN codigo_postal TEXT NOT NULL DEFAULT ''", [])?;
    }

    Ok(())
}

fn migrate_productos_add_sat_claves(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare("PRAGMA table_info(productos)")?;
    let columns_iter = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut has_sat_clave_prod_serv = false;
    let mut has_sat_clave_unidad = false;

    for col in columns_iter {
        match col?.as_str() {
            "sat_clave_prod_serv" => has_sat_clave_prod_serv = true,
            "sat_clave_unidad" => has_sat_clave_unidad = true,
            _ => {}
        }
    }

    if !has_sat_clave_prod_serv {
        conn.execute("ALTER TABLE productos ADD COLUMN sat_clave_prod_serv TEXT NOT NULL DEFAULT ''", [])?;
    }
    if !has_sat_clave_unidad {
        conn.execute("ALTER TABLE productos ADD COLUMN sat_clave_unidad TEXT NOT NULL DEFAULT ''", [])?;
    }

    Ok(())
}

fn migrate_facturacion_cfdi40(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS empresa_config_fiscal (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            rfc TEXT NOT NULL DEFAULT '',
            razon_social TEXT NOT NULL DEFAULT '',
            regimen_fiscal TEXT NOT NULL DEFAULT '',
            registro_patronal TEXT NULL,
            actualizado_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS clientes_datos_fiscales (
            cliente_id TEXT PRIMARY KEY,
            rfc TEXT NOT NULL UNIQUE,
            razon_social TEXT NOT NULL,
            regimen_fiscal TEXT NOT NULL,
            codigo_postal TEXT NOT NULL,
            FOREIGN KEY (cliente_id) REFERENCES clientes(id) ON UPDATE CASCADE ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS facturas_emitidas (
            id TEXT PRIMARY KEY,
            venta_id TEXT NOT NULL UNIQUE,
            uuid TEXT NULL,
            rfc_receptor TEXT NOT NULL,
            monto_total REAL NOT NULL DEFAULT 0,
            estado TEXT NOT NULL DEFAULT 'PENDIENTE' CHECK(estado IN ('PENDIENTE', 'TIMBRADA', 'CANCELADA')),
            fecha_emision TEXT NOT NULL,
            pdf_path TEXT NULL,
            xml_path TEXT NULL,
            FOREIGN KEY (venta_id) REFERENCES ventas(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_clientes_datos_fiscales_rfc ON clientes_datos_fiscales(rfc);
        CREATE INDEX IF NOT EXISTS idx_facturas_emitidas_venta ON facturas_emitidas(venta_id);
        CREATE INDEX IF NOT EXISTS idx_facturas_emitidas_estado_fecha ON facturas_emitidas(estado, fecha_emision);
        ",
    )?;

    Ok(())
}

fn migrate_ventas_add_estado_cliente(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare("PRAGMA table_info(ventas)")?;
    let columns_iter = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut has_estado = false;
    let mut has_cliente_id = false;

    for col in columns_iter {
        let name = col?;
        if name == "estado" {
            has_estado = true;
        } else if name == "cliente_id" {
            has_cliente_id = true;
        }
    }

    if !has_estado {
        conn.execute(
            "ALTER TABLE ventas ADD COLUMN estado TEXT NOT NULL DEFAULT 'COMPLETADA'",
            [],
        )?;
    }
    if !has_cliente_id {
        conn.execute("ALTER TABLE ventas ADD COLUMN cliente_id TEXT NULL", [])?;
    }

    Ok(())
}

fn migrate_ventas_add_pago_efectivo(conn: &Connection) -> Result<(), AppError> {
    if !table_has_column(conn, "ventas", "efectivo_recibido")? {
        conn.execute("ALTER TABLE ventas ADD COLUMN efectivo_recibido REAL NULL", [])?;
    }
    if !table_has_column(conn, "ventas", "cambio_entregado")? {
        conn.execute("ALTER TABLE ventas ADD COLUMN cambio_entregado REAL NULL", [])?;
    }

    Ok(())
}

fn migrate_ventas_add_cancelacion_auditoria(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare("PRAGMA table_info(ventas)")?;
    let columns_iter = stmt.query_map([], |row| row.get::<_, String>(1))?;
    let mut has_usuario_autorizo = false;
    let mut has_motivo = false;
    let mut has_fecha = false;

    for col in columns_iter {
        match col?.as_str() {
            "usuario_autorizo_cancelacion_id" => has_usuario_autorizo = true,
            "motivo_cancelacion" => has_motivo = true,
            "fecha_cancelacion" => has_fecha = true,
            _ => {}
        }
    }

    if !has_usuario_autorizo {
        conn.execute("ALTER TABLE ventas ADD COLUMN usuario_autorizo_cancelacion_id TEXT NULL", [])?;
    }
    if !has_motivo {
        conn.execute("ALTER TABLE ventas ADD COLUMN motivo_cancelacion TEXT NULL", [])?;
    }
    if !has_fecha {
        conn.execute("ALTER TABLE ventas ADD COLUMN fecha_cancelacion TEXT NULL", [])?;
    }

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_ventas_cancelacion_usuario ON ventas(usuario_autorizo_cancelacion_id)",
        [],
    )?;

    Ok(())
}

fn migrate_user_role_schema(conn: &Connection) -> Result<(), AppError> {
    let sql: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'usuarios'",
        [],
        |row| row.get(0),
    )?;

    if !sql.contains("'USER'") {
        conn.execute(
            "UPDATE usuarios SET role = 'USUARIO' WHERE role = 'USER'",
            [],
        )?;
        return Ok(());
    }

    conn.execute_batch(
        "
        PRAGMA foreign_keys = OFF;

        CREATE TABLE usuarios_new (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL UNIQUE,
            nombre TEXT NOT NULL,
            role TEXT NOT NULL CHECK(role IN ('SUPERADMIN', 'ADMIN', 'USUARIO')),
            sucursal_id TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        INSERT INTO usuarios_new (id, email, nombre, role, sucursal_id, password_hash)
        SELECT id, email, nombre, CASE WHEN role = 'USER' THEN 'USUARIO' ELSE role END, sucursal_id, password_hash
        FROM usuarios;

        DROP TABLE usuarios;
        ALTER TABLE usuarios_new RENAME TO usuarios;

        PRAGMA foreign_keys = ON;
        ",
    )?;

    Ok(())
}

#[tauri::command]
fn iniciar_sesion(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    email: String,
    clave: String,
) -> AppResult<Usuario> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let email_normalizado = normalize_email(&email);

    let (usuario, stored_password): (Usuario, String) = conn
        .query_row(
            "SELECT id, email, nombre, role, sucursal_id, password_hash FROM usuarios WHERE email = ?1 AND eliminado = 0",
            [&email_normalizado],
            |row| {
                Ok((
                    Usuario {
                        id: row.get(0)?,
                        email: row.get(1)?,
                        nombre: row.get(2)?,
                        role: row.get(3)?,
                        sucursal_id: row.get(4)?,
                    },
                    row.get(5)?,
                ))
            },
        )
        .map_err(|_| to_command_error(AppError::Auth("Credenciales inválidas o usuario no encontrado.".to_string())))?;

    let password_ok = verify_password_and_migrate(&conn, &usuario.id, &clave, &stored_password)
        .map_err(|_| to_command_error(AppError::Auth("Credenciales inválidas o usuario no encontrado.".to_string())))?;

    if !password_ok {
        return Err("Credenciales inválidas o usuario no encontrado.".to_string());
    }

    let mut sesion = state_sesion
        .0
        .lock()
        .map_err(|_| "No se pudo actualizar la sesión actual.".to_string())?;
    *sesion = Some(usuario.clone());

    Ok(usuario)
}

#[tauri::command]
fn get_sesion_actual(state_sesion: tauri::State<SesionActual>) -> AppResult<Option<Usuario>> {
    let sesion = state_sesion
        .0
        .lock()
        .map_err(|_| "No se pudo leer la sesión actual.".to_string())?;
    Ok(sesion.clone())
}

#[tauri::command]
fn cerrar_sesion_local(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<()> {
    let usuario_actual = {
        let sesion = state_sesion
            .0
            .lock()
            .map_err(|_| "No se pudo leer la sesión actual.".to_string())?;
        sesion.clone()
    };

    if let Some(usuario) = usuario_actual {
        let conn = get_conn(&state_db).map_err(to_command_error)?;
        let cajas_abiertas: i64 = conn
            .query_row(
                "
                SELECT COUNT(*)
                FROM cajas_sesiones
                WHERE usuario_id = ?1
                  AND sucursal_id = ?2
                  AND estado = 'ABIERTA'
                ",
                params![usuario.id, usuario.sucursal_id],
                |row| row.get(0),
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;

        if cajas_abiertas > 0 {
            return Err("No puedes cerrar sesión mientras tu caja esté ABIERTA. Realiza el corte de caja primero.".to_string());
        }
    }

    let mut sesion = state_sesion
        .0
        .lock()
        .map_err(|_| "No se pudo cerrar la sesión actual.".to_string())?;
    *sesion = None;
    Ok(())
}

#[tauri::command]
fn update_mi_perfil(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    perfil: PerfilUpdate,
) -> AppResult<Usuario> {
    validate_perfil_update(&perfil).map_err(to_command_error)?;

    let usuario_actual = {
        let sesion = state_sesion
            .0
            .lock()
            .map_err(|_| "No se pudo leer la sesión actual.".to_string())?;
        sesion
            .clone()
            .ok_or_else(|| "No hay una sesión activa para actualizar el perfil.".to_string())?
    };

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let stored_password: String = conn
        .query_row(
            "SELECT password_hash FROM usuarios WHERE id = ?1",
            [&usuario_actual.id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let password_ok = verify_password_and_migrate(
        &conn,
        &usuario_actual.id,
        &perfil.password_actual,
        &stored_password,
    )
    .map_err(|_| to_command_error(AppError::Auth("La contraseña actual no es correcta.".to_string())))?;

    if !password_ok {
        return Err("La contraseña actual no es correcta.".to_string());
    }

    let nombre = build_full_name(
        &perfil.nombres,
        &perfil.apellido_paterno,
        &perfil.apellido_materno,
    );
    let email = normalize_email(&perfil.email);
    let nueva_password_limpia = perfil
        .nueva_password
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(nueva_password) = nueva_password_limpia {
        let password_hash = hash(nueva_password, DEFAULT_COST)
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        conn.execute(
            "UPDATE usuarios SET nombre = ?1, email = ?2, password_hash = ?3 WHERE id = ?4 AND eliminado = 0",
            params![nombre, email, password_hash, usuario_actual.id],
        )
        .map_err(|error| map_write_error(error, "usuario"))
        .map_err(to_command_error)?;
    } else {
        conn.execute(
            "UPDATE usuarios SET nombre = ?1, email = ?2 WHERE id = ?3 AND eliminado = 0",
            params![nombre, email, usuario_actual.id],
        )
        .map_err(|error| map_write_error(error, "usuario"))
        .map_err(to_command_error)?;
    }

    let usuario_actualizado = conn
        .query_row(
            "SELECT id, email, nombre, role, sucursal_id FROM usuarios WHERE id = ?1 AND eliminado = 0",
            [&usuario_actual.id],
            |row| {
                Ok(Usuario {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    nombre: row.get(2)?,
                    role: row.get(3)?,
                    sucursal_id: row.get(4)?,
                })
            },
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut sesion = state_sesion
        .0
        .lock()
        .map_err(|_| "No se pudo actualizar la sesión actual.".to_string())?;
    *sesion = Some(usuario_actualizado.clone());

    Ok(usuario_actualizado)
}

#[tauri::command]
fn necesita_configuracion_inicial(state_db: tauri::State<DbState>) -> AppResult<bool> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM usuarios WHERE eliminado = 0", [], |row| row.get(0))
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    Ok(user_count == 0)
}

#[tauri::command]
fn crear_configuracion_inicial(
    state_db: tauri::State<DbState>,
    sucursal: Sucursal,
    usuario: Usuario,
    password: String,
) -> AppResult<()> {
    validate_sucursal(&sucursal).map_err(to_command_error)?;
    validate_usuario(&usuario, true).map_err(to_command_error)?;

    if password.trim().is_empty() {
        return Err("El primer usuario necesita contraseña.".to_string());
    }

    if usuario.sucursal_id != sucursal.id {
        return Err("El usuario inicial debe estar asociado a la sucursal creada.".to_string());
    }

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let user_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM usuarios WHERE eliminado = 0", [], |row| row.get(0))
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if user_count > 0 {
        return Err("El sistema ya tiene usuarios registrados.".to_string());
    }

    let password_hash = hash(password, DEFAULT_COST).map_err(AppError::from).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    tx.execute(
        "INSERT INTO sucursales (id, nombre, direccion, telefono, codigo_postal) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![sucursal.id, sucursal.nombre, sucursal.direccion, sucursal.telefono, sucursal.codigo_postal],
    )
    .map_err(|error| map_write_error(error, "sucursal"))
    .map_err(to_command_error)?;

    tx.execute(
        "INSERT INTO usuarios (id, email, nombre, role, sucursal_id, password_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            usuario.id,
            normalize_email(&usuario.email),
            usuario.nombre,
            usuario.role,
            usuario.sucursal_id,
            password_hash
        ],
    )
    .map_err(|error| map_write_error(error, "usuario"))
    .map_err(to_command_error)?;

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn get_usuarios(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Vec<Usuario>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let actor = current_session_user(&state_sesion)?;
    let sucursal_scope = scoped_sucursal_for_read(&actor, None);
    let mut sql = String::from("SELECT id, email, nombre, role, sucursal_id FROM usuarios WHERE eliminado = 0");
    let mut params_vec: Vec<String> = Vec::new();
    if let Some(sucursal_id) = sucursal_scope {
        sql.push_str(" AND sucursal_id = ?1");
        params_vec.push(sucursal_id);
    }
    sql.push_str(" ORDER BY nombre");

    let mut stmt = conn.prepare(&sql).map_err(AppError::from).map_err(to_command_error)?;

    let usuarios_iter = stmt
        .query_map(params_from_iter(params_vec.iter()), |row| {
            Ok(Usuario {
                id: row.get(0)?,
                email: row.get(1)?,
                nombre: row.get(2)?,
                role: row.get(3)?,
                sucursal_id: row.get(4)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut usuarios = Vec::new();
    for usuario in usuarios_iter {
        usuarios.push(usuario.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(usuarios)
}

#[tauri::command]
fn create_usuario(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    usuario: Usuario,
    password: String,
) -> AppResult<()> {
    validate_usuario(&usuario, false).map_err(to_command_error)?;

    if password.trim().is_empty() {
        return Err("El usuario necesita contraseña.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let actor = current_session_user(&state_sesion)?;
    ensure_can_manage_usuario(&conn, &actor, None, Some(&usuario))?;

    let password_hash = hash(password, DEFAULT_COST).map_err(AppError::from).map_err(to_command_error)?;

    conn.execute(
        "INSERT INTO usuarios (id, email, nombre, role, sucursal_id, password_hash) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            usuario.id,
            normalize_email(&usuario.email),
            usuario.nombre,
            usuario.role,
            usuario.sucursal_id,
            password_hash
        ],
    )
    .map_err(|error| map_write_error(error, "usuario"))
    .map_err(to_command_error)?;

    Ok(())
}

#[tauri::command]
fn update_usuario(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
    usuario: Usuario,
) -> AppResult<()> {
    validate_usuario(&usuario, false).map_err(to_command_error)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let actor = current_session_user(&state_sesion)?;
    let target = get_active_usuario_by_id(&conn, &id)?;
    ensure_can_manage_usuario(&conn, &actor, Some(&target), Some(&usuario))?;

    let affected = conn
        .execute(
            "
            UPDATE usuarios
            SET email = ?1,
                nombre = ?2,
                role = ?3,
                sucursal_id = ?4,
                sincronizado = 0,
                updated_at = datetime('now')
            WHERE id = ?5 AND eliminado = 0
            ",
            params![
                normalize_email(&usuario.email),
                usuario.nombre,
                usuario.role,
                usuario.sucursal_id,
                id
            ],
        )
        .map_err(|error| map_write_error(error, "usuario"))
        .map_err(to_command_error)?;

    if affected == 0 {
        return Err("No se encontró el usuario que intentas actualizar.".to_string());
    }

    Ok(())
}

#[tauri::command]
fn delete_usuario(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
) -> AppResult<()> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let actor = current_session_user(&state_sesion)?;
    let target = get_active_usuario_by_id(&conn, &id)?;
    ensure_can_manage_usuario(&conn, &actor, Some(&target), None)?;

    let affected = conn
        .execute(
            "UPDATE usuarios SET eliminado = 1, sincronizado = 0, updated_at = datetime('now') WHERE id = ?1 AND eliminado = 0",
            [&id],
        )
        .map_err(|error| map_write_error(error, "usuario"))
        .map_err(to_command_error)?;

    if affected == 0 {
        return Err("No se encontró el usuario que intentas eliminar.".to_string());
    }

    Ok(())
}

#[tauri::command]
fn get_sucursales(state_db: tauri::State<DbState>) -> AppResult<Vec<Sucursal>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, nombre, direccion, telefono, codigo_postal FROM sucursales WHERE eliminado = 0 ORDER BY nombre")
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let sucursales_iter = stmt
        .query_map([], |row| {
            Ok(Sucursal {
                id: row.get(0)?,
                nombre: row.get(1)?,
                direccion: row.get(2)?,
                telefono: row.get(3)?,
                codigo_postal: row.get(4)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut sucursales = Vec::new();
    for sucursal in sucursales_iter {
        sucursales.push(sucursal.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(sucursales)
}

#[tauri::command]
fn get_proveedores(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Vec<Proveedor>> {
    require_admin_or_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, TRIM(COALESCE(nombre, '')), TRIM(COALESCE(contacto_nombre, '')), TRIM(COALESCE(telefono, '')), TRIM(COALESCE(email, '')), TRIM(COALESCE(direccion, '')) FROM proveedores WHERE eliminado = 0 ORDER BY TRIM(nombre)")
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map([], |row| {
            Ok(Proveedor {
                id: row.get(0)?,
                nombre: row.get(1)?,
                contacto_nombre: row.get(2)?,
                telefono: row.get(3)?,
                email: row.get(4)?,
                direccion: row.get(5)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut proveedores = Vec::new();
    for item in iter {
        proveedores.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(proveedores)
}

#[tauri::command]
fn create_proveedor(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    proveedor: Proveedor,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let proveedor = Proveedor {
        id: proveedor.id.trim().to_string(),
        nombre: normalize_title_trim(&proveedor.nombre),
        contacto_nombre: normalize_title_trim(&proveedor.contacto_nombre),
        telefono: normalize_plain_trim(&proveedor.telefono),
        email: normalize_email_trim(&proveedor.email),
        direccion: normalize_title_trim(&proveedor.direccion),
    };
    validate_proveedor(&proveedor).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_unique_catalog_name(&conn, "proveedores", "proveedor", None, &proveedor.nombre)
        .map_err(to_command_error)?;
    conn.execute(
        "
        INSERT INTO proveedores (
            id, nombre, contacto_nombre, telefono, email, direccion,
            sincronizado, updated_at, eliminado
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, datetime('now'), 0)
        ",
        params![
            proveedor.id,
            proveedor.nombre,
            proveedor.contacto_nombre,
            proveedor.telefono,
            proveedor.email,
            proveedor.direccion
        ],
    )
    .map_err(|error| map_write_error(error, "proveedor"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn update_proveedor(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
    proveedor: Proveedor,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let proveedor = Proveedor {
        id: id.trim().to_string(),
        nombre: normalize_title_trim(&proveedor.nombre),
        contacto_nombre: normalize_title_trim(&proveedor.contacto_nombre),
        telefono: normalize_plain_trim(&proveedor.telefono),
        email: normalize_email_trim(&proveedor.email),
        direccion: normalize_title_trim(&proveedor.direccion),
    };
    validate_proveedor(&proveedor).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_unique_catalog_name(&conn, "proveedores", "proveedor", Some(&id), &proveedor.nombre)
        .map_err(to_command_error)?;
    let affected = conn
        .execute(
            "
            UPDATE proveedores
            SET nombre = ?1,
                contacto_nombre = ?2,
                telefono = ?3,
                email = ?4,
                direccion = ?5,
                sincronizado = 0,
                updated_at = datetime('now')
            WHERE id = ?6 AND eliminado = 0
            ",
            params![
                proveedor.nombre,
                proveedor.contacto_nombre,
                proveedor.telefono,
                proveedor.email,
                proveedor.direccion,
                id
            ],
        )
        .map_err(|error| map_write_error(error, "proveedor"))
        .map_err(to_command_error)?;

    if affected == 0 {
        return Err("No se encontró el proveedor que intentas actualizar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn delete_provider(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let active_products: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM productos WHERE proveedor_id = ?1 AND eliminado = 0",
            [&id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if active_products > 0 {
        return Err("No se puede eliminar el proveedor porque tiene productos asociados.".to_string());
    }

    let affected = conn
        .execute(
            "UPDATE proveedores SET eliminado = 1, sincronizado = 0, updated_at = datetime('now') WHERE id = ?1 AND eliminado = 0",
            [&id],
        )
        .map_err(|error| map_write_error(error, "proveedor"))
        .map_err(to_command_error)?;
    if affected == 0 {
        return Err("No se encontró el proveedor que intentas eliminar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn get_marcas(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Vec<Marca>> {
    require_admin_or_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, TRIM(COALESCE(nombre, '')) FROM marcas WHERE eliminado = 0 ORDER BY TRIM(nombre)")
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let iter = stmt
        .query_map([], |row| Ok(Marca { id: row.get(0)?, nombre: row.get(1)? }))
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let mut marcas = Vec::new();
    for item in iter {
        marcas.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(marcas)
}

#[tauri::command]
fn create_marca(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    marca: Marca,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let marca = Marca { id: marca.id.trim().to_string(), nombre: normalize_title_trim(&marca.nombre) };
    validate_marca(&marca).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_unique_catalog_name(&conn, "marcas", "marca", None, &marca.nombre)
        .map_err(to_command_error)?;
    conn.execute(
        "INSERT INTO marcas (id, nombre, sincronizado, updated_at, eliminado) VALUES (?1, ?2, 0, datetime('now'), 0)",
        params![marca.id, marca.nombre],
    )
    .map_err(|error| map_write_error(error, "marca"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn update_marca(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
    marca: Marca,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let marca = Marca { id: id.trim().to_string(), nombre: normalize_title_trim(&marca.nombre) };
    validate_marca(&marca).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_unique_catalog_name(&conn, "marcas", "marca", Some(&id), &marca.nombre)
        .map_err(to_command_error)?;
    let affected = conn
        .execute(
            "UPDATE marcas SET nombre = ?1, sincronizado = 0, updated_at = datetime('now') WHERE id = ?2 AND eliminado = 0",
            params![marca.nombre, id],
        )
        .map_err(|error| map_write_error(error, "marca"))
        .map_err(to_command_error)?;
    if affected == 0 {
        return Err("No se encontró la marca que intentas actualizar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn delete_marca(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let marca_nombre: String = conn
        .query_row("SELECT nombre FROM marcas WHERE id = ?1 AND eliminado = 0", [&id], |row| row.get(0))
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?
        .ok_or_else(|| "No se encontró la marca que intentas eliminar.".to_string())?;
    let active_products: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM productos WHERE marca = ?1 AND eliminado = 0",
            [&marca_nombre],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if active_products > 0 {
        return Err("No se puede eliminar la marca porque tiene productos asociados.".to_string());
    }
    conn.execute(
        "UPDATE marcas SET eliminado = 1, sincronizado = 0, updated_at = datetime('now') WHERE id = ?1 AND eliminado = 0",
        [&id],
    )
    .map_err(|error| map_write_error(error, "marca"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn get_categorias(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Vec<Categoria>> {
    require_admin_or_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, TRIM(COALESCE(nombre, '')) FROM categorias WHERE eliminado = 0 ORDER BY TRIM(nombre)")
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let iter = stmt
        .query_map([], |row| Ok(Categoria { id: row.get(0)?, nombre: row.get(1)? }))
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let mut categorias = Vec::new();
    for item in iter {
        categorias.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(categorias)
}

#[tauri::command]
fn create_categoria(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    categoria: Categoria,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let categoria = Categoria { id: categoria.id.trim().to_string(), nombre: normalize_title_trim(&categoria.nombre) };
    validate_categoria(&categoria).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_unique_catalog_name(&conn, "categorias", "categoría", None, &categoria.nombre)
        .map_err(to_command_error)?;
    conn.execute(
        "INSERT INTO categorias (id, nombre, sincronizado, updated_at, eliminado) VALUES (?1, ?2, 0, datetime('now'), 0)",
        params![categoria.id, categoria.nombre],
    )
    .map_err(|error| map_write_error(error, "categoría"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn update_categoria(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
    categoria: Categoria,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let categoria = Categoria { id: id.trim().to_string(), nombre: normalize_title_trim(&categoria.nombre) };
    validate_categoria(&categoria).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_unique_catalog_name(&conn, "categorias", "categoría", Some(&id), &categoria.nombre)
        .map_err(to_command_error)?;
    let affected = conn
        .execute(
            "UPDATE categorias SET nombre = ?1, sincronizado = 0, updated_at = datetime('now') WHERE id = ?2 AND eliminado = 0",
            params![categoria.nombre, id],
        )
        .map_err(|error| map_write_error(error, "categoría"))
        .map_err(to_command_error)?;
    if affected == 0 {
        return Err("No se encontró la categoría que intentas actualizar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn delete_categoria(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let categoria_nombre: String = conn
        .query_row("SELECT nombre FROM categorias WHERE id = ?1 AND eliminado = 0", [&id], |row| row.get(0))
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?
        .ok_or_else(|| "No se encontró la categoría que intentas eliminar.".to_string())?;
    let active_products: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM productos WHERE categoria = ?1 AND eliminado = 0",
            [&categoria_nombre],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if active_products > 0 {
        return Err("No se puede eliminar la categoría porque tiene productos asociados.".to_string());
    }
    conn.execute(
        "UPDATE categorias SET eliminado = 1, sincronizado = 0, updated_at = datetime('now') WHERE id = ?1 AND eliminado = 0",
        [&id],
    )
    .map_err(|error| map_write_error(error, "categoría"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn get_unidades(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Vec<UnidadMedida>> {
    require_admin_or_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, TRIM(COALESCE(nombre, '')), TRIM(COALESCE(clave_sat, '')) FROM unidades WHERE eliminado = 0 ORDER BY TRIM(nombre)")
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let iter = stmt
        .query_map([], |row| {
            Ok(UnidadMedida {
                id: row.get(0)?,
                nombre: row.get(1)?,
                clave_sat: row.get(2)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let mut unidades = Vec::new();
    for item in iter {
        unidades.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(unidades)
}

#[tauri::command]
fn create_unidad(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    unidad: UnidadMedida,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let unidad = UnidadMedida {
        id: unidad.id.trim().to_string(),
        nombre: normalize_title_trim(&unidad.nombre),
        clave_sat: normalize_upper_trim(&unidad.clave_sat),
    };
    validate_unidad(&unidad).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_unique_catalog_name(&conn, "unidades", "unidad", None, &unidad.nombre)
        .map_err(to_command_error)?;
    conn.execute(
        "INSERT INTO unidades (id, nombre, clave_sat, sincronizado, updated_at, eliminado) VALUES (?1, ?2, ?3, 0, datetime('now'), 0)",
        params![unidad.id, unidad.nombre, unidad.clave_sat],
    )
    .map_err(|error| map_write_error(error, "unidad"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn update_unidad(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
    unidad: UnidadMedida,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let unidad = UnidadMedida {
        id: id.trim().to_string(),
        nombre: normalize_title_trim(&unidad.nombre),
        clave_sat: normalize_upper_trim(&unidad.clave_sat),
    };
    validate_unidad(&unidad).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_unique_catalog_name(&conn, "unidades", "unidad", Some(&id), &unidad.nombre)
        .map_err(to_command_error)?;
    let affected = conn
        .execute(
            "
            UPDATE unidades
            SET nombre = ?1,
                clave_sat = ?2,
                sincronizado = 0,
                updated_at = datetime('now')
            WHERE id = ?3 AND eliminado = 0
            ",
            params![unidad.nombre, unidad.clave_sat, id],
        )
        .map_err(|error| map_write_error(error, "unidad"))
        .map_err(to_command_error)?;
    if affected == 0 {
        return Err("No se encontró la unidad que intentas actualizar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn delete_unidad(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let unidad_nombre: String = conn
        .query_row("SELECT nombre FROM unidades WHERE id = ?1 AND eliminado = 0", [&id], |row| row.get(0))
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?
        .ok_or_else(|| "No se encontró la unidad que intentas eliminar.".to_string())?;
    let active_products: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM productos WHERE unidad = ?1 AND eliminado = 0",
            [&unidad_nombre],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if active_products > 0 {
        return Err("No se puede eliminar la unidad porque tiene productos asociados.".to_string());
    }
    conn.execute(
        "UPDATE unidades SET eliminado = 1, sincronizado = 0, updated_at = datetime('now') WHERE id = ?1 AND eliminado = 0",
        [&id],
    )
    .map_err(|error| map_write_error(error, "unidad"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn get_clientes(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Vec<Cliente>> {
    require_admin_or_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, TRIM(COALESCE(nombre, '')), TRIM(COALESCE(telefono, '')), TRIM(COALESCE(direccion, '')), limite_credito, saldo_deudor FROM clientes WHERE eliminado = 0 ORDER BY TRIM(nombre)")
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map([], |row| {
            Ok(Cliente {
                id: row.get(0)?,
                nombre: row.get(1)?,
                telefono: row.get(2)?,
                direccion: row.get(3)?,
                limite_credito: row.get(4)?,
                saldo_deudor: row.get(5)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut clientes = Vec::new();
    for item in iter {
        clientes.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(clientes)
}

#[tauri::command]
fn create_cliente(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    cliente: Cliente,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let cliente = Cliente {
        id: cliente.id.trim().to_string(),
        nombre: normalize_title_trim(&cliente.nombre),
        telefono: normalize_plain_trim(&cliente.telefono),
        direccion: normalize_title_trim(&cliente.direccion),
        limite_credito: normalize_money(cliente.limite_credito, "El límite de crédito", true)
            .map_err(to_command_error)?,
        saldo_deudor: 0.0,
    };
    validate_cliente(&cliente).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.execute(
        "
        INSERT INTO clientes (
            id, nombre, telefono, direccion, limite_credito, saldo_deudor,
            sincronizado, updated_at, eliminado
        ) VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, datetime('now'), 0)
        ",
        params![
            cliente.id,
            cliente.nombre,
            cliente.telefono,
            cliente.direccion,
            cliente.limite_credito
        ],
    )
    .map_err(|error| map_write_error(error, "cliente"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn update_cliente(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
    cliente: Cliente,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let cliente = Cliente {
        id: id.trim().to_string(),
        nombre: normalize_title_trim(&cliente.nombre),
        telefono: normalize_plain_trim(&cliente.telefono),
        direccion: normalize_title_trim(&cliente.direccion),
        limite_credito: normalize_money(cliente.limite_credito, "El límite de crédito", true)
            .map_err(to_command_error)?,
        saldo_deudor: cliente.saldo_deudor,
    };
    validate_cliente(&cliente).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let saldo_actual: f64 = conn
        .query_row(
            "SELECT COALESCE(saldo_deudor, 0) FROM clientes WHERE id = ?1 AND eliminado = 0",
            [&cliente.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?
        .ok_or_else(|| "No se encontró el cliente que intentas actualizar.".to_string())?;
    if cliente.limite_credito > 0.0 && round_money(cliente.limite_credito) < round_money(saldo_actual) {
        return Err(format!(
            "El límite de crédito no puede ser menor al saldo deudor actual (${:.2}).",
            saldo_actual
        ));
    }

    let affected = conn
        .execute(
            "
            UPDATE clientes
            SET nombre = ?1,
                telefono = ?2,
                direccion = ?3,
                limite_credito = ?4,
                sincronizado = 0,
                updated_at = datetime('now')
            WHERE id = ?5
              AND eliminado = 0
              AND (?4 = 0 OR ROUND(?4, 2) >= ROUND(COALESCE(saldo_deudor, 0), 2))
            ",
            params![
                cliente.nombre,
                cliente.telefono,
                cliente.direccion,
                cliente.limite_credito,
                cliente.id
            ],
        )
        .map_err(|error| map_write_error(error, "cliente"))
        .map_err(to_command_error)?;

    if affected == 0 {
        return Err("No se encontró el cliente que intentas actualizar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn delete_cliente(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let saldo: f64 = conn
        .query_row(
            "SELECT COALESCE(saldo_deudor, 0) FROM clientes WHERE id = ?1 AND eliminado = 0",
            [&id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if saldo > 0.0 {
        return Err("No se puede eliminar cliente con saldo deudor pendiente.".to_string());
    }

    let affected = conn
        .execute(
            "UPDATE clientes SET eliminado = 1, sincronizado = 0, updated_at = datetime('now') WHERE id = ?1 AND eliminado = 0",
            [&id],
        )
        .map_err(|error| map_write_error(error, "cliente"))
        .map_err(to_command_error)?;
    if affected == 0 {
        return Err("No se encontró el cliente que intentas eliminar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn get_cliente_datos_fiscales(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    cliente_id: String,
) -> AppResult<Option<ClienteDatosFiscales>> {
    require_admin_or_superadmin(&state_sesion)?;
    if cliente_id.trim().is_empty() {
        return Err("Falta el cliente para consultar datos fiscales.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.query_row(
        "
        SELECT cliente_id, rfc, razon_social, regimen_fiscal, codigo_postal
        FROM clientes_datos_fiscales
        WHERE cliente_id = ?1
        ",
        [&cliente_id],
        |row| {
            Ok(ClienteDatosFiscales {
                cliente_id: row.get(0)?,
                rfc: row.get(1)?,
                razon_social: row.get(2)?,
                regimen_fiscal: row.get(3)?,
                codigo_postal: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(AppError::from)
    .map_err(to_command_error)
}

#[tauri::command]
fn guardar_cliente_datos_fiscales(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    datos: ClienteDatosFiscales,
) -> AppResult<ClienteDatosFiscales> {
    require_admin_or_superadmin(&state_sesion)?;
    let datos = ClienteDatosFiscales {
        cliente_id: datos.cliente_id.trim().to_string(),
        rfc: normalize_upper_trim(&datos.rfc),
        razon_social: datos.razon_social.trim().to_string(),
        regimen_fiscal: datos.regimen_fiscal.trim().to_string(),
        codigo_postal: datos.codigo_postal.trim().to_string(),
    };
    validate_cliente_datos_fiscales(&datos).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;

    let cliente_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM clientes WHERE id = ?1 AND eliminado = 0",
            [&datos.cliente_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if cliente_exists == 0 {
        return Err("El cliente no existe.".to_string());
    }

    conn.execute(
        "
        INSERT INTO clientes_datos_fiscales (
            cliente_id, rfc, razon_social, regimen_fiscal, codigo_postal,
            sincronizado, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, 0, datetime('now'))
        ON CONFLICT(cliente_id) DO UPDATE SET
            rfc = excluded.rfc,
            razon_social = excluded.razon_social,
            regimen_fiscal = excluded.regimen_fiscal,
            codigo_postal = excluded.codigo_postal,
            sincronizado = 0,
            updated_at = datetime('now')
        ",
        params![
            datos.cliente_id,
            datos.rfc,
            datos.razon_social,
            datos.regimen_fiscal,
            datos.codigo_postal
        ],
    )
    .map_err(|error| map_write_error(error, "datos fiscales del cliente"))
    .map_err(to_command_error)?;

    Ok(datos)
}

#[tauri::command]
fn registrar_abono(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    abono: AbonoCreditoInput,
) -> AppResult<()> {
    let actor = current_session_user(&state_sesion)?;
    require_admin_or_superadmin(&state_sesion)?;
    if actor.id != abono.usuario_id {
        return Err("No puedes registrar abonos a nombre de otro usuario.".to_string());
    }

    validate_abono_credito(&abono).map_err(to_command_error)?;
    let monto = normalize_money(abono.monto, "El abono", false).map_err(to_command_error)?;
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let caja_id: String = tx
        .query_row(
            "
            SELECT id
            FROM cajas_sesiones
            WHERE usuario_id = ?1
              AND sucursal_id = ?2
              AND estado = 'ABIERTA'
            ORDER BY fecha_apertura DESC
            LIMIT 1
            ",
            params![&actor.id, &actor.sucursal_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?
        .ok_or_else(|| "No puedes registrar abonos sin una caja ABIERTA.".to_string())?;

    let saldo_actual: f64 = tx
        .query_row(
            "SELECT saldo_deudor FROM clientes WHERE id = ?1 AND eliminado = 0",
            [&abono.cliente_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if monto > round_money(saldo_actual) {
        return Err("El abono no puede ser mayor al saldo deudor actual.".to_string());
    }

    tx.execute(
        "
        INSERT INTO creditos_abonos (
            id, cliente_id, monto, fecha, usuario_id,
            sync_uuid, sincronizado, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, datetime('now'))
        ",
        params![
            abono.id,
            abono.cliente_id,
            monto,
            abono.fecha,
            abono.usuario_id,
            generate_uuid_like()
        ],
    )
    .map_err(|error| map_write_error(error, "abono"))
    .map_err(to_command_error)?;

    let affected = tx.execute(
        "
        UPDATE clientes
        SET saldo_deudor = ROUND(saldo_deudor - ?1, 2),
            sincronizado = 0,
            updated_at = datetime('now')
        WHERE id = ?2
          AND eliminado = 0
          AND saldo_deudor + 0.0001 >= ?1
        ",
        params![monto, abono.cliente_id],
    )
    .map_err(|error| map_write_error(error, "cliente"))
    .map_err(to_command_error)?;
    if affected != 1 {
        return Err("No se pudo aplicar el abono porque el saldo del cliente cambió. Actualiza e intenta de nuevo.".to_string());
    }

    tx.execute(
        "
        INSERT INTO caja_movimientos (id, sesion_id, tipo, monto, motivo, sync_uuid, sincronizado, updated_at)
        VALUES (?1, ?2, 'INGRESO', ?3, ?4, ?5, 0, datetime('now'))
        ",
        params![
            format!("ABONO-{}", abono.id),
            caja_id,
            monto,
            format!("ABONO A CRÉDITO CLIENTE #{}", abono.cliente_id),
            generate_uuid_like()
        ],
    )
    .map_err(|error| map_write_error(error, "movimiento de caja"))
    .map_err(to_command_error)?;

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[derive(Debug)]
struct ActivePromotion {
    id: String,
    nombre: String,
    tipo_descuento: String,
    valor: f64,
}

fn discounted_price(precio: f64, tipo_descuento: &str, valor: f64) -> f64 {
    let result = match tipo_descuento {
        "PORCENTAJE" => precio * (1.0 - (valor / 100.0)),
        "MONTO_FIJO" => precio - valor,
        _ => precio,
    };
    round_money(result.max(0.0))
}

fn precio_base_por_cantidad(product: &ProductoConStock, cantidad: f64) -> (f64, String) {
    let mostrador = round_money(product.precio_venta);
    if product.mayoreo_apartir > 0.0 && cantidad >= product.mayoreo_apartir {
        for (precio, tipo) in [
            (product.precio_1, "MAYOREO_1"),
            (product.precio_2, "MAYOREO_2"),
            (product.precio_3, "MAYOREO_3"),
            (product.precio_4, "MAYOREO_4"),
        ] {
            let precio = round_money(precio);
            if precio > 0.0 && precio < mostrador {
                return (precio, tipo.to_string());
            }
        }
    }
    (mostrador, "MOSTRADOR".to_string())
}

fn find_active_promotion(
    conn: &Connection,
    producto_id: &str,
    categoria: &str,
    marca: &str,
    sucursal_id: &str,
    precio_venta: f64,
    costo_unitario: f64,
) -> Result<Option<ActivePromotion>, AppError> {
    let mut stmt = conn.prepare(
        "
        SELECT pr.id, pr.nombre, pr.tipo_descuento, pr.valor
        FROM promociones pr
        INNER JOIN promocion_sucursales ps ON ps.promocion_id = pr.id
        WHERE ps.sucursal_id = ?1
          AND ps.eliminado = 0
          AND pr.activo = 1
          AND pr.eliminado = 0
          AND datetime('now') >= datetime(pr.fecha_inicio)
          AND datetime('now') <= datetime(pr.fecha_fin)
          AND (
            pr.producto_id = ?2
            OR (
              pr.producto_id IS NULL
              AND COALESCE(NULLIF(pr.categoria_id, ''), '') = ?3
            )
            OR (
              pr.producto_id IS NULL
              AND pr.categoria_id IS NULL
              AND COALESCE(NULLIF(pr.marca, ''), '') = ?4
            )
          )
        ORDER BY pr.updated_at DESC
        ",
    )?;

    let rows = stmt.query_map(params![sucursal_id, producto_id, categoria, marca], |row| {
        Ok(ActivePromotion {
            id: row.get(0)?,
            nombre: row.get(1)?,
            tipo_descuento: row.get(2)?,
            valor: row.get(3)?,
        })
    })?;

    let mut best: Option<ActivePromotion> = None;
    let mut best_price = f64::MAX;
    for item in rows {
        let promo = item?;
        let price = discounted_price(precio_venta, &promo.tipo_descuento, promo.valor);
        if price + 0.0001 < costo_unitario {
            continue;
        }
        if price < best_price {
            best_price = price;
            best = Some(promo);
        }
    }
    Ok(best)
}

fn apply_active_promotion(conn: &Connection, product: &mut ProductoConStock) -> Result<(), AppError> {
    let Some(promo) = find_active_promotion(
        conn,
        &product.id,
        &product.categoria,
        &product.marca,
        &product.sucursal_id,
        product.precio_venta,
        product.costo_promedio,
    )? else {
        return Ok(());
    };

    let original = round_money(product.precio_venta);
    let discounted = discounted_price(original, &promo.tipo_descuento, promo.valor);
    if discounted < original && discounted + 0.0001 >= product.costo_promedio {
        product.precio_original = Some(original);
        product.precio_descontado = Some(discounted);
        product.nombre_promo = Some(promo.nombre);
        product.promocion_id = Some(promo.id);
        product.promo_tipo_descuento = Some(promo.tipo_descuento);
        product.promo_valor = Some(promo.valor);
        product.precio_venta = discounted;
    }
    Ok(())
}

#[tauri::command]
fn get_productos_por_sucursal(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    sucursal_id: String,
) -> AppResult<Vec<ProductoConStock>> {
    let actor = current_session_user(&state_sesion)?;
    ensure_can_read_sucursal(&actor, &sucursal_id)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare(
            "
            SELECT
                p.id,
                TRIM(COALESCE(p.codigo_barras, '')),
                TRIM(COALESCE(p.codigo_proveedor, '')),
                TRIM(COALESCE(p.proveedor_id, '')),
                TRIM(COALESCE(p.clave_producto, '')),
                TRIM(COALESCE(p.descripcion, '')),
                TRIM(COALESCE(p.marca, '')),
                TRIM(COALESCE(p.categoria, '')),
                TRIM(COALESCE(p.unidad, '')),
                COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS precio_costo_local,
                COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS costo_promedio,
                COALESCE(NULLIF(i.precio_venta, 0), p.precio_venta) AS precio_venta_local,
                TRIM(COALESCE(p.sat_clave_prod_serv, '')),
                TRIM(COALESCE(p.sat_clave_unidad, '')),
                i.sucursal_id,
                i.stock,
                i.stock_minimo,
                COALESCE(p.precio_1, 0),
                COALESCE(p.precio_2, 0),
                COALESCE(p.precio_3, 0),
                COALESCE(p.precio_4, 0),
                COALESCE(p.mayoreo_apartir, 0),
                COALESCE(p.a_granel, 0),
                COALESCE(p.no_en_catalogo, 0),
                COALESCE(p.ventas_negativas, 0),
                p.caducidad,
                TRIM(COALESCE(p.fotos, '')),
                TRIM(COALESCE(p.descripcion_catalogo, ''))
            FROM productos p
            INNER JOIN inventario_sucursal i ON i.producto_id = p.id
            WHERE i.sucursal_id = ?1
              AND p.eliminado = 0
              AND i.eliminado = 0
            ORDER BY p.descripcion
            ",
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map([sucursal_id], |row| {
            Ok(ProductoConStock {
                id: row.get(0)?,
                codigo_barras: row.get(1)?,
                codigo_proveedor: row.get(2)?,
                proveedor_id: row.get(3)?,
                clave_producto: row.get(4)?,
                descripcion: row.get(5)?,
                marca: row.get(6)?,
                categoria: row.get(7)?,
                unidad: row.get(8)?,
                precio_costo: row.get(9)?,
                costo_promedio: row.get(10)?,
                precio_venta: row.get(11)?,
                sat_clave_prod_serv: row.get(12)?,
                sat_clave_unidad: row.get(13)?,
                sucursal_id: row.get(14)?,
                stock: row.get(15)?,
                stock_minimo: row.get(16)?,
                precio_original: None,
                precio_descontado: None,
                nombre_promo: None,
                promocion_id: None,
                promo_tipo_descuento: None,
                promo_valor: None,
                precio_1: row.get(17)?,
                precio_2: row.get(18)?,
                precio_3: row.get(19)?,
                precio_4: row.get(20)?,
                mayoreo_apartir: row.get(21)?,
                a_granel: row.get::<_, i64>(22)? == 1,
                no_en_catalogo: row.get::<_, i64>(23)? == 1,
                ventas_negativas: row.get::<_, i64>(24)? == 1,
                caducidad: row.get(25)?,
                fotos: row.get(26)?,
                descripcion_catalogo: row.get(27)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut productos = Vec::new();
    for item in iter {
        let mut product = item.map_err(AppError::from).map_err(to_command_error)?;
        apply_active_promotion(&conn, &mut product).map_err(to_command_error)?;
        productos.push(product);
    }

    Ok(productos)
}

#[tauri::command]
fn buscar_productos_para_compra(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    sucursal_id: String,
    query: String,
) -> AppResult<Vec<ProductoConStock>> {
    let actor = require_admin_or_superadmin(&state_sesion)?;
    ensure_can_read_sucursal(&actor, &sucursal_id)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let exact_query = query.trim().to_string();
    if exact_query.is_empty() {
        return Ok(Vec::new());
    }
    let prefix_pattern = format!("{}%", exact_query);
    let contains_pattern = format!("%{}%", exact_query);

    let mut stmt = conn
        .prepare(
            "
            SELECT
                p.id,
                TRIM(COALESCE(p.codigo_barras, '')),
                TRIM(COALESCE(p.codigo_proveedor, '')),
                TRIM(COALESCE(p.proveedor_id, '')),
                TRIM(COALESCE(p.clave_producto, '')),
                TRIM(COALESCE(p.descripcion, '')),
                TRIM(COALESCE(p.marca, '')),
                TRIM(COALESCE(p.categoria, '')),
                TRIM(COALESCE(p.unidad, '')),
                COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS precio_costo_local,
                COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS costo_promedio,
                COALESCE(NULLIF(i.precio_venta, 0), p.precio_venta) AS precio_venta_local,
                TRIM(COALESCE(p.sat_clave_prod_serv, '')),
                TRIM(COALESCE(p.sat_clave_unidad, '')),
                COALESCE(i.sucursal_id, ?1) AS sucursal_id,
                COALESCE(i.stock, 0) AS stock,
                COALESCE(i.stock_minimo, 0) AS stock_minimo
            FROM productos p
            LEFT JOIN inventario_sucursal i ON i.producto_id = p.id AND i.sucursal_id = ?1 AND i.eliminado = 0
            WHERE p.eliminado = 0
              AND (
                p.codigo_barras = ?2
                OR p.id = ?2
                OR p.codigo_proveedor = ?2
                OR p.clave_producto = ?2
                OR p.codigo_barras LIKE ?3 COLLATE NOCASE
                OR p.id LIKE ?3 COLLATE NOCASE
                OR p.codigo_proveedor LIKE ?3 COLLATE NOCASE
                OR p.clave_producto LIKE ?3 COLLATE NOCASE
                OR p.descripcion LIKE ?3 COLLATE NOCASE
                OR p.marca LIKE ?3 COLLATE NOCASE
                OR p.descripcion LIKE ?4 COLLATE NOCASE
              )
            ORDER BY
                CASE
                    WHEN p.codigo_barras = ?2 THEN 0
                    WHEN p.id = ?2 THEN 1
                    WHEN p.codigo_proveedor = ?2 THEN 2
                    WHEN p.clave_producto = ?2 THEN 3
                    WHEN p.codigo_barras LIKE ?3 COLLATE NOCASE THEN 4
                    WHEN p.id LIKE ?3 COLLATE NOCASE THEN 5
                    WHEN p.codigo_proveedor LIKE ?3 COLLATE NOCASE THEN 6
                    WHEN p.clave_producto LIKE ?3 COLLATE NOCASE THEN 7
                    WHEN p.descripcion LIKE ?3 COLLATE NOCASE THEN 8
                    WHEN p.marca LIKE ?3 COLLATE NOCASE THEN 9
                    ELSE 10
                END,
                p.descripcion COLLATE NOCASE
            LIMIT 10
            ",
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map(params![sucursal_id, exact_query, prefix_pattern, contains_pattern], |row| {
            Ok(ProductoConStock {
                id: row.get(0)?,
                codigo_barras: row.get(1)?,
                codigo_proveedor: row.get(2)?,
                proveedor_id: row.get(3)?,
                clave_producto: row.get(4)?,
                descripcion: row.get(5)?,
                marca: row.get(6)?,
                categoria: row.get(7)?,
                unidad: row.get(8)?,
                precio_costo: row.get(9)?,
                costo_promedio: row.get(10)?,
                precio_venta: row.get(11)?,
                sat_clave_prod_serv: row.get(12)?,
                sat_clave_unidad: row.get(13)?,
                sucursal_id: row.get(14)?,
                stock: row.get(15)?,
                stock_minimo: row.get(16)?,
                precio_original: None,
                precio_descontado: None,
                nombre_promo: None,
                promocion_id: None,
                promo_tipo_descuento: None,
                promo_valor: None,
                precio_1: 0.0,
                precio_2: 0.0,
                precio_3: 0.0,
                precio_4: 0.0,
                mayoreo_apartir: 0.0,
                a_granel: false,
                no_en_catalogo: false,
                ventas_negativas: false,
                caducidad: None,
                fotos: String::new(),
                descripcion_catalogo: String::new(),
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut productos = Vec::new();
    for item in iter {
        productos.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }

    Ok(productos)
}

#[tauri::command]
fn buscar_productos_por_sucursal(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    sucursal_id: String,
    query: String,
) -> AppResult<Vec<ProductoConStock>> {
    let actor = current_session_user(&state_sesion)?;
    ensure_can_read_sucursal(&actor, &sucursal_id)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let exact_query = query.trim().to_string();
    if exact_query.is_empty() {
        return Ok(Vec::new());
    }
    let prefix_pattern = format!("{}%", exact_query);
    let contains_pattern = format!("%{}%", exact_query);

    let mut stmt = conn
        .prepare(
            "
            SELECT
                p.id,
                TRIM(COALESCE(p.codigo_barras, '')),
                TRIM(COALESCE(p.codigo_proveedor, '')),
                TRIM(COALESCE(p.proveedor_id, '')),
                TRIM(COALESCE(p.clave_producto, '')),
                TRIM(COALESCE(p.descripcion, '')),
                TRIM(COALESCE(p.marca, '')),
                TRIM(COALESCE(p.categoria, '')),
                TRIM(COALESCE(p.unidad, '')),
                COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS precio_costo_local,
                COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS costo_promedio,
                COALESCE(NULLIF(i.precio_venta, 0), p.precio_venta) AS precio_venta_local,
                TRIM(COALESCE(p.sat_clave_prod_serv, '')),
                TRIM(COALESCE(p.sat_clave_unidad, '')),
                i.sucursal_id,
                i.stock,
                i.stock_minimo,
                COALESCE(p.precio_1, 0),
                COALESCE(p.precio_2, 0),
                COALESCE(p.precio_3, 0),
                COALESCE(p.precio_4, 0),
                COALESCE(p.mayoreo_apartir, 0),
                COALESCE(p.a_granel, 0),
                COALESCE(p.no_en_catalogo, 0),
                COALESCE(p.ventas_negativas, 0),
                p.caducidad,
                TRIM(COALESCE(p.fotos, '')),
                TRIM(COALESCE(p.descripcion_catalogo, ''))
            FROM productos p
            INNER JOIN inventario_sucursal i ON i.producto_id = p.id
            WHERE i.sucursal_id = ?1
              AND p.eliminado = 0
              AND i.eliminado = 0
              AND (
                p.codigo_barras = ?2
                OR p.codigo_proveedor = ?2
                OR p.clave_producto = ?2
                OR p.codigo_barras LIKE ?3 COLLATE NOCASE
                OR p.codigo_proveedor LIKE ?3 COLLATE NOCASE
                OR p.clave_producto LIKE ?3 COLLATE NOCASE
                OR p.descripcion LIKE ?3 COLLATE NOCASE
                OR p.marca LIKE ?3 COLLATE NOCASE
                OR p.descripcion LIKE ?4 COLLATE NOCASE
              )
            ORDER BY
                CASE
                    WHEN p.codigo_barras = ?2 THEN 0
                    WHEN p.codigo_proveedor = ?2 THEN 1
                    WHEN p.clave_producto = ?2 THEN 2
                    WHEN p.codigo_barras LIKE ?3 COLLATE NOCASE THEN 3
                    WHEN p.codigo_proveedor LIKE ?3 COLLATE NOCASE THEN 4
                    WHEN p.clave_producto LIKE ?3 COLLATE NOCASE THEN 5
                    WHEN p.descripcion LIKE ?3 COLLATE NOCASE THEN 6
                    WHEN p.marca LIKE ?3 COLLATE NOCASE THEN 7
                    ELSE 8
                END,
                p.descripcion COLLATE NOCASE
            LIMIT 10
            ",
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map(params![sucursal_id, exact_query, prefix_pattern, contains_pattern], |row| {
            Ok(ProductoConStock {
                id: row.get(0)?,
                codigo_barras: row.get(1)?,
                codigo_proveedor: row.get(2)?,
                proveedor_id: row.get(3)?,
                clave_producto: row.get(4)?,
                descripcion: row.get(5)?,
                marca: row.get(6)?,
                categoria: row.get(7)?,
                unidad: row.get(8)?,
                precio_costo: row.get(9)?,
                costo_promedio: row.get(10)?,
                precio_venta: row.get(11)?,
                sat_clave_prod_serv: row.get(12)?,
                sat_clave_unidad: row.get(13)?,
                sucursal_id: row.get(14)?,
                stock: row.get(15)?,
                stock_minimo: row.get(16)?,
                precio_original: None,
                precio_descontado: None,
                nombre_promo: None,
                promocion_id: None,
                promo_tipo_descuento: None,
                promo_valor: None,
                precio_1: row.get(17)?,
                precio_2: row.get(18)?,
                precio_3: row.get(19)?,
                precio_4: row.get(20)?,
                mayoreo_apartir: row.get(21)?,
                a_granel: row.get::<_, i64>(22)? == 1,
                no_en_catalogo: row.get::<_, i64>(23)? == 1,
                ventas_negativas: row.get::<_, i64>(24)? == 1,
                caducidad: row.get(25)?,
                fotos: row.get(26)?,
                descripcion_catalogo: row.get(27)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut productos = Vec::new();
    for item in iter {
        let mut product = item.map_err(AppError::from).map_err(to_command_error)?;
        apply_active_promotion(&conn, &mut product).map_err(to_command_error)?;
        productos.push(product);
    }

    Ok(productos)
}

#[tauri::command]
fn get_productos_por_sucursal_page(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    sucursal_id: String,
    query: String,
    page: i64,
    page_size: i64,
) -> AppResult<ProductoInventarioPage> {
    let actor = current_session_user(&state_sesion)?;
    ensure_can_read_sucursal(&actor, &sucursal_id)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let (page, page_size) = normalize_page_args(page, page_size);
    let offset = page * page_size;
    let exact_query = query.trim().to_string();
    let prefix_pattern = format!("{}%", exact_query);
    let contains_pattern = format!("%{}%", exact_query);

    let total: i64 = if exact_query.is_empty() {
        conn.query_row(
            "
            SELECT COUNT(*)
            FROM productos p
            INNER JOIN inventario_sucursal i ON i.producto_id = p.id
            WHERE i.sucursal_id = ?1
              AND p.eliminado = 0
              AND i.eliminado = 0
            ",
            [&sucursal_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?
    } else {
        conn.query_row(
            "
            SELECT COUNT(*)
            FROM productos p
            INNER JOIN inventario_sucursal i ON i.producto_id = p.id
            WHERE i.sucursal_id = ?1
              AND p.eliminado = 0
              AND i.eliminado = 0
              AND (
                p.codigo_barras = ?2
                OR p.id = ?2
                OR p.codigo_proveedor = ?2
                OR p.clave_producto = ?2
                OR p.codigo_barras LIKE ?3 COLLATE NOCASE
                OR p.id LIKE ?3 COLLATE NOCASE
                OR p.codigo_proveedor LIKE ?3 COLLATE NOCASE
                OR p.clave_producto LIKE ?3 COLLATE NOCASE
                OR p.descripcion LIKE ?3 COLLATE NOCASE
                OR p.marca LIKE ?3 COLLATE NOCASE
                OR p.categoria LIKE ?3 COLLATE NOCASE
                OR p.unidad LIKE ?3 COLLATE NOCASE
                OR p.descripcion LIKE ?4 COLLATE NOCASE
              )
            ",
            params![sucursal_id, exact_query, prefix_pattern, contains_pattern],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?
    };

    let sql = if exact_query.is_empty() {
        "
        SELECT
            p.id,
            TRIM(COALESCE(p.codigo_barras, '')),
            TRIM(COALESCE(p.codigo_proveedor, '')),
            TRIM(COALESCE(p.proveedor_id, '')),
            TRIM(COALESCE(p.clave_producto, '')),
            TRIM(COALESCE(p.descripcion, '')),
            TRIM(COALESCE(p.marca, '')),
            TRIM(COALESCE(p.categoria, '')),
            TRIM(COALESCE(p.unidad, '')),
            COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS precio_costo_local,
            COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS costo_promedio,
            COALESCE(NULLIF(i.precio_venta, 0), p.precio_venta) AS precio_venta_local,
            TRIM(COALESCE(p.sat_clave_prod_serv, '')),
            TRIM(COALESCE(p.sat_clave_unidad, '')),
            i.sucursal_id,
            i.stock,
            i.stock_minimo
        FROM productos p
        INNER JOIN inventario_sucursal i ON i.producto_id = p.id
        WHERE i.sucursal_id = ?1
          AND p.eliminado = 0
          AND i.eliminado = 0
        ORDER BY p.descripcion COLLATE NOCASE
        LIMIT ?2 OFFSET ?3
        "
    } else {
        "
        SELECT
            p.id,
            TRIM(COALESCE(p.codigo_barras, '')),
            TRIM(COALESCE(p.codigo_proveedor, '')),
            TRIM(COALESCE(p.proveedor_id, '')),
            TRIM(COALESCE(p.clave_producto, '')),
            TRIM(COALESCE(p.descripcion, '')),
            TRIM(COALESCE(p.marca, '')),
            TRIM(COALESCE(p.categoria, '')),
            TRIM(COALESCE(p.unidad, '')),
            COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS precio_costo_local,
            COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS costo_promedio,
            COALESCE(NULLIF(i.precio_venta, 0), p.precio_venta) AS precio_venta_local,
            TRIM(COALESCE(p.sat_clave_prod_serv, '')),
            TRIM(COALESCE(p.sat_clave_unidad, '')),
            i.sucursal_id,
            i.stock,
            i.stock_minimo
        FROM productos p
        INNER JOIN inventario_sucursal i ON i.producto_id = p.id
        WHERE i.sucursal_id = ?1
          AND p.eliminado = 0
          AND i.eliminado = 0
          AND (
            p.codigo_barras = ?2
            OR p.codigo_proveedor = ?2
            OR p.clave_producto = ?2
            OR p.codigo_barras LIKE ?3 COLLATE NOCASE
            OR p.codigo_proveedor LIKE ?3 COLLATE NOCASE
            OR p.clave_producto LIKE ?3 COLLATE NOCASE
            OR p.descripcion LIKE ?3 COLLATE NOCASE
            OR p.marca LIKE ?3 COLLATE NOCASE
            OR p.categoria LIKE ?3 COLLATE NOCASE
            OR p.unidad LIKE ?3 COLLATE NOCASE
            OR p.descripcion LIKE ?4 COLLATE NOCASE
          )
        ORDER BY
            CASE
                    WHEN p.codigo_barras = ?2 THEN 0
                    WHEN p.id = ?2 THEN 1
                    WHEN p.codigo_proveedor = ?2 THEN 2
                    WHEN p.clave_producto = ?2 THEN 3
                    WHEN p.codigo_barras LIKE ?3 COLLATE NOCASE THEN 4
                    WHEN p.id LIKE ?3 COLLATE NOCASE THEN 5
                    WHEN p.codigo_proveedor LIKE ?3 COLLATE NOCASE THEN 6
                    WHEN p.clave_producto LIKE ?3 COLLATE NOCASE THEN 7
                    WHEN p.descripcion LIKE ?3 COLLATE NOCASE THEN 8
                    WHEN p.marca LIKE ?3 COLLATE NOCASE THEN 9
                    ELSE 10
            END,
            p.descripcion COLLATE NOCASE
        LIMIT ?5 OFFSET ?6
        "
    };

    let mut stmt = conn.prepare(sql).map_err(AppError::from).map_err(to_command_error)?;
    let mapper = |row: &rusqlite::Row<'_>| {
        Ok(ProductoConStock {
            id: row.get(0)?,
            codigo_barras: row.get(1)?,
            codigo_proveedor: row.get(2)?,
            proveedor_id: row.get(3)?,
            clave_producto: row.get(4)?,
            descripcion: row.get(5)?,
            marca: row.get(6)?,
            categoria: row.get(7)?,
            unidad: row.get(8)?,
            precio_costo: row.get(9)?,
            costo_promedio: row.get(10)?,
            precio_venta: row.get(11)?,
            sat_clave_prod_serv: row.get(12)?,
            sat_clave_unidad: row.get(13)?,
            sucursal_id: row.get(14)?,
            stock: row.get(15)?,
            stock_minimo: row.get(16)?,
            precio_original: None,
            precio_descontado: None,
            nombre_promo: None,
            promocion_id: None,
            promo_tipo_descuento: None,
            promo_valor: None,
            precio_1: 0.0,
            precio_2: 0.0,
            precio_3: 0.0,
            precio_4: 0.0,
            mayoreo_apartir: 0.0,
            a_granel: false,
            no_en_catalogo: false,
            ventas_negativas: false,
            caducidad: None,
            fotos: String::new(),
            descripcion_catalogo: String::new(),
        })
    };
    let iter = if exact_query.is_empty() {
        stmt.query_map(params![sucursal_id, page_size, offset], mapper)
    } else {
        stmt.query_map(
            params![sucursal_id, exact_query, prefix_pattern, contains_pattern, page_size, offset],
            mapper,
        )
    }
    .map_err(AppError::from)
    .map_err(to_command_error)?;

    let mut rows = Vec::new();
    for item in iter {
        let mut product = item.map_err(AppError::from).map_err(to_command_error)?;
        apply_active_promotion(&conn, &mut product).map_err(to_command_error)?;
        rows.push(product);
    }

    Ok(ProductoInventarioPage { rows, total })
}

#[tauri::command]
fn asegurar_producto_venta_diversa(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    sucursal_id: String,
) -> AppResult<ProductoConStock> {
    let user = require_admin_or_superadmin(&state_sesion)?;
    if !is_superadmin(&user) && user.sucursal_id != sucursal_id {
        return Err("No puedes crear venta rápida para otra sucursal.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.execute(
        "
        INSERT OR IGNORE INTO proveedores (
            id, nombre, contacto_nombre, telefono, email, direccion, eliminado, sincronizado, updated_at
        ) VALUES (
            'PROVEEDOR-VENTA-DIVERSA', 'Proveedor interno venta diversa', '', '', '', '',
            0, 0, datetime('now')
        )
        ",
        [],
    )
    .map_err(|error| map_write_error(error, "proveedor interno"))
    .map_err(to_command_error)?;

    conn.execute(
        "
        INSERT INTO productos (
            id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
            marca, categoria, unidad, precio_costo, costo_promedio, precio_venta,
            sat_clave_prod_serv, sat_clave_unidad, eliminado, sincronizado, updated_at
        ) VALUES (
            'VENTA-DIVERSA', 'VENTA-DIVERSA', 'VENTA-DIVERSA', 'PROVEEDOR-VENTA-DIVERSA',
            'VENTA_DIVERSA', 'Artículo diverso', 'Sin marca', 'Venta rápida', 'Pieza',
            0, 0, 0, '01010101', 'H87', 0, 0, datetime('now')
        )
        ON CONFLICT(id) DO UPDATE SET
            eliminado = 0,
            sincronizado = 0,
            updated_at = datetime('now')
        ",
        [],
    )
    .map_err(|error| map_write_error(error, "producto de venta diversa"))
    .map_err(to_command_error)?;

    conn.execute(
        "
        INSERT INTO inventario_sucursal (
            producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta,
            sincronizado, updated_at
        ) VALUES (
            'VENTA-DIVERSA', ?1, 1000000, 0, 0, 0, 0, datetime('now')
        )
        ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
            stock = CASE WHEN inventario_sucursal.stock < 100000 THEN 1000000 ELSE inventario_sucursal.stock END,
            sincronizado = 0,
            updated_at = datetime('now')
        ",
        [&sucursal_id],
    )
    .map_err(|error| map_write_error(error, "inventario de venta diversa"))
    .map_err(to_command_error)?;

    Ok(ProductoConStock {
        id: "VENTA-DIVERSA".to_string(),
        codigo_barras: "VENTA-DIVERSA".to_string(),
        codigo_proveedor: "VENTA-DIVERSA".to_string(),
        proveedor_id: "PROVEEDOR-VENTA-DIVERSA".to_string(),
        clave_producto: "VENTA_DIVERSA".to_string(),
        descripcion: "Artículo diverso".to_string(),
        marca: "Sin marca".to_string(),
        categoria: "Venta rápida".to_string(),
        unidad: "Pieza".to_string(),
        precio_costo: 0.0,
        costo_promedio: 0.0,
        precio_venta: 0.0,
        sat_clave_prod_serv: "01010101".to_string(),
        sat_clave_unidad: "H87".to_string(),
        sucursal_id,
        stock: 1000000.0,
        stock_minimo: 0.0,
        precio_original: None,
        precio_descontado: None,
        nombre_promo: None,
        promocion_id: None,
        promo_tipo_descuento: None,
        promo_valor: None,
        precio_1: 0.0,
        precio_2: 0.0,
        precio_3: 0.0,
        precio_4: 0.0,
        mayoreo_apartir: 0.0,
        a_granel: true,
        no_en_catalogo: false,
        ventas_negativas: true,
        caducidad: None,
        fotos: String::new(),
        descripcion_catalogo: String::new(),
    })
}

fn normalize_promocion_input(input: PromocionInput) -> PromocionInput {
    PromocionInput {
        id: input.id.trim().to_string(),
        nombre: input.nombre.trim().to_string(),
        tipo_descuento: input.tipo_descuento.trim().to_uppercase(),
        valor: input.valor,
        fecha_inicio: input.fecha_inicio.trim().to_string(),
        fecha_fin: input.fecha_fin.trim().to_string(),
        activo: input.activo,
        producto_id: input
            .producto_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        categoria_id: input
            .categoria_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        marca: input
            .marca
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        sucursal_ids: input
            .sucursal_ids
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect(),
    }
}

fn validate_promocion_input(input: &PromocionInput) -> AppResult<()> {
    if input.id.is_empty() || input.nombre.is_empty() {
        return Err("La promoción necesita identificador y nombre.".to_string());
    }
    if !matches!(input.tipo_descuento.as_str(), "PORCENTAJE" | "MONTO_FIJO") {
        return Err("El tipo de descuento debe ser PORCENTAJE o MONTO_FIJO.".to_string());
    }
    if !input.valor.is_finite() || input.valor <= 0.0 {
        return Err("El valor del descuento debe ser mayor que cero.".to_string());
    }
    if input.tipo_descuento == "PORCENTAJE" && input.valor > 100.0 {
        return Err("El porcentaje de descuento no puede ser mayor a 100%.".to_string());
    }
    if input.fecha_inicio.is_empty() || input.fecha_fin.is_empty() {
        return Err("La promoción necesita fecha de inicio y fecha fin.".to_string());
    }
    if !input.fecha_inicio.contains('T') || !input.fecha_fin.contains('T') {
        return Err("Las fechas de la promoción deben incluir fecha y hora.".to_string());
    }
    if input.fecha_fin <= input.fecha_inicio {
        return Err("La fecha fin de la promoción debe ser posterior a la fecha de inicio.".to_string());
    }
    
    let has_producto = input.producto_id.is_some();
    let has_categoria = input.categoria_id.is_some();
    let has_marca = input.marca.is_some();
    
    if !has_producto && !has_categoria && !has_marca {
        return Err("Selecciona un producto, una categoría o una marca para la promoción.".to_string());
    }
    
    let active_targets = [has_producto, has_categoria, has_marca].iter().filter(|&&b| b).count();
    if active_targets > 1 {
        return Err("La promoción debe aplicar a producto, categoría o marca, pero solo a uno a la vez.".to_string());
    }

    if input.sucursal_ids.is_empty() {
        return Err("Selecciona al menos una sucursal para la promoción.".to_string());
    }
    Ok(())
}

fn scoped_promocion_sucursales(actor: &Usuario, requested: &[String]) -> Vec<String> {
    if is_superadmin(actor) {
        requested.to_vec()
    } else {
        vec![actor.sucursal_id.clone()]
    }
}

fn ensure_sucursales_exist(conn: &Connection, sucursal_ids: &[String]) -> AppResult<()> {
    for sucursal_id in sucursal_ids {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sucursales WHERE id = ?1 AND eliminado = 0",
                [sucursal_id],
                |row| row.get(0),
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        if exists == 0 {
            return Err(format!("La sucursal {sucursal_id} no existe o está eliminada."));
        }
    }
    Ok(())
}

fn validate_promocion_no_loss(
    conn: &Connection,
    input: &PromocionInput,
    sucursal_ids: &[String],
) -> AppResult<()> {
    let placeholders = vec!["?"; sucursal_ids.len()].join(", ");
    let target_filter = if input.producto_id.is_some() {
        "p.id = ?"
    } else if input.categoria_id.is_some() {
        "p.categoria = ?"
    } else {
        "p.marca = ?"
    };
    let sql = format!(
        "
        SELECT p.descripcion, s.nombre,
               COALESCE(NULLIF(i.precio_venta, 0), p.precio_venta) AS precio,
               COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS costo
        FROM productos p
        INNER JOIN inventario_sucursal i ON i.producto_id = p.id
        INNER JOIN sucursales s ON s.id = i.sucursal_id
        WHERE p.eliminado = 0
          AND i.eliminado = 0
          AND i.sucursal_id IN ({placeholders})
          AND {target_filter}
        "
    );
    let mut values: Vec<Value> = sucursal_ids.iter().cloned().map(Value::Text).collect();
    values.push(Value::Text(
        input
            .producto_id
            .clone()
            .or_else(|| input.categoria_id.clone())
            .or_else(|| input.marca.clone())
            .unwrap_or_default(),
    ));

    let mut stmt = conn.prepare(&sql).map_err(AppError::from).map_err(to_command_error)?;
    let mut rows = stmt
        .query(params_from_iter(values.iter()))
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let mut checked = 0;
    while let Some(row) = rows.next().map_err(AppError::from).map_err(to_command_error)? {
        checked += 1;
        let descripcion: String = row.get(0).map_err(AppError::from).map_err(to_command_error)?;
        let sucursal: String = row.get(1).map_err(AppError::from).map_err(to_command_error)?;
        let precio: f64 = row.get(2).map_err(AppError::from).map_err(to_command_error)?;
        let costo: f64 = row.get(3).map_err(AppError::from).map_err(to_command_error)?;
        let precio_final = discounted_price(precio, &input.tipo_descuento, input.valor);
        if precio_final + 0.0001 < costo {
            return Err(format!(
                "Promoción rechazada: '{}' en '{}' quedaría en ${:.2}, por debajo de su costo ${:.2}.",
                descripcion, sucursal, precio_final, costo
            ));
        }
    }

    if checked == 0 {
        return Err("La promoción no coincide con productos activos en las sucursales seleccionadas.".to_string());
    }
    Ok(())
}

fn get_promocion_sucursal_ids(conn: &Connection, promocion_id: &str) -> AppResult<Vec<String>> {
    let mut stmt = conn
        .prepare("SELECT sucursal_id FROM promocion_sucursales WHERE promocion_id = ?1 AND eliminado = 0 ORDER BY sucursal_id")
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let iter = stmt
        .query_map([promocion_id], |row| row.get::<_, String>(0))
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let mut ids = Vec::new();
    for item in iter {
        ids.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(ids)
}

#[tauri::command]
fn get_promociones(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Vec<Promocion>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let actor = require_admin_or_superadmin(&state_sesion)?;
    let mut sql = String::from(
        "
        SELECT DISTINCT pr.id, pr.nombre, pr.tipo_descuento, pr.valor, pr.fecha_inicio, pr.fecha_fin,
               pr.activo, pr.producto_id, pr.categoria_id, pr.marca, pr.eliminado, pr.updated_at
        FROM promociones pr
        INNER JOIN promocion_sucursales ps ON ps.promocion_id = pr.id
        WHERE pr.eliminado = 0
          AND ps.eliminado = 0
        ",
    );
    let mut params_vec: Vec<String> = Vec::new();
    if !is_superadmin(&actor) {
        sql.push_str(" AND ps.sucursal_id = ?1");
        params_vec.push(actor.sucursal_id.clone());
    }
    sql.push_str(" ORDER BY pr.fecha_inicio DESC, pr.nombre");

    let mut stmt = conn.prepare(&sql).map_err(AppError::from).map_err(to_command_error)?;
    let iter = stmt
        .query_map(params_from_iter(params_vec.iter()), |row| {
            let activo_int: i64 = row.get(6)?;
            let eliminado_int: i64 = row.get(10)?;
            Ok(Promocion {
                id: row.get(0)?,
                nombre: row.get(1)?,
                tipo_descuento: row.get(2)?,
                valor: row.get(3)?,
                fecha_inicio: row.get(4)?,
                fecha_fin: row.get(5)?,
                activo: activo_int == 1,
                producto_id: row.get(7)?,
                categoria_id: row.get(8)?,
                marca: row.get(9)?,
                eliminado: eliminado_int == 1,
                updated_at: row.get(11)?,
                sucursal_ids: Vec::new(),
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut promociones = Vec::new();
    for item in iter {
        let mut promo = item.map_err(AppError::from).map_err(to_command_error)?;
        promo.sucursal_ids = get_promocion_sucursal_ids(&conn, &promo.id)?;
        promociones.push(promo);
    }
    Ok(promociones)
}

#[tauri::command]
fn get_productos_para_promociones(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    sucursal_ids: Vec<String>,
) -> AppResult<Vec<ProductoPromocionPrecio>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let actor = require_admin_or_superadmin(&state_sesion)?;
    let scoped_ids = if is_superadmin(&actor) {
        if sucursal_ids.is_empty() {
            let mut stmt = conn
                .prepare("SELECT id FROM sucursales WHERE eliminado = 0 ORDER BY nombre")
                .map_err(AppError::from)
                .map_err(to_command_error)?;
            let iter = stmt
                .query_map([], |row| row.get::<_, String>(0))
                .map_err(AppError::from)
                .map_err(to_command_error)?;
            let mut ids = Vec::new();
            for item in iter {
                ids.push(item.map_err(AppError::from).map_err(to_command_error)?);
            }
            ids
        } else {
            sucursal_ids
        }
    } else {
        vec![actor.sucursal_id]
    };

    if scoped_ids.is_empty() {
        return Ok(Vec::new());
    }
    ensure_sucursales_exist(&conn, &scoped_ids)?;

    let placeholders = vec!["?"; scoped_ids.len()].join(", ");
    let sql = format!(
        "
        SELECT
            p.id,
            p.codigo_barras,
            p.codigo_proveedor,
            p.clave_producto,
            p.descripcion,
            p.marca,
            p.categoria,
            p.unidad,
            MIN(COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo)) AS precio_costo_min,
            MAX(COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo)) AS precio_costo_max,
            MIN(COALESCE(NULLIF(i.precio_venta, 0), p.precio_venta)) AS precio_venta_min,
            MAX(COALESCE(NULLIF(i.precio_venta, 0), p.precio_venta)) AS precio_venta_max,
            COUNT(DISTINCT i.sucursal_id) AS sucursales_con_precio
        FROM productos p
        INNER JOIN inventario_sucursal i ON i.producto_id = p.id
        WHERE p.eliminado = 0
          AND i.eliminado = 0
          AND i.sucursal_id IN ({placeholders})
        GROUP BY p.id, p.codigo_barras, p.codigo_proveedor, p.clave_producto,
                 p.descripcion, p.marca, p.categoria, p.unidad
        ORDER BY p.descripcion COLLATE NOCASE
        "
    );

    let mut stmt = conn.prepare(&sql).map_err(AppError::from).map_err(to_command_error)?;
    let iter = stmt
        .query_map(params_from_iter(scoped_ids.iter()), |row| {
            let precio_costo_min: f64 = row.get(8)?;
            let precio_costo_max: f64 = row.get(9)?;
            let precio_venta_min: f64 = row.get(10)?;
            let precio_venta_max: f64 = row.get(11)?;
            Ok(ProductoPromocionPrecio {
                id: row.get(0)?,
                codigo_barras: row.get(1)?,
                codigo_proveedor: row.get(2)?,
                clave_producto: row.get(3)?,
                descripcion: row.get(4)?,
                marca: row.get(5)?,
                categoria: row.get(6)?,
                unidad: row.get(7)?,
                precio_costo: precio_costo_max,
                precio_venta: precio_venta_min,
                precio_costo_min,
                precio_costo_max,
                precio_venta_min,
                precio_venta_max,
                sucursales_con_precio: row.get(12)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut productos = Vec::new();
    for item in iter {
        productos.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(productos)
}

#[tauri::command]
fn guardar_promocion(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    promocion: PromocionInput,
) -> AppResult<()> {
    let promocion = normalize_promocion_input(promocion);
    validate_promocion_input(&promocion)?;
    let actor = require_admin_or_superadmin(&state_sesion)?;
    let sucursal_ids = scoped_promocion_sucursales(&actor, &promocion.sucursal_ids);
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_sucursales_exist(&conn, &sucursal_ids)?;
    validate_promocion_no_loss(&conn, &promocion, &sucursal_ids)?;

    if !is_superadmin(&actor) {
        let current_ids = get_promocion_sucursal_ids(&conn, &promocion.id).unwrap_or_default();
        if !current_ids.is_empty() && !current_ids.iter().all(|id| id == &actor.sucursal_id) {
            return Err("Operación inválida: un administrador solo puede modificar promociones de su sucursal.".to_string());
        }
    }

    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;
    tx.execute(
        "
        INSERT INTO promociones (
            id, nombre, tipo_descuento, valor, fecha_inicio, fecha_fin, activo,
            producto_id, categoria_id, marca, eliminado, sincronizado, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, 0, datetime('now'))
        ON CONFLICT(id) DO UPDATE SET
            nombre = excluded.nombre,
            tipo_descuento = excluded.tipo_descuento,
            valor = excluded.valor,
            fecha_inicio = excluded.fecha_inicio,
            fecha_fin = excluded.fecha_fin,
            activo = excluded.activo,
            producto_id = excluded.producto_id,
            categoria_id = excluded.categoria_id,
            marca = excluded.marca,
            eliminado = 0,
            sincronizado = 0,
            updated_at = datetime('now')
        ",
        params![
            promocion.id,
            promocion.nombre,
            promocion.tipo_descuento,
            promocion.valor,
            promocion.fecha_inicio,
            promocion.fecha_fin,
            if promocion.activo { 1 } else { 0 },
            promocion.producto_id,
            promocion.categoria_id,
            promocion.marca
        ],
    )
    .map_err(|error| map_write_error(error, "promoción"))
    .map_err(to_command_error)?;

    tx.execute(
        "
        UPDATE promocion_sucursales
        SET eliminado = 1,
            sincronizado = 0,
            updated_at = datetime('now')
        WHERE promocion_id = ?1
        ",
        [&promocion.id],
    )
    .map_err(|error| map_write_error(error, "sucursales de promoción"))
    .map_err(to_command_error)?;
    for sucursal_id in sucursal_ids {
        tx.execute(
            "
            INSERT INTO promocion_sucursales (promocion_id, sucursal_id, eliminado, sincronizado, updated_at)
            VALUES (?1, ?2, 0, 0, datetime('now'))
            ON CONFLICT(promocion_id, sucursal_id) DO UPDATE SET
                eliminado = 0,
                sincronizado = 0,
                updated_at = datetime('now')
            ",
            params![promocion.id, sucursal_id],
        )
        .map_err(|error| map_write_error(error, "sucursales de promoción"))
        .map_err(to_command_error)?;
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn eliminar_promocion(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
) -> AppResult<()> {
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let actor = require_admin_or_superadmin(&state_sesion)?;
    if !is_superadmin(&actor) {
        let current_ids = get_promocion_sucursal_ids(&conn, &id)?;
        if current_ids.is_empty() || !current_ids.iter().all(|sucursal_id| sucursal_id == &actor.sucursal_id) {
            return Err("Operación inválida: un administrador solo puede eliminar promociones de su sucursal.".to_string());
        }
    }
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;
    let affected = tx
        .execute(
            "UPDATE promociones SET eliminado = 1, activo = 0, sincronizado = 0, updated_at = datetime('now') WHERE id = ?1 AND eliminado = 0",
            [&id],
        )
        .map_err(|error| map_write_error(error, "promoción"))
        .map_err(to_command_error)?;
    if affected == 0 {
        return Err("No se encontró la promoción indicada.".to_string());
    }
    tx.execute(
        "
        UPDATE promocion_sucursales
        SET eliminado = 1,
            sincronizado = 0,
            updated_at = datetime('now')
        WHERE promocion_id = ?1
          AND eliminado = 0
        ",
        [&id],
    )
    .map_err(|error| map_write_error(error, "sucursales de promoción"))
    .map_err(to_command_error)?;
    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn importar_datos_universal_visual(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    payload: ImportarDatosUniversalInput,
) -> AppResult<ImportarDatosUniversalResult> {
    require_admin_or_superadmin(&state_sesion)?;
    if payload.rows.is_empty() {
        return Err("No hay filas para importar.".to_string());
    }

    let destino = normalize_upper_trim(&payload.destino);
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;
    let mut registros_upsertados = 0usize;
    let mut omitidos = 0usize;

    for (index, row) in payload.rows.iter().enumerate() {
        match destino.as_str() {
            "CLIENTES" => {
                let nombre = json_value_text(row, &["nombre", "cliente", "razonSocial", "razon_social"]);
                if nombre.trim().is_empty() {
                    omitidos += 1;
                    continue;
                }
                let id_raw = json_value_text(row, &["id", "clienteId", "cliente_id", "clave"]);
                let id = if id_raw.trim().is_empty() {
                    legacy_catalog_id("IMPORT-CLI", &format!("{}-{}", nombre, index + 1))
                } else {
                    legacy_catalog_id("IMPORT-CLI", &id_raw)
                };
                let telefono = normalize_plain_trim(&json_value_text(row, &["telefono", "tel", "celular", "phone"]));
                let direccion = normalize_title_trim(&json_value_text(row, &["direccion", "domicilio", "address"]));
                let limite_credito = json_value_number(row, &["limiteCredito", "limite_credito", "credito"]);
                let saldo_deudor = json_value_number(row, &["saldoDeudor", "saldo_deudor", "saldo"]);
                tx.execute(
                    "
                    INSERT INTO clientes (id, nombre, telefono, direccion, limite_credito, saldo_deudor, sincronizado, updated_at, eliminado)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, datetime('now'), 0)
                    ON CONFLICT(id) DO UPDATE SET
                        nombre = excluded.nombre,
                        telefono = excluded.telefono,
                        direccion = excluded.direccion,
                        limite_credito = excluded.limite_credito,
                        saldo_deudor = excluded.saldo_deudor,
                        eliminado = 0,
                        sincronizado = 0,
                        updated_at = datetime('now')
                    ",
                    params![id, normalize_title_trim(&nombre), telefono, direccion, limite_credito.max(0.0), saldo_deudor.max(0.0)],
                )
                .map_err(|error| map_write_error(error, "cliente"))
                .map_err(to_command_error)?;
                registros_upsertados += 1;
            }
            "PROVEEDORES" => {
                let nombre = json_value_text(row, &["nombre", "proveedor", "razonSocial", "razon_social"]);
                if nombre.trim().is_empty() {
                    omitidos += 1;
                    continue;
                }
                let id_raw = json_value_text(row, &["id", "proveedorId", "proveedor_id", "clave"]);
                let id = if id_raw.trim().is_empty() {
                    legacy_catalog_id("IMPORT-PROV", &format!("{}-{}", nombre, index + 1))
                } else {
                    legacy_catalog_id("IMPORT-PROV", &id_raw)
                };
                let contacto = normalize_title_trim(&json_value_text(row, &["contactoNombre", "contacto_nombre", "contacto", "encargado"]));
                let telefono = normalize_plain_trim(&json_value_text(row, &["telefono", "tel", "celular", "phone"]));
                let email = normalize_email_trim(&json_value_text(row, &["email", "correo", "mail"]));
                let direccion = normalize_title_trim(&json_value_text(row, &["direccion", "domicilio", "address"]));
                tx.execute(
                    "
                    INSERT INTO proveedores (id, nombre, contacto_nombre, telefono, email, direccion, sincronizado, updated_at, eliminado)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, datetime('now'), 0)
                    ON CONFLICT(id) DO UPDATE SET
                        nombre = excluded.nombre,
                        contacto_nombre = excluded.contacto_nombre,
                        telefono = excluded.telefono,
                        email = excluded.email,
                        direccion = excluded.direccion,
                        eliminado = 0,
                        sincronizado = 0,
                        updated_at = datetime('now')
                    ",
                    params![id, normalize_title_trim(&nombre), contacto, telefono, email, direccion],
                )
                .map_err(|error| map_write_error(error, "proveedor"))
                .map_err(to_command_error)?;
                registros_upsertados += 1;
            }
            "MARCAS" | "CATEGORIAS" => {
                let nombre = json_value_text(row, &["nombre", "marca", "categoria", "descripcion"]);
                if nombre.trim().is_empty() {
                    omitidos += 1;
                    continue;
                }
                let table = if destino == "MARCAS" { "marcas" } else { "categorias" };
                let prefix = if destino == "MARCAS" { "IMPORT-MARCA" } else { "IMPORT-CAT" };
                let id_raw = json_value_text(row, &["id", "clave"]);
                let id = if id_raw.trim().is_empty() {
                    legacy_catalog_id(prefix, &nombre)
                } else {
                    legacy_catalog_id(prefix, &id_raw)
                };
                tx.execute(
                    &format!(
                        "
                        INSERT INTO {table} (id, nombre, sincronizado, updated_at, eliminado)
                        VALUES (?1, ?2, 0, datetime('now'), 0)
                        ON CONFLICT(id) DO UPDATE SET
                            nombre = excluded.nombre,
                            eliminado = 0,
                            sincronizado = 0,
                            updated_at = datetime('now')
                        "
                    ),
                    params![id, normalize_title_trim(&nombre)],
                )
                .map_err(|error| map_write_error(error, table))
                .map_err(to_command_error)?;
                registros_upsertados += 1;
            }
            "UNIDADES" => {
                let nombre = json_value_text(row, &["nombre", "unidad", "descripcion"]);
                if nombre.trim().is_empty() {
                    omitidos += 1;
                    continue;
                }
                let id_raw = json_value_text(row, &["id", "clave"]);
                let id = if id_raw.trim().is_empty() {
                    legacy_catalog_id("IMPORT-UNI", &nombre)
                } else {
                    legacy_catalog_id("IMPORT-UNI", &id_raw)
                };
                let clave_sat = normalize_upper_trim(&json_value_text(row, &["claveSat", "clave_sat", "satClaveUnidad", "sat_clave_unidad"]));
                let clave_sat = if clave_sat.len() == 3 { clave_sat } else { "H87".to_string() };
                tx.execute(
                    "
                    INSERT INTO unidades (id, nombre, clave_sat, sincronizado, updated_at, eliminado)
                    VALUES (?1, ?2, ?3, 0, datetime('now'), 0)
                    ON CONFLICT(id) DO UPDATE SET
                        nombre = excluded.nombre,
                        clave_sat = excluded.clave_sat,
                        eliminado = 0,
                        sincronizado = 0,
                        updated_at = datetime('now')
                    ",
                    params![id, normalize_title_trim(&nombre), clave_sat],
                )
                .map_err(|error| map_write_error(error, "unidad"))
                .map_err(to_command_error)?;
                registros_upsertados += 1;
            }
            _ => {
                return Err("Destino no soportado por el importador universal.".to_string());
            }
        }
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(ImportarDatosUniversalResult {
        destino,
        total_leidos: payload.rows.len(),
        registros_upsertados,
        omitidos,
    })
}

fn csv_value(record: &csv::StringRecord, column_indexes: &HashMap<String, usize>, field: &str) -> String {
    column_indexes
        .get(field)
        .and_then(|index| record.get(*index))
        .map(normalize_plain_trim)
        .unwrap_or_default()
}

fn csv_money_value(record: &csv::StringRecord, column_indexes: &HashMap<String, usize>, field: &str) -> f64 {
    let raw = csv_value(record, column_indexes, field);
    if raw.is_empty() {
        return 0.0;
    }
    let clean = if raw.rfind(',') > raw.rfind('.') {
        raw.replace('.', "").replace(',', ".")
    } else {
        raw.replace(',', "")
    }
    .replace('$', "")
    .trim()
    .to_string();
    clean.parse::<f64>().unwrap_or(0.0).max(0.0)
}

fn csv_relation_value(
    record: &csv::StringRecord,
    column_indexes: &HashMap<String, usize>,
    relation_maps: &HashMap<String, HashMap<String, String>>,
    field: &str,
) -> String {
    let raw = csv_value(record, column_indexes, field);
    relation_maps
        .get(field)
        .and_then(|map| map.get(&raw))
        .cloned()
        .unwrap_or(raw)
}

fn sniff_csv_delimiter(file_path: &str) -> u8 {
    let mut buffer = String::new();
    if let Ok(mut file) = File::open(file_path) {
        let mut bytes = vec![0u8; 8192];
        if let Ok(size) = file.read(&mut bytes) {
            buffer = String::from_utf8_lossy(&bytes[..size]).to_string();
        }
    }
    let first_line = buffer
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    let comma = first_line.matches(',').count();
    let semicolon = first_line.matches(';').count();
    let tab = first_line.matches('\t').count();
    if semicolon >= comma && semicolon >= tab && semicolon > 0 {
        b';'
    } else if tab >= comma && tab >= semicolon && tab > 0 {
        b'\t'
    } else {
        b','
    }
}

fn resolve_csv_delimiter(file_path: &str, delimiter: &str) -> u8 {
    match delimiter.trim().to_uppercase().as_str() {
        "PUNTO_COMA" | "SEMICOLON" | ";" => b';',
        "TAB" | "TABULADOR" => b'\t',
        "COMA" | "COMMA" | "," => b',',
        _ => sniff_csv_delimiter(file_path),
    }
}

fn delimiter_label(delimiter: u8) -> String {
    match delimiter {
        b';' => "PUNTO_COMA".to_string(),
        b'\t' => "TAB".to_string(),
        _ => "COMA".to_string(),
    }
}

fn ensure_import_catalog(tx: &Transaction<'_>, table: &str, prefix: &str, name: &str) -> Result<String, AppError> {
    let clean_name = normalize_title_trim(name);
    let final_name = if clean_name.is_empty() {
        match table {
            "marcas" => "SIN MARCA".to_string(),
            "categorias" => "GENERAL".to_string(),
            "unidades" => "PIEZA".to_string(),
            _ => "GENERAL".to_string(),
        }
    } else {
        clean_name
    };
    let existing = tx
        .query_row(
            &format!("SELECT id FROM {table} WHERE nombre = ?1 COLLATE NOCASE AND eliminado = 0 LIMIT 1"),
            [&final_name],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = legacy_catalog_id(prefix, &final_name);
    tx.execute(
        &format!(
            "
            INSERT INTO {table} (id, nombre, sincronizado, updated_at, eliminado)
            VALUES (?1, ?2, 0, datetime('now'), 0)
            ON CONFLICT(id) DO UPDATE SET
                nombre = excluded.nombre,
                eliminado = 0,
                sincronizado = 0,
                updated_at = datetime('now')
            "
        ),
        params![id, final_name],
    )?;
    Ok(id)
}

#[tauri::command]
fn seleccionar_archivo_csv_importacion() -> AppResult<Option<CsvArchivoSeleccionado>> {
    let Some(path) = rfd::FileDialog::new()
        .set_title("Seleccionar CSV para importar")
        .add_filter("CSV", &["csv"])
        .pick_file()
    else {
        return Ok(None);
    };

    let file_path = path.to_string_lossy().to_string();
    let file_name = path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_else(|| "archivo.csv".to_string());
    let delimiter = sniff_csv_delimiter(&file_path);
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .delimiter(delimiter)
        .from_path(&file_path)
        .map_err(|error| format!("No se pudo leer encabezados del CSV seleccionado: {error}"))?;
    let headers = reader
        .headers()
        .map_err(|error| format!("No se detectaron encabezados válidos en el CSV: {error}"))?
        .iter()
        .map(|header| header.trim().to_string())
        .collect::<Vec<_>>();

    if headers.is_empty() {
        return Err("El CSV seleccionado no tiene encabezados.".to_string());
    }

    Ok(Some(CsvArchivoSeleccionado {
        file_path,
        file_name,
        headers,
        delimiter: delimiter_label(delimiter),
    }))
}

#[tauri::command]
fn analizar_csv_importacion(payload: AnalizarCsvImportacionInput) -> AppResult<AnalizarCsvImportacionResult> {
    let file_path = payload.file_path.trim().to_string();
    if file_path.is_empty() {
        return Err("Falta la ruta local del CSV para analizarlo por streaming.".to_string());
    }

    let relation_fields = ["proveedorId", "marca", "categoria", "unidad"];
    let delimiter = resolve_csv_delimiter(&file_path, &payload.delimiter);
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .delimiter(delimiter)
        .from_path(&file_path)
        .map_err(|error| format!("No se pudo abrir el CSV para análisis: {error}"))?;

    let mut total_filas = 0usize;
    let mut preview_rows = Vec::new();
    let mut warnings = Vec::new();
    let mut unique_sets: HashMap<String, HashSet<String>> = HashMap::new();
    let mut unique_values: HashMap<String, Vec<String>> = HashMap::new();
    for field in relation_fields {
        unique_sets.insert(field.to_string(), HashSet::new());
        unique_values.insert(field.to_string(), Vec::new());
    }

    for record_result in reader.records() {
        let record = match record_result {
            Ok(record) => record,
            Err(error) => {
                warnings.push(format!("Fila {} no se pudo leer: {}", total_filas + 2, error));
                continue;
            }
        };
        total_filas += 1;

        if preview_rows.len() < 20 {
            let mut row = HashMap::new();
            for (field, index) in &payload.column_indexes {
                row.insert(field.clone(), record.get(*index).unwrap_or_default().to_string());
            }
            preview_rows.push(row);
        }

        for field in relation_fields {
            if !payload.column_indexes.contains_key(field) {
                continue;
            }
            let value = csv_value(&record, &payload.column_indexes, field);
            if value.is_empty() {
                continue;
            }
            let set = unique_sets.entry(field.to_string()).or_default();
            if set.insert(value.clone()) {
                let values = unique_values.entry(field.to_string()).or_default();
                if values.len() < 500 {
                    values.push(value);
                }
            }
        }
    }

    for field in relation_fields {
        let total_unique = unique_sets.get(field).map(|set| set.len()).unwrap_or(0);
        let shown = unique_values.get(field).map(|values| values.len()).unwrap_or(0);
        if total_unique > shown {
            warnings.push(format!(
                "{field}: se detectaron {total_unique} valores únicos; se muestran {shown}. Divide el archivo o depura catálogos para una homologación más segura."
            ));
        }
    }

    Ok(AnalizarCsvImportacionResult {
        total_filas,
        unique_values,
        preview_rows,
        warnings,
    })
}

#[tauri::command]
fn importar_csv_productos_mapeado(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    payload: ImportarCsvProductosMapeadoInput,
) -> AppResult<ImportarCsvProductosMapeadoResult> {
    let actor = require_admin_or_superadmin(&state_sesion)?;
    let file_path = payload.file_path.trim().to_string();
    let sucursal_id = payload.sucursal_id.trim().to_string();
    ensure_can_read_sucursal(&actor, &sucursal_id)?;

    if file_path.is_empty() {
        return Err("Selecciona un archivo CSV válido para importar.".to_string());
    }
    for required in ["codigo", "descripcion", "precioVenta", "proveedorId"] {
        if !payload.column_indexes.contains_key(required) {
            return Err(format!("Falta mapear el campo obligatorio: {required}."));
        }
    }

    let delimiter = resolve_csv_delimiter(&file_path, &payload.delimiter);
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .delimiter(delimiter)
        .from_path(&file_path)
        .map_err(|error| format!("No se pudo abrir el CSV: {error}"))?;

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let sucursal_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1 AND eliminado = 0",
            [&sucursal_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if sucursal_exists == 0 {
        return Err("La sucursal seleccionada no existe o está eliminada.".to_string());
    }

    let mut total_leidos = 0usize;
    let mut productos_upsertados = 0usize;
    let mut inventario_upsertado = 0usize;
    let mut filas_omitidas = 0usize;
    let mut errores = Vec::new();

    for record_result in reader.records() {
        total_leidos += 1;
        let fila = total_leidos + 1;
        let record = match record_result {
            Ok(record) => record,
            Err(_) => {
                filas_omitidas += 1;
                if errores.len() < 50 {
                    errores.push(CsvImportIssue {
                        fila,
                        motivo: "No se pudo leer la fila CSV.".to_string(),
                        codigo: String::new(),
                        descripcion: String::new(),
                    });
                }
                continue;
            }
        };

        let codigo = normalize_upper_trim(&csv_value(&record, &payload.column_indexes, "codigo"));
        let descripcion = normalize_title_trim(&csv_value(&record, &payload.column_indexes, "descripcion"));
        if codigo.is_empty() || descripcion.is_empty() {
            filas_omitidas += 1;
            if errores.len() < 50 {
                errores.push(CsvImportIssue {
                    fila,
                    motivo: "Falta código o descripción.".to_string(),
                    codigo,
                    descripcion,
                });
            }
            continue;
        }

        let proveedor_id = csv_relation_value(&record, &payload.column_indexes, &payload.foreign_key_map, "proveedorId");
        let proveedor_id = if proveedor_id.trim().is_empty() || proveedor_id.eq_ignore_ascii_case("null") {
            filas_omitidas += 1;
            if errores.len() < 50 {
                errores.push(CsvImportIssue {
                    fila,
                    motivo: "Proveedor vacío o sin homologar.".to_string(),
                    codigo,
                    descripcion,
                });
            }
            continue;
        } else {
            proveedor_id.trim().to_string()
        };
        let proveedor_exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM proveedores WHERE id = ?1 AND eliminado = 0",
                [&proveedor_id],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if proveedor_exists == 0 {
            filas_omitidas += 1;
            if errores.len() < 50 {
                errores.push(CsvImportIssue {
                    fila,
                    motivo: format!("El proveedor homologado no existe o está eliminado: {proveedor_id}."),
                    codigo,
                    descripcion,
                });
            }
            continue;
        }

        let marca = csv_relation_value(&record, &payload.column_indexes, &payload.foreign_key_map, "marca");
        let categoria = csv_relation_value(&record, &payload.column_indexes, &payload.foreign_key_map, "categoria");
        let unidad = csv_relation_value(&record, &payload.column_indexes, &payload.foreign_key_map, "unidad");
        let marca_nombre = normalize_title_trim(&marca);
        let categoria_nombre = normalize_title_trim(&categoria);
        let unidad_nombre = normalize_title_trim(&unidad);

        ensure_import_catalog(&tx, "marcas", "IMPORT-MARCA", &marca_nombre).map_err(to_command_error)?;
        ensure_import_catalog(&tx, "categorias", "IMPORT-CAT", &categoria_nombre).map_err(to_command_error)?;
        let unidad_id = ensure_import_catalog(&tx, "unidades", "IMPORT-UNI", &unidad_nombre).map_err(to_command_error)?;
        let unidad_sat: String = tx
            .query_row("SELECT clave_sat FROM unidades WHERE id = ?1 AND eliminado = 0", [&unidad_id], |row| row.get(0))
            .unwrap_or_else(|_| "H87".to_string());

        let precio_costo = round_money(csv_money_value(&record, &payload.column_indexes, "precioCosto"));
        let precio_venta = round_money(csv_money_value(&record, &payload.column_indexes, "precioVenta"));
        if precio_venta <= 0.0 {
            filas_omitidas += 1;
            if errores.len() < 50 {
                errores.push(CsvImportIssue {
                    fila,
                    motivo: "Precio de venta inválido o en cero.".to_string(),
                    codigo,
                    descripcion,
                });
            }
            continue;
        }
        let stock = csv_money_value(&record, &payload.column_indexes, "stock");
        let stock_minimo = csv_money_value(&record, &payload.column_indexes, "stockMinimo");
        let codigo_barras = normalize_upper_trim(&csv_value(&record, &payload.column_indexes, "codigoBarras"));
        let producto_id = legacy_catalog_id("CSV-PROD", &codigo);

        tx.execute(
            "
            INSERT INTO productos (
                id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
                marca, categoria, unidad, precio_costo, costo_promedio, precio_venta,
                sat_clave_prod_serv, sat_clave_unidad, sincronizado, updated_at, eliminado
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10, ?11, '01010101', ?12, 0, datetime('now'), 0)
            ON CONFLICT(id) DO UPDATE SET
                codigo_barras = excluded.codigo_barras,
                codigo_proveedor = excluded.codigo_proveedor,
                proveedor_id = excluded.proveedor_id,
                clave_producto = excluded.clave_producto,
                descripcion = excluded.descripcion,
                marca = excluded.marca,
                categoria = excluded.categoria,
                unidad = excluded.unidad,
                precio_costo = excluded.precio_costo,
                costo_promedio = excluded.costo_promedio,
                precio_venta = excluded.precio_venta,
                sat_clave_unidad = excluded.sat_clave_unidad,
                eliminado = 0,
                sincronizado = 0,
                updated_at = datetime('now')
            ",
            params![
                producto_id,
                if codigo_barras.is_empty() { None::<String> } else { Some(codigo_barras) },
                codigo,
                proveedor_id,
                codigo,
                descripcion,
                if marca_nombre.is_empty() { "SIN MARCA".to_string() } else { marca_nombre },
                if categoria_nombre.is_empty() { "GENERAL".to_string() } else { categoria_nombre },
                if unidad_nombre.is_empty() { "PIEZA".to_string() } else { unidad_nombre },
                precio_costo,
                precio_venta,
                if unidad_sat.len() == 3 { unidad_sat } else { "H87".to_string() },
            ],
        )
        .map_err(|error| map_write_error(error, "producto"))
        .map_err(to_command_error)?;
        productos_upsertados += 1;

        tx.execute(
            "
            INSERT INTO inventario_sucursal (
                producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta, sincronizado, updated_at, eliminado
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, datetime('now'), 0)
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
                stock = excluded.stock,
                stock_minimo = excluded.stock_minimo,
                costo_promedio = excluded.costo_promedio,
                precio_venta = excluded.precio_venta,
                eliminado = 0,
                sincronizado = 0,
                updated_at = datetime('now')
            ",
            params![producto_id, sucursal_id, stock, stock_minimo, precio_costo, precio_venta],
        )
        .map_err(|error| map_write_error(error, "inventario"))
        .map_err(to_command_error)?;
        inventario_upsertado += 1;
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(ImportarCsvProductosMapeadoResult {
        total_leidos,
        productos_upsertados,
        inventario_upsertado,
        filas_omitidas,
        errores,
    })
}

#[tauri::command]
fn importar_articulos_legacy_visual(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    payload: ImportarArticulosLegacyInput,
) -> AppResult<ImportarArticulosLegacyResult> {
    let actor = require_admin_or_superadmin(&state_sesion)?;
    let sucursal_id = payload.sucursal_id.trim().to_string();
    ensure_can_read_sucursal(&actor, &sucursal_id)?;
    if payload.rows.is_empty() {
        return Err("No hay filas para importar.".to_string());
    }

    let mut proveedor_default_id = payload.proveedor_default_id.trim().to_string();
    if proveedor_default_id.is_empty() {
        proveedor_default_id = "PROV-SIN-ASIGNAR".to_string();
    }

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let sucursal_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1 AND eliminado = 0",
            [&sucursal_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if sucursal_exists == 0 {
        return Err("La sucursal seleccionada no existe o está eliminada.".to_string());
    }

    tx.execute(
        "
        INSERT INTO proveedores (id, nombre, contacto_nombre, telefono, email, direccion, sincronizado, updated_at, eliminado)
        VALUES (?1, 'PROVEEDOR SIN ASIGNAR', '', '', '', '', 0, datetime('now'), 0)
        ON CONFLICT(id) DO UPDATE SET
            nombre = excluded.nombre,
            eliminado = 0,
            sincronizado = 0,
            updated_at = datetime('now')
        ",
        [&proveedor_default_id],
    )
    .map_err(|error| map_write_error(error, "proveedor"))
    .map_err(to_command_error)?;

    tx.execute(
        "
        CREATE TABLE IF NOT EXISTS productos_legacy_meta (
            producto_id TEXT PRIMARY KEY,
            legacy_id INTEGER,
            caducidad TEXT,
            fotos TEXT,
            descripcion_catalogo TEXT,
            mayoreo_apartir REAL,
            a_granel TEXT,
            no_en_catalogo TEXT,
            ventas_negativas TEXT,
            created_at_legacy TEXT,
            updated_at_legacy TEXT,
            sincronizado INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            eliminado INTEGER NOT NULL DEFAULT 0 CHECK(eliminado IN (0, 1)),
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE CASCADE
        )
        ",
        [],
    )
    .map_err(AppError::from)
    .map_err(to_command_error)?;

    let mut productos_upsertados = 0usize;
    let mut inventario_upsertado = 0usize;
    let mut catalogos_actualizados = 0usize;

    for (index, row) in payload.rows.iter().enumerate() {
        let descripcion = row
            .descripcion_articulo
            .as_ref()
            .map(|v| normalize_title_trim(v))
            .unwrap_or_default();
        if descripcion.is_empty() {
            continue;
        }

        let producto_id = if let Some(legacy_id) = row.id {
            format!("LEGACY-ART-{legacy_id}")
        } else if let Some(clave) = &row.clave {
            let clean = clave.trim();
            if clean.is_empty() {
                format!("LEGACY-ART-AUTO-{}", index + 1)
            } else {
                legacy_catalog_id("LEGACY-ART", clean)
            }
        } else {
            format!("LEGACY-ART-AUTO-{}", index + 1)
        };

        let proveedor_id = if let Some(proveedor_nombre) = optional_normalized_name(row.proveedor_nombre.as_ref()) {
            let prov_id = row
                .provedor
                .map(|proveedor_legacy| format!("LEGACY-PROV-{proveedor_legacy}"))
                .unwrap_or_else(|| legacy_catalog_id("LEGACY-PROV", &proveedor_nombre));
            tx.execute(
                "
                INSERT INTO proveedores (id, nombre, contacto_nombre, telefono, email, direccion, sincronizado, updated_at, eliminado)
                VALUES (?1, ?2, '', '', '', '', 0, datetime('now'), 0)
                ON CONFLICT(id) DO UPDATE SET
                    nombre = excluded.nombre,
                    eliminado = 0,
                    sincronizado = 0,
                    updated_at = datetime('now')
                ",
                params![prov_id, proveedor_nombre],
            )
            .map_err(|error| map_write_error(error, "proveedor"))
            .map_err(to_command_error)?;
            catalogos_actualizados += 1;
            prov_id
        } else if let Some(proveedor_legacy) = row.provedor {
            if let Some(existing_id) = find_proveedor_id_by_legacy_id(
                &tx,
                &["IMPORT-PROV", "LEGACY-PROV", "PROV", "PROVEEDOR"],
                proveedor_legacy,
            )
            .map_err(to_command_error)?
            {
                existing_id
            } else {
                let prov_id = format!("LEGACY-PROV-{proveedor_legacy}");
                tx.execute(
                    "
                    INSERT INTO proveedores (id, nombre, contacto_nombre, telefono, email, direccion, sincronizado, updated_at, eliminado)
                    VALUES (?1, ?2, '', '', '', '', 0, datetime('now'), 0)
                    ON CONFLICT(id) DO UPDATE SET
                        nombre = excluded.nombre,
                        eliminado = 0,
                        sincronizado = 0,
                        updated_at = datetime('now')
                    ",
                    params![prov_id, format!("PROVEEDOR LEGACY #{proveedor_legacy}")],
                )
                .map_err(|error| map_write_error(error, "proveedor"))
                .map_err(to_command_error)?;
                catalogos_actualizados += 1;
                prov_id
            }
        } else {
            proveedor_default_id.clone()
        };

        let marca_resuelta = if let Some(marca_nombre) = optional_normalized_name(row.marca_nombre.as_ref()) {
            let marca_id = row
                .marca
                .map(|marca_legacy| format!("LEGACY-MARCA-{marca_legacy}"))
                .unwrap_or_else(|| legacy_catalog_id("LEGACY-MARCA", &marca_nombre));
            (marca_id, marca_nombre)
        } else if let Some(marca_legacy) = row.marca {
            find_catalog_by_legacy_id(
                &tx,
                "marcas",
                &["IMPORT-MARCA", "LEGACY-MARCA", "MARCA"],
                marca_legacy,
            )
            .map_err(to_command_error)?
            .unwrap_or_else(|| ("LEGACY-MARCA-SIN_MARCA".to_string(), "SIN MARCA".to_string()))
        } else {
            ("LEGACY-MARCA-SIN_MARCA".to_string(), "SIN MARCA".to_string())
        };
        let (marca_id, marca_nombre) = marca_resuelta;
        tx.execute(
            "
            INSERT INTO marcas (id, nombre, sincronizado, updated_at, eliminado)
            VALUES (?1, ?2, 0, datetime('now'), 0)
            ON CONFLICT(id) DO UPDATE SET
                nombre = excluded.nombre,
                eliminado = 0,
                sincronizado = 0,
                updated_at = datetime('now')
            ",
            params![marca_id, marca_nombre],
        )
        .map_err(|error| map_write_error(error, "marca"))
        .map_err(to_command_error)?;

        let categoria_nombre = row
            .categoria
            .as_ref()
            .map(|v| normalize_title_trim(v))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "GENERAL".to_string());
        let categoria_id = legacy_catalog_id("LEGACY-CAT", &categoria_nombre);
        tx.execute(
            "
            INSERT INTO categorias (id, nombre, sincronizado, updated_at, eliminado)
            VALUES (?1, ?2, 0, datetime('now'), 0)
            ON CONFLICT(id) DO UPDATE SET
                nombre = excluded.nombre,
                eliminado = 0,
                sincronizado = 0,
                updated_at = datetime('now')
            ",
            params![categoria_id, categoria_nombre],
        )
        .map_err(|error| map_write_error(error, "categoría"))
        .map_err(to_command_error)?;

        let unidad_nombre = row
            .unidad
            .as_ref()
            .map(|v| normalize_title_trim(v))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "PIEZA".to_string());
        let unidad_id = legacy_catalog_id("LEGACY-UNI", &unidad_nombre);
        tx.execute(
            "
            INSERT INTO unidades (id, nombre, clave_sat, sincronizado, updated_at, eliminado)
            VALUES (?1, ?2, 'H87', 0, datetime('now'), 0)
            ON CONFLICT(id) DO UPDATE SET
                nombre = excluded.nombre,
                eliminado = 0,
                sincronizado = 0,
                updated_at = datetime('now')
            ",
            params![unidad_id, unidad_nombre],
        )
        .map_err(|error| map_write_error(error, "unidad"))
        .map_err(to_command_error)?;

        catalogos_actualizados += 3;

        let precio_costo = round_money(row.precio_compra.unwrap_or(0.0).max(0.0));
        let precio_venta = round_money(
            row.precio_venta
                .filter(|v| *v > 0.0)
                .or(row.precio_1.filter(|v| *v > 0.0))
                .or(row.precio_2.filter(|v| *v > 0.0))
                .or(row.precio_3.filter(|v| *v > 0.0))
                .or(row.precio_4.filter(|v| *v > 0.0))
                .unwrap_or(0.0),
        );
        let codigo_proveedor = row
            .clave
            .as_ref()
            .map(|v| normalize_upper_trim(v))
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| format!("LEGACY-COD-{}", index + 1));
        let clave_producto = codigo_proveedor.clone();

        tx.execute(
            "
            INSERT INTO productos (
                id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
                marca, categoria, unidad, precio_costo, costo_promedio, precio_venta,
                sat_clave_prod_serv, sat_clave_unidad, precio_1, precio_2, precio_3, precio_4,
                mayoreo_apartir, a_granel, no_en_catalogo, ventas_negativas, caducidad, fotos,
                descripcion_catalogo, sincronizado, updated_at, eliminado
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, '01010101', 'H87',
                    ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, 0, datetime('now'), 0)
            ON CONFLICT(id) DO UPDATE SET
                codigo_barras = excluded.codigo_barras,
                codigo_proveedor = excluded.codigo_proveedor,
                proveedor_id = excluded.proveedor_id,
                clave_producto = excluded.clave_producto,
                descripcion = excluded.descripcion,
                marca = excluded.marca,
                categoria = excluded.categoria,
                unidad = excluded.unidad,
                precio_costo = excluded.precio_costo,
                costo_promedio = excluded.costo_promedio,
                precio_venta = excluded.precio_venta,
                sat_clave_prod_serv = excluded.sat_clave_prod_serv,
                sat_clave_unidad = excluded.sat_clave_unidad,
                precio_1 = excluded.precio_1,
                precio_2 = excluded.precio_2,
                precio_3 = excluded.precio_3,
                precio_4 = excluded.precio_4,
                mayoreo_apartir = excluded.mayoreo_apartir,
                a_granel = excluded.a_granel,
                no_en_catalogo = excluded.no_en_catalogo,
                ventas_negativas = excluded.ventas_negativas,
                caducidad = excluded.caducidad,
                fotos = excluded.fotos,
                descripcion_catalogo = excluded.descripcion_catalogo,
                eliminado = 0,
                sincronizado = 0,
                updated_at = datetime('now')
            ",
            params![
                producto_id,
                row.codigo_barra
                    .as_ref()
                    .map(|v| normalize_upper_trim(v))
                    .filter(|v| !v.is_empty()),
                codigo_proveedor,
                proveedor_id,
                clave_producto,
                descripcion,
                marca_nombre,
                categoria_nombre,
                unidad_nombre,
                precio_costo,
                precio_costo,
                precio_venta,
                row.precio_1.unwrap_or(0.0).max(0.0),
                row.precio_2.unwrap_or(0.0).max(0.0),
                row.precio_3.unwrap_or(0.0).max(0.0),
                row.precio_4.unwrap_or(0.0).max(0.0),
                row.mayoreo_apartir.unwrap_or(0.0).max(0.0),
                legacy_truthy(row.a_granel.as_ref()),
                legacy_truthy(row.no_en_catalogo.as_ref()),
                legacy_truthy(row.ventas_negativas.as_ref()),
                row.caducidad.as_ref().map(|v| normalize_plain_trim(v)).filter(|v| !v.is_empty()),
                row.fotos.as_ref().map(|v| normalize_plain_trim(v)).filter(|v| !v.is_empty()),
                row.descripcion_catalogo.as_ref().map(|v| normalize_title_trim(v)).filter(|v| !v.is_empty())
            ],
        )
        .map_err(|error| map_write_error(error, "producto"))
        .map_err(to_command_error)?;
        productos_upsertados += 1;

        tx.execute(
            "
            INSERT INTO inventario_sucursal (
                producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta, sincronizado, updated_at, eliminado
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, datetime('now'), 0)
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
                stock = excluded.stock,
                stock_minimo = excluded.stock_minimo,
                costo_promedio = excluded.costo_promedio,
                precio_venta = excluded.precio_venta,
                eliminado = 0,
                sincronizado = 0,
                updated_at = datetime('now')
            ",
            params![
                producto_id,
                sucursal_id,
                row.existencia_stock.unwrap_or(0.0).max(0.0),
                row.cant_min_stock.unwrap_or(0.0).max(0.0),
                precio_costo,
                precio_venta
            ],
        )
        .map_err(|error| map_write_error(error, "inventario"))
        .map_err(to_command_error)?;
        inventario_upsertado += 1;

        tx.execute(
            "
            INSERT INTO productos_legacy_meta (
                producto_id, legacy_id, caducidad, fotos, descripcion_catalogo, mayoreo_apartir,
                a_granel, no_en_catalogo, ventas_negativas, created_at_legacy, updated_at_legacy,
                sincronizado, updated_at, eliminado
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, datetime('now'), 0)
            ON CONFLICT(producto_id) DO UPDATE SET
                legacy_id = excluded.legacy_id,
                caducidad = excluded.caducidad,
                fotos = excluded.fotos,
                descripcion_catalogo = excluded.descripcion_catalogo,
                mayoreo_apartir = excluded.mayoreo_apartir,
                a_granel = excluded.a_granel,
                no_en_catalogo = excluded.no_en_catalogo,
                ventas_negativas = excluded.ventas_negativas,
                created_at_legacy = excluded.created_at_legacy,
                updated_at_legacy = excluded.updated_at_legacy,
                sincronizado = 0,
                updated_at = datetime('now'),
                eliminado = 0
            ",
            params![
                producto_id,
                row.id,
                row.caducidad.as_ref().map(|v| v.trim().to_string()),
                row.fotos.as_ref().map(|v| normalize_plain_trim(v)),
                row.descripcion_catalogo.as_ref().map(|v| normalize_title_trim(v)),
                row.mayoreo_apartir,
                row.a_granel.as_ref().map(|v| normalize_title_trim(v)),
                row.no_en_catalogo.as_ref().map(|v| normalize_title_trim(v)),
                row.ventas_negativas.as_ref().map(|v| normalize_title_trim(v)),
                row.created_at.as_ref().map(|v| normalize_plain_trim(v)),
                row.updated_at.as_ref().map(|v| normalize_plain_trim(v))
            ],
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(ImportarArticulosLegacyResult {
        total_leidos: payload.rows.len(),
        productos_upsertados,
        inventario_upsertado,
        catalogos_actualizados,
    })
}

fn normalize_page_args(page: i64, page_size: i64) -> (i64, i64) {
    let safe_page_size = page_size.clamp(10, 100);
    let safe_page = page.max(0);
    (safe_page, safe_page_size)
}

#[tauri::command]
fn get_productos_catalogo_page(
    state_db: tauri::State<DbState>,
    query: String,
    page: i64,
    page_size: i64,
) -> AppResult<ProductoCatalogoPage> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let (page, page_size) = normalize_page_args(page, page_size);
    let offset = page * page_size;
    let query = query.trim().to_string();
    let pattern = format!("%{}%", query);

    let total: i64 = if query.is_empty() {
        conn.query_row("SELECT COUNT(*) FROM productos WHERE eliminado = 0", [], |row| row.get(0))
            .map_err(AppError::from)
            .map_err(to_command_error)?
    } else {
        conn.query_row(
            "
            SELECT COUNT(*)
            FROM productos
            WHERE eliminado = 0
              AND (
                codigo_barras LIKE ?1 COLLATE NOCASE
                OR codigo_proveedor LIKE ?1 COLLATE NOCASE
                OR clave_producto LIKE ?1 COLLATE NOCASE
                OR descripcion LIKE ?1 COLLATE NOCASE
                OR marca LIKE ?1 COLLATE NOCASE
                OR categoria LIKE ?1 COLLATE NOCASE
                OR unidad LIKE ?1 COLLATE NOCASE
              )
            ",
            [&pattern],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?
    };

    let sql = if query.is_empty() {
        "
        SELECT id, TRIM(COALESCE(codigo_barras, '')), TRIM(COALESCE(codigo_proveedor, '')),
               TRIM(COALESCE(proveedor_id, '')), TRIM(COALESCE(clave_producto, '')),
               TRIM(COALESCE(descripcion, '')), TRIM(COALESCE(marca, '')),
               TRIM(COALESCE(categoria, '')), TRIM(COALESCE(unidad, '')),
               precio_costo, costo_promedio, precio_venta,
               TRIM(COALESCE(sat_clave_prod_serv, '')), TRIM(COALESCE(sat_clave_unidad, '')),
               COALESCE(precio_1, 0), COALESCE(precio_2, 0), COALESCE(precio_3, 0), COALESCE(precio_4, 0),
               COALESCE(mayoreo_apartir, 0), COALESCE(a_granel, 0), COALESCE(no_en_catalogo, 0),
               COALESCE(ventas_negativas, 0), caducidad, TRIM(COALESCE(fotos, '')),
               TRIM(COALESCE(descripcion_catalogo, ''))
        FROM productos
        WHERE eliminado = 0
        ORDER BY TRIM(descripcion)
        LIMIT ?1 OFFSET ?2
        "
    } else {
        "
        SELECT id, TRIM(COALESCE(codigo_barras, '')), TRIM(COALESCE(codigo_proveedor, '')),
               TRIM(COALESCE(proveedor_id, '')), TRIM(COALESCE(clave_producto, '')),
               TRIM(COALESCE(descripcion, '')), TRIM(COALESCE(marca, '')),
               TRIM(COALESCE(categoria, '')), TRIM(COALESCE(unidad, '')),
               precio_costo, costo_promedio, precio_venta,
               TRIM(COALESCE(sat_clave_prod_serv, '')), TRIM(COALESCE(sat_clave_unidad, '')),
               COALESCE(precio_1, 0), COALESCE(precio_2, 0), COALESCE(precio_3, 0), COALESCE(precio_4, 0),
               COALESCE(mayoreo_apartir, 0), COALESCE(a_granel, 0), COALESCE(no_en_catalogo, 0),
               COALESCE(ventas_negativas, 0), caducidad, TRIM(COALESCE(fotos, '')),
               TRIM(COALESCE(descripcion_catalogo, ''))
        FROM productos
        WHERE eliminado = 0
          AND (
            codigo_barras LIKE ?1 COLLATE NOCASE
            OR codigo_proveedor LIKE ?1 COLLATE NOCASE
            OR clave_producto LIKE ?1 COLLATE NOCASE
            OR descripcion LIKE ?1 COLLATE NOCASE
            OR marca LIKE ?1 COLLATE NOCASE
            OR categoria LIKE ?1 COLLATE NOCASE
            OR unidad LIKE ?1 COLLATE NOCASE
          )
        ORDER BY TRIM(descripcion)
        LIMIT ?2 OFFSET ?3
        "
    };
    let mut stmt = conn.prepare(sql).map_err(AppError::from).map_err(to_command_error)?;
    let mapper = |row: &rusqlite::Row<'_>| {
        Ok(Producto {
            id: row.get(0)?,
            codigo_barras: row.get(1)?,
            codigo_proveedor: row.get(2)?,
            proveedor_id: row.get(3)?,
            clave_producto: row.get(4)?,
            descripcion: row.get(5)?,
            marca: row.get(6)?,
            categoria: row.get(7)?,
            unidad: row.get(8)?,
            precio_costo: row.get(9)?,
            costo_promedio: row.get(10)?,
            precio_venta: row.get(11)?,
            sat_clave_prod_serv: row.get(12)?,
            sat_clave_unidad: row.get(13)?,
            precio_1: row.get(14)?,
            precio_2: row.get(15)?,
            precio_3: row.get(16)?,
            precio_4: row.get(17)?,
            mayoreo_apartir: row.get(18)?,
            a_granel: row.get::<_, i64>(19)? == 1,
            no_en_catalogo: row.get::<_, i64>(20)? == 1,
            ventas_negativas: row.get::<_, i64>(21)? == 1,
            caducidad: row.get(22)?,
            fotos: row.get(23)?,
            descripcion_catalogo: row.get(24)?,
        })
    };
    let iter = if query.is_empty() {
        stmt.query_map(params![page_size, offset], mapper)
    } else {
        stmt.query_map(params![pattern, page_size, offset], mapper)
    }
    .map_err(AppError::from)
    .map_err(to_command_error)?;
    let mut rows = Vec::new();
    for item in iter {
        rows.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(ProductoCatalogoPage { rows, total })
}

#[tauri::command]
fn get_productos_catalogo(state_db: tauri::State<DbState>) -> AppResult<Vec<Producto>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare(
            "
            SELECT id, TRIM(COALESCE(codigo_barras, '')), TRIM(COALESCE(codigo_proveedor, '')),
                   TRIM(COALESCE(proveedor_id, '')), TRIM(COALESCE(clave_producto, '')),
                   TRIM(COALESCE(descripcion, '')), TRIM(COALESCE(marca, '')),
                   TRIM(COALESCE(categoria, '')), TRIM(COALESCE(unidad, '')),
                   precio_costo, costo_promedio, precio_venta,
                   TRIM(COALESCE(sat_clave_prod_serv, '')), TRIM(COALESCE(sat_clave_unidad, '')),
                   COALESCE(precio_1, 0), COALESCE(precio_2, 0), COALESCE(precio_3, 0), COALESCE(precio_4, 0),
                   COALESCE(mayoreo_apartir, 0), COALESCE(a_granel, 0), COALESCE(no_en_catalogo, 0),
                   COALESCE(ventas_negativas, 0), caducidad, TRIM(COALESCE(fotos, '')),
                   TRIM(COALESCE(descripcion_catalogo, ''))
            FROM productos
            WHERE eliminado = 0
            ORDER BY TRIM(descripcion)
            ",
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map([], |row| {
            Ok(Producto {
                id: row.get(0)?,
                codigo_barras: row.get(1)?,
                codigo_proveedor: row.get(2)?,
                proveedor_id: row.get(3)?,
                clave_producto: row.get(4)?,
                descripcion: row.get(5)?,
                marca: row.get(6)?,
                categoria: row.get(7)?,
                unidad: row.get(8)?,
                precio_costo: row.get(9)?,
                costo_promedio: row.get(10)?,
                precio_venta: row.get(11)?,
                sat_clave_prod_serv: row.get(12)?,
                sat_clave_unidad: row.get(13)?,
                precio_1: row.get(14)?,
                precio_2: row.get(15)?,
                precio_3: row.get(16)?,
                precio_4: row.get(17)?,
                mayoreo_apartir: row.get(18)?,
                a_granel: row.get::<_, i64>(19)? == 1,
                no_en_catalogo: row.get::<_, i64>(20)? == 1,
                ventas_negativas: row.get::<_, i64>(21)? == 1,
                caducidad: row.get(22)?,
                fotos: row.get(23)?,
                descripcion_catalogo: row.get(24)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut productos = Vec::new();
    for item in iter {
        productos.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(productos)
}

#[tauri::command]
fn create_producto_catalogo(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    producto: Producto,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let mut producto = sanitize_producto(producto);
    producto.precio_costo = 0.0;
    producto.costo_promedio = 0.0;
    producto.precio_venta = 0.0;
    validate_producto(&producto).map_err(to_command_error)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let proveedor_id = resolve_valid_producto_proveedor_id(&conn, &producto.proveedor_id).map_err(to_command_error)?;
    let marca = ensure_catalog_value_exists(&conn, "marcas", "marca", &producto.marca)
        .map_err(to_command_error)?;
    let categoria = ensure_catalog_value_exists(&conn, "categorias", "categoría", &producto.categoria)
        .map_err(to_command_error)?;
    let unidad = ensure_catalog_value_exists(&conn, "unidades", "unidad", &producto.unidad)
        .map_err(to_command_error)?;
    conn.execute(
        "
        INSERT INTO productos (
            id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
            marca, categoria, unidad, precio_costo, costo_promedio, precio_venta,
            sat_clave_prod_serv, sat_clave_unidad, precio_1, precio_2, precio_3, precio_4,
            mayoreo_apartir, a_granel, no_en_catalogo, ventas_negativas, caducidad, fotos,
            descripcion_catalogo, sincronizado, updated_at, eliminado
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, 0, 0, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, 0, datetime('now'), 0)
        ",
        params![
            producto.id,
            if producto.codigo_barras.trim().is_empty() {
                None::<String>
            } else {
                Some(producto.codigo_barras)
            },
            producto.codigo_proveedor,
            proveedor_id,
            producto.clave_producto,
            producto.descripcion,
            marca,
            categoria,
            unidad,
            producto.sat_clave_prod_serv,
            producto.sat_clave_unidad,
            producto.precio_1,
            producto.precio_2,
            producto.precio_3,
            producto.precio_4,
            producto.mayoreo_apartir,
            if producto.a_granel { 1 } else { 0 },
            if producto.no_en_catalogo { 1 } else { 0 },
            if producto.ventas_negativas { 1 } else { 0 },
            producto.caducidad,
            producto.fotos,
            producto.descripcion_catalogo
        ],
    )
    .map_err(|error| map_write_error(error, "producto"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn update_producto_catalogo(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    producto_id: String,
    producto: Producto,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    let mut producto = sanitize_producto(producto);
    producto.precio_costo = 0.0;
    producto.costo_promedio = 0.0;
    producto.precio_venta = 0.0;
    validate_producto(&producto).map_err(to_command_error)?;
    if producto_id.trim().is_empty() {
        return Err("Falta el identificador del producto a actualizar.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let proveedor_id = resolve_valid_producto_proveedor_id(&conn, &producto.proveedor_id).map_err(to_command_error)?;
    let marca = ensure_catalog_value_exists(&conn, "marcas", "marca", &producto.marca)
        .map_err(to_command_error)?;
    let categoria = ensure_catalog_value_exists(&conn, "categorias", "categoría", &producto.categoria)
        .map_err(to_command_error)?;
    let unidad = ensure_catalog_value_exists(&conn, "unidades", "unidad", &producto.unidad)
        .map_err(to_command_error)?;
    let affected = conn
        .execute(
            "
            UPDATE productos
            SET codigo_barras = ?1,
                codigo_proveedor = ?2,
                proveedor_id = ?3,
                clave_producto = ?4,
                descripcion = ?5,
                marca = ?6,
                categoria = ?7,
                unidad = ?8,
                sat_clave_prod_serv = ?9,
                sat_clave_unidad = ?10,
                precio_1 = ?11,
                precio_2 = ?12,
                precio_3 = ?13,
                precio_4 = ?14,
                mayoreo_apartir = ?15,
                a_granel = ?16,
                no_en_catalogo = ?17,
                ventas_negativas = ?18,
                caducidad = ?19,
                fotos = ?20,
                descripcion_catalogo = ?21,
                sincronizado = 0,
                updated_at = datetime('now')
            WHERE id = ?22 AND eliminado = 0
            ",
            params![
                if producto.codigo_barras.trim().is_empty() {
                    None::<String>
                } else {
                    Some(producto.codigo_barras)
                },
                producto.codigo_proveedor,
                proveedor_id,
                producto.clave_producto,
                producto.descripcion,
                marca,
                categoria,
                unidad,
                producto.sat_clave_prod_serv,
                producto.sat_clave_unidad,
                producto.precio_1,
                producto.precio_2,
                producto.precio_3,
                producto.precio_4,
                producto.mayoreo_apartir,
                if producto.a_granel { 1 } else { 0 },
                if producto.no_en_catalogo { 1 } else { 0 },
                if producto.ventas_negativas { 1 } else { 0 },
                producto.caducidad,
                producto.fotos,
                producto.descripcion_catalogo,
                producto_id
            ],
        )
        .map_err(|error| map_write_error(error, "producto"))
        .map_err(to_command_error)?;
    if affected == 0 {
        return Err("No se encontró el producto que intentas actualizar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn guardar_inventario_sucursal(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    producto_id: String,
    inventario: InventarioSucursalInput,
) -> AppResult<()> {
    let actor = require_admin_or_superadmin(&state_sesion)?;
    ensure_can_read_sucursal(&actor, &inventario.sucursal_id)?;
    if producto_id.trim().is_empty() {
        return Err("Selecciona un producto válido.".to_string());
    }
    validate_inventario_input(&inventario).map_err(to_command_error)?;

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;
    let producto_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM productos WHERE id = ?1 AND eliminado = 0",
            [&producto_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if producto_exists == 0 {
        return Err("El producto seleccionado no existe.".to_string());
    }

    let sucursal_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1 AND eliminado = 0",
            [&inventario.sucursal_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if sucursal_exists == 0 {
        return Err("La sucursal seleccionada no existe.".to_string());
    }

    let stock_anterior: f64 = tx
        .query_row(
            "SELECT stock FROM inventario_sucursal WHERE producto_id = ?1 AND sucursal_id = ?2",
            params![&producto_id, &inventario.sucursal_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?
        .unwrap_or(0.0);

    tx.execute(
        "
        INSERT INTO inventario_sucursal (
            producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta,
            sincronizado, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, datetime('now'))
        ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
          stock = excluded.stock,
          stock_minimo = excluded.stock_minimo,
          costo_promedio = excluded.costo_promedio,
          precio_venta = excluded.precio_venta,
          eliminado = 0,
          sincronizado = 0,
          updated_at = datetime('now')
        ",
        params![
            &producto_id,
            &inventario.sucursal_id,
            inventario.stock,
            inventario.stock_minimo,
            inventario.costo_promedio,
            inventario.precio_venta
        ],
    )
    .map_err(|error| map_write_error(error, "inventario"))
    .map_err(to_command_error)?;

    let diferencia = ((inventario.stock - stock_anterior) * 1000.0).round() / 1000.0;
    if diferencia.abs() > f64::EPSILON {
        let tipo = if diferencia > 0.0 {
            "AJUSTE_ENTRADA"
        } else {
            "AJUSTE_SALIDA"
        };
        let fecha_movimiento: String = tx
            .query_row("SELECT datetime('now')", [], |row| row.get(0))
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        insertar_movimiento_inventario(
            &tx,
            &producto_id,
            &inventario.sucursal_id,
            tipo,
            "INVENTARIO",
            &format!("AJUSTE-STOCK-{}", current_timestamp_string()),
            diferencia,
            Some(inventario.costo_promedio),
            Some(&actor.id),
            &fecha_movimiento,
        )
        .map_err(to_command_error)?;
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn eliminar_inventario_sucursal(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    producto_id: String,
    sucursal_id: String,
) -> AppResult<()> {
    let actor = require_admin_or_superadmin(&state_sesion)?;
    ensure_can_read_sucursal(&actor, &sucursal_id)?;
    if producto_id.trim().is_empty() || sucursal_id.trim().is_empty() {
        return Err("Falta el producto o la sucursal a eliminar del inventario.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let affected = conn
        .execute(
            "
            UPDATE inventario_sucursal
            SET eliminado = 1,
                sincronizado = 0,
                updated_at = datetime('now')
            WHERE producto_id = ?1
              AND sucursal_id = ?2
              AND eliminado = 0
            ",
            params![producto_id, sucursal_id],
        )
        .map_err(|error| map_write_error(error, "inventario"))
        .map_err(to_command_error)?;

    if affected == 0 {
        return Err("No se encontró el producto en el inventario de esta sucursal.".to_string());
    }

    Ok(())
}

#[tauri::command]
fn create_producto(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    producto: Producto,
    inventario: InventarioSucursalInput,
) -> AppResult<()> {
    let actor = require_admin_or_superadmin(&state_sesion)?;
    ensure_can_read_sucursal(&actor, &inventario.sucursal_id)?;
    let producto = sanitize_producto(producto);
    validate_producto(&producto).map_err(to_command_error)?;
    validate_inventario_input(&inventario).map_err(to_command_error)?;

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let proveedor_id = resolve_valid_producto_proveedor_id(&conn, &producto.proveedor_id).map_err(to_command_error)?;
    let marca = ensure_catalog_value_exists(&conn, "marcas", "marca", &producto.marca)
        .map_err(to_command_error)?;
    let categoria = ensure_catalog_value_exists(&conn, "categorias", "categoría", &producto.categoria)
        .map_err(to_command_error)?;
    let unidad = ensure_catalog_value_exists(&conn, "unidades", "unidad", &producto.unidad)
        .map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    tx.execute(
        "
        INSERT INTO productos (
            id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
            marca, categoria, unidad, precio_costo, costo_promedio, precio_venta,
            sat_clave_prod_serv, sat_clave_unidad, precio_1, precio_2, precio_3, precio_4,
            mayoreo_apartir, a_granel, no_en_catalogo, ventas_negativas, caducidad, fotos,
            descripcion_catalogo, sincronizado, updated_at, eliminado
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, 0, datetime('now'), 0)
        ",
        params![
            producto.id,
            if producto.codigo_barras.trim().is_empty() {
                None::<String>
            } else {
                Some(producto.codigo_barras)
            },
            producto.codigo_proveedor,
            proveedor_id,
            producto.clave_producto,
            producto.descripcion,
            marca,
            categoria,
            unidad,
            producto.precio_costo,
            if producto.costo_promedio > 0.0 { producto.costo_promedio } else { producto.precio_costo },
            producto.precio_venta,
            producto.sat_clave_prod_serv,
            producto.sat_clave_unidad,
            producto.precio_1,
            producto.precio_2,
            producto.precio_3,
            producto.precio_4,
            producto.mayoreo_apartir,
            if producto.a_granel { 1 } else { 0 },
            if producto.no_en_catalogo { 1 } else { 0 },
            if producto.ventas_negativas { 1 } else { 0 },
            producto.caducidad,
            producto.fotos,
            producto.descripcion_catalogo
        ],
    )
    .map_err(|error| map_write_error(error, "producto"))
    .map_err(to_command_error)?;

    tx.execute(
        "
        INSERT INTO inventario_sucursal (
            producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta,
            sincronizado, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, datetime('now'))
        ",
        params![
            producto.id,
            inventario.sucursal_id,
            inventario.stock,
            inventario.stock_minimo,
            if inventario.costo_promedio > 0.0 {
                inventario.costo_promedio
            } else if producto.costo_promedio > 0.0 {
                producto.costo_promedio
            } else {
                producto.precio_costo
            },
            if inventario.precio_venta > 0.0 { inventario.precio_venta } else { producto.precio_venta }
        ],
    )
    .map_err(|error| map_write_error(error, "inventario"))
    .map_err(to_command_error)?;

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn update_producto(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    producto_id: String,
    producto: Producto,
    inventario: InventarioSucursalInput,
) -> AppResult<()> {
    let actor = require_admin_or_superadmin(&state_sesion)?;
    ensure_can_read_sucursal(&actor, &inventario.sucursal_id)?;
    let producto = sanitize_producto(producto);
    validate_producto(&producto).map_err(to_command_error)?;
    validate_inventario_input(&inventario).map_err(to_command_error)?;

    if producto_id.trim().is_empty() {
        return Err("Falta el identificador del producto a actualizar.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let proveedor_id = resolve_valid_producto_proveedor_id(&conn, &producto.proveedor_id).map_err(to_command_error)?;
    let marca = ensure_catalog_value_exists(&conn, "marcas", "marca", &producto.marca)
        .map_err(to_command_error)?;
    let categoria = ensure_catalog_value_exists(&conn, "categorias", "categoría", &producto.categoria)
        .map_err(to_command_error)?;
    let unidad = ensure_catalog_value_exists(&conn, "unidades", "unidad", &producto.unidad)
        .map_err(to_command_error)?;
    conn.execute(
        "
        UPDATE productos
        SET codigo_barras = ?1,
            codigo_proveedor = ?2,
            proveedor_id = ?3,
            clave_producto = ?4,
            descripcion = ?5,
            marca = ?6,
            categoria = ?7,
            unidad = ?8,
            precio_costo = ?9,
            costo_promedio = CASE WHEN ?10 > 0 THEN ?10 ELSE costo_promedio END,
            precio_venta = ?11,
            sat_clave_prod_serv = ?12,
            sat_clave_unidad = ?13,
            precio_1 = ?14,
            precio_2 = ?15,
            precio_3 = ?16,
            precio_4 = ?17,
            mayoreo_apartir = ?18,
            a_granel = ?19,
            no_en_catalogo = ?20,
            ventas_negativas = ?21,
            caducidad = ?22,
            fotos = ?23,
            descripcion_catalogo = ?24,
            sincronizado = 0,
            updated_at = datetime('now')
        WHERE id = ?25 AND eliminado = 0
        ",
        params![
            if producto.codigo_barras.trim().is_empty() {
                None::<String>
            } else {
                Some(producto.codigo_barras)
            },
            producto.codigo_proveedor,
            proveedor_id,
            producto.clave_producto,
            producto.descripcion,
            marca,
            categoria,
            unidad,
            producto.precio_costo,
            producto.costo_promedio,
            producto.precio_venta,
            producto.sat_clave_prod_serv,
            producto.sat_clave_unidad,
            producto.precio_1,
            producto.precio_2,
            producto.precio_3,
            producto.precio_4,
            producto.mayoreo_apartir,
            if producto.a_granel { 1 } else { 0 },
            if producto.no_en_catalogo { 1 } else { 0 },
            if producto.ventas_negativas { 1 } else { 0 },
            producto.caducidad,
            producto.fotos,
            producto.descripcion_catalogo,
            producto_id
        ],
    )
    .map_err(|error| map_write_error(error, "producto"))
    .map_err(to_command_error)?;

    conn.execute(
        "
        INSERT INTO inventario_sucursal (
            producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta,
            sincronizado, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, datetime('now'))
        ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
          stock = excluded.stock,
          stock_minimo = excluded.stock_minimo,
          costo_promedio = CASE WHEN excluded.costo_promedio > 0 THEN excluded.costo_promedio ELSE inventario_sucursal.costo_promedio END,
          precio_venta = CASE WHEN excluded.precio_venta > 0 THEN excluded.precio_venta ELSE inventario_sucursal.precio_venta END,
          sincronizado = 0,
          updated_at = datetime('now')
        ",
        params![
            producto_id,
            inventario.sucursal_id,
            inventario.stock,
            inventario.stock_minimo,
            if inventario.costo_promedio > 0.0 { inventario.costo_promedio } else { producto.costo_promedio },
            if inventario.precio_venta > 0.0 { inventario.precio_venta } else { producto.precio_venta }
        ],
    )
    .map_err(|error| map_write_error(error, "inventario"))
    .map_err(to_command_error)?;

    Ok(())
}

#[tauri::command]
fn delete_producto(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
) -> AppResult<()> {
    require_admin_or_superadmin(&state_sesion)?;
    if id.trim().is_empty() {
        return Err("Falta el identificador del producto.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let affected = conn
        .execute(
            "
            UPDATE productos
            SET eliminado = 1,
                sincronizado = 0,
                updated_at = datetime('now')
            WHERE id = ?1 AND eliminado = 0
            ",
            [&id],
        )
        .map_err(|error| map_write_error(error, "producto"))
        .map_err(to_command_error)?;

    if affected == 0 {
        return Err("No se encontró el producto que intentas eliminar.".to_string());
    }

    Ok(())
}

#[tauri::command]
fn registrar_compra(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    compra: RegistrarCompraInput,
) -> AppResult<()> {
    let actor = require_admin_or_superadmin(&state_sesion)?;
    ensure_can_read_sucursal(&actor, &compra.sucursal_id)?;
    validate_registrar_compra_input(&compra).map_err(to_command_error)?;

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;
    let detalles = consolidar_detalles_compra(&compra.detalles);

    let proveedor_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM proveedores WHERE id = ?1 AND eliminado = 0",
            [&compra.proveedor_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if proveedor_exists == 0 {
        return Err("El proveedor seleccionado ya no existe.".to_string());
    }

    let sucursal_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1 AND eliminado = 0",
            [&compra.sucursal_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if sucursal_exists == 0 {
        return Err("La sucursal seleccionada ya no existe.".to_string());
    }

    let mut total = 0.0_f64;
    for detalle in &detalles {
        total += detalle.cantidad * round_money(detalle.precio_costo_pactado);
    }
    total = round_money(total);

    tx.execute(
        "
        INSERT INTO compras (
            id, proveedor_id, sucursal_id, fecha, total,
            sync_uuid, sincronizado, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, datetime('now'))
        ",
        params![
            compra.id,
            compra.proveedor_id.trim(),
            compra.sucursal_id.trim(),
            compra.fecha.trim(),
            total,
            generate_uuid_like()
        ],
    )
    .map_err(|error| map_write_error(error, "compra"))
    .map_err(to_command_error)?;

    for detalle in &detalles {
        let precio_costo_pactado = round_money(detalle.precio_costo_pactado);
        let producto_exists: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM productos WHERE id = ?1 AND eliminado = 0",
                [&detalle.producto_id],
                |row| row.get(0),
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        if producto_exists == 0 {
            return Err(format!("El producto {} no existe o fue eliminado.", detalle.producto_id));
        }

        let (stock_actual, costo_actual) =
            inventario_costo_promedio(&tx, &detalle.producto_id, &compra.sucursal_id)
                .map_err(to_command_error)?;
        let nuevo_stock = stock_actual + detalle.cantidad;
        let nuevo_costo_promedio = round_money(if nuevo_stock > 0.0 {
            ((stock_actual * costo_actual) + (detalle.cantidad * precio_costo_pactado)) / nuevo_stock
        } else {
            precio_costo_pactado
        });

        tx.execute(
            "
            INSERT INTO detalle_compras (
                id, compra_id, producto_id, cantidad, precio_costo_pactado,
                costo_promedio_resultante, sync_uuid, sincronizado, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, datetime('now'))
            ",
            params![
                detalle.id,
                compra.id,
                detalle.producto_id,
                detalle.cantidad,
                precio_costo_pactado,
                nuevo_costo_promedio,
                generate_uuid_like()
            ],
        )
        .map_err(|error| map_write_error(error, "detalle de compra"))
        .map_err(to_command_error)?;

        tx.execute(
            "
            INSERT INTO inventario_sucursal (
                producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta,
                sincronizado, updated_at
            )
            VALUES (?1, ?2, ?3, 0, ?4, COALESCE((SELECT precio_venta FROM productos WHERE id = ?1), 0), 0, datetime('now'))
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
              stock = inventario_sucursal.stock + excluded.stock,
              costo_promedio = excluded.costo_promedio,
              sincronizado = 0,
              updated_at = datetime('now')
            ",
            params![detalle.producto_id, compra.sucursal_id, detalle.cantidad, nuevo_costo_promedio],
        )
        .map_err(|error| map_write_error(error, "inventario"))
        .map_err(to_command_error)?;

        tx.execute(
            "
            UPDATE productos
            SET precio_costo = CASE WHEN precio_costo <= 0 THEN ?1 ELSE precio_costo END,
                costo_promedio = CASE WHEN costo_promedio <= 0 THEN ?2 ELSE costo_promedio END,
                sincronizado = 0,
                updated_at = datetime('now')
            WHERE id = ?3
              AND (precio_costo <= 0 OR costo_promedio <= 0)
            ",
            params![precio_costo_pactado, nuevo_costo_promedio, detalle.producto_id],
        )
        .map_err(|error| map_write_error(error, "producto"))
        .map_err(to_command_error)?;

        insertar_movimiento_inventario(
            &tx,
            &detalle.producto_id,
            &compra.sucursal_id,
            "COMPRA",
            "COMPRA",
            &compra.id,
            detalle.cantidad,
            Some(precio_costo_pactado),
            None,
            &compra.fecha,
        )
        .map_err(to_command_error)?;
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

fn calcular_resumen_caja(conn: &Connection, sesion: &CajaSesion) -> Result<CajaEstado, AppError> {
    let ventas_efectivo: f64 = conn
        .query_row(
            "
            SELECT COALESCE(SUM(total), 0)
            FROM ventas
            WHERE usuario_id = ?1
              AND sucursal_id = ?2
              AND metodo_pago = 'EFECTIVO'
              AND estado = 'COMPLETADA'
              AND fecha >= ?3
            ",
            params![sesion.usuario_id, sesion.sucursal_id, sesion.fecha_apertura],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;

    let ingresos: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(monto), 0) FROM caja_movimientos WHERE sesion_id = ?1 AND tipo = 'INGRESO'",
            [&sesion.id],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;

    let egresos: f64 = conn
        .query_row(
            "
            SELECT COALESCE(SUM(monto), 0)
            FROM caja_movimientos
            WHERE sesion_id = ?1
              AND tipo = 'EGRESO'
              AND motivo NOT LIKE 'DEVOLUCIÓN EN VENTA #%'
            ",
            [&sesion.id],
            |row| row.get(0),
        )
        .map_err(AppError::from)?;

    let monto_esperado_actual = sesion.monto_inicial + ventas_efectivo + ingresos - egresos;

    Ok(CajaEstado {
        sesion: sesion.clone(),
        ventas_efectivo,
        ingresos,
        egresos,
        monto_esperado_actual,
    })
}

#[tauri::command]
fn get_caja_actual(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    usuario_id: String,
    sucursal_id: String,
) -> AppResult<Option<CajaEstado>> {
    let actor = current_session_user(&state_sesion)?;
    if actor.id != usuario_id || actor.sucursal_id != sucursal_id {
        return Err("No puedes consultar la caja de otro usuario o sucursal.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;

    let mut stmt = conn
        .prepare(
            "
            SELECT id, usuario_id, sucursal_id, fecha_apertura, monto_inicial, fecha_cierre, monto_final_real, monto_esperado, estado
            FROM cajas_sesiones
            WHERE usuario_id = ?1 AND sucursal_id = ?2 AND estado = 'ABIERTA'
            ORDER BY fecha_apertura DESC
            LIMIT 1
            ",
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let sesion = stmt
        .query_row(params![usuario_id, sucursal_id], |row| {
            Ok(CajaSesion {
                id: row.get(0)?,
                usuario_id: row.get(1)?,
                sucursal_id: row.get(2)?,
                fecha_apertura: row.get(3)?,
                monto_inicial: row.get(4)?,
                fecha_cierre: row.get(5)?,
                monto_final_real: row.get(6)?,
                monto_esperado: row.get(7)?,
                estado: row.get(8)?,
            })
        })
        .ok();

    match sesion {
        Some(value) => {
            let resumen = calcular_resumen_caja(&conn, &value).map_err(to_command_error)?;
            Ok(Some(resumen))
        }
        None => Ok(None),
    }
}

#[tauri::command]
fn abrir_caja(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    apertura: AbrirCajaInput,
) -> AppResult<CajaEstado> {
    let actor = current_session_user(&state_sesion)?;
    if actor.id != apertura.usuario_id || actor.sucursal_id != apertura.sucursal_id {
        return Err("No puedes abrir caja para otro usuario o sucursal.".to_string());
    }

    validate_abrir_caja_input(&apertura).map_err(to_command_error)?;
    let monto_inicial = normalize_money(apertura.monto_inicial, "El fondo inicial", true)
        .map_err(to_command_error)?;
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let abierta_actual: i64 = tx
        .query_row(
            "
            SELECT COUNT(*)
            FROM cajas_sesiones
            WHERE usuario_id = ?1 AND sucursal_id = ?2 AND estado = 'ABIERTA'
            ",
            params![apertura.usuario_id, apertura.sucursal_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if abierta_actual > 0 {
        return Err("Ya existe una caja ABIERTA para este usuario en esta sucursal.".to_string());
    }

    tx.execute(
        "
        INSERT INTO cajas_sesiones (
            id, usuario_id, sucursal_id, fecha_apertura, monto_inicial, fecha_cierre,
            monto_final_real, monto_esperado, estado, sync_uuid, sincronizado, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?5, 'ABIERTA', ?6, 0, datetime('now'))
        ",
        params![
            apertura.id,
            apertura.usuario_id,
            apertura.sucursal_id,
            apertura.fecha_apertura,
            monto_inicial,
            generate_uuid_like()
        ],
    )
    .map_err(|error| map_write_error(error, "sesión de caja"))
    .map_err(to_command_error)?;

    let sesion = CajaSesion {
        id: apertura.id,
        usuario_id: apertura.usuario_id,
        sucursal_id: apertura.sucursal_id,
        fecha_apertura: apertura.fecha_apertura,
        monto_inicial,
        fecha_cierre: None,
        monto_final_real: None,
        monto_esperado: monto_inicial,
        estado: "ABIERTA".to_string(),
    };

    let resumen = calcular_resumen_caja(&tx, &sesion).map_err(to_command_error)?;
    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(resumen)
}

#[tauri::command]
fn registrar_movimiento_caja(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    movimiento: MovimientoCajaInput,
) -> AppResult<CajaEstado> {
    let actor = current_session_user(&state_sesion)?;
    validate_movimiento_caja_input(&movimiento).map_err(to_command_error)?;
    let monto = normalize_money(movimiento.monto, "El monto del movimiento", false)
        .map_err(to_command_error)?;
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let sesion = tx
        .query_row(
            "
            SELECT id, usuario_id, sucursal_id, fecha_apertura, monto_inicial, fecha_cierre, monto_final_real, monto_esperado, estado
            FROM cajas_sesiones
            WHERE id = ?1
            ",
            [&movimiento.sesion_id],
            |row| {
                Ok(CajaSesion {
                    id: row.get(0)?,
                    usuario_id: row.get(1)?,
                    sucursal_id: row.get(2)?,
                    fecha_apertura: row.get(3)?,
                    monto_inicial: row.get(4)?,
                    fecha_cierre: row.get(5)?,
                    monto_final_real: row.get(6)?,
                    monto_esperado: row.get(7)?,
                    estado: row.get(8)?,
                })
            },
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if sesion.estado != "ABIERTA" {
        return Err("Solo se pueden registrar movimientos en una caja ABIERTA.".to_string());
    }
    if sesion.usuario_id != actor.id || sesion.sucursal_id != actor.sucursal_id {
        return Err("No puedes registrar movimientos en una caja que no pertenece a tu sesión.".to_string());
    }

    let resumen_antes = calcular_resumen_caja(&tx, &sesion).map_err(to_command_error)?;
    if movimiento.tipo == "EGRESO"
        && (monto * 100.0).round() > (resumen_antes.monto_esperado_actual * 100.0).round()
    {
        return Err(format!(
            "No puedes registrar un egreso de ${monto:.2} porque el efectivo esperado en caja es ${:.2}.",
            resumen_antes.monto_esperado_actual
        ));
    }

    tx.execute(
        "
        INSERT INTO caja_movimientos (id, sesion_id, tipo, monto, motivo, sync_uuid, sincronizado, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, datetime('now'))
        ",
        params![
            movimiento.id,
            movimiento.sesion_id,
            movimiento.tipo,
            monto,
            movimiento.motivo.trim(),
            generate_uuid_like()
        ],
    )
    .map_err(|error| map_write_error(error, "movimiento de caja"))
    .map_err(to_command_error)?;

    let resumen = calcular_resumen_caja(&tx, &sesion).map_err(to_command_error)?;
    let affected = tx
        .execute(
        "UPDATE cajas_sesiones SET monto_esperado = ?1, sincronizado = 0, updated_at = datetime('now') WHERE id = ?2 AND estado = 'ABIERTA'",
        params![resumen.monto_esperado_actual, sesion.id],
    )
    .map_err(AppError::from)
    .map_err(to_command_error)?;
    if affected != 1 {
        return Err("La caja cambió de estado antes de confirmar el movimiento. Intenta de nuevo.".to_string());
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(resumen)
}

#[tauri::command]
fn cerrar_caja(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    cierre: CerrarCajaInput,
) -> AppResult<CajaEstado> {
    let actor = current_session_user(&state_sesion)?;
    validate_cerrar_caja_input(&cierre).map_err(to_command_error)?;
    let monto_final_real = normalize_money(cierre.monto_final_real, "El monto final real", true)
        .map_err(to_command_error)?;
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let sesion = tx
        .query_row(
            "
            SELECT id, usuario_id, sucursal_id, fecha_apertura, monto_inicial, fecha_cierre, monto_final_real, monto_esperado, estado
            FROM cajas_sesiones
            WHERE id = ?1
            ",
            [&cierre.sesion_id],
            |row| {
                Ok(CajaSesion {
                    id: row.get(0)?,
                    usuario_id: row.get(1)?,
                    sucursal_id: row.get(2)?,
                    fecha_apertura: row.get(3)?,
                    monto_inicial: row.get(4)?,
                    fecha_cierre: row.get(5)?,
                    monto_final_real: row.get(6)?,
                    monto_esperado: row.get(7)?,
                    estado: row.get(8)?,
                })
            },
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if sesion.estado != "ABIERTA" {
        return Err("La caja seleccionada ya está cerrada.".to_string());
    }
    if sesion.usuario_id != actor.id || sesion.sucursal_id != actor.sucursal_id {
        return Err("No puedes cerrar una caja que no pertenece a tu sesión.".to_string());
    }

    let resumen = calcular_resumen_caja(&tx, &sesion).map_err(to_command_error)?;
    if resumen.monto_esperado_actual > 0.0 && monto_final_real <= 0.0 {
        return Err("No puedes cerrar la caja en $0.00 cuando el monto esperado es mayor a cero.".to_string());
    }

    let affected = tx.execute(
        "
        UPDATE cajas_sesiones
        SET fecha_cierre = ?1,
            monto_final_real = ?2,
            monto_esperado = ?3,
            estado = 'CERRADA',
            sincronizado = 0,
            updated_at = datetime('now')
        WHERE id = ?4 AND estado = 'ABIERTA'
        ",
        params![
            cierre.fecha_cierre,
            monto_final_real,
            resumen.monto_esperado_actual,
            cierre.sesion_id
        ],
    )
    .map_err(|error| map_write_error(error, "cierre de caja"))
    .map_err(to_command_error)?;
    if affected != 1 {
        return Err("La caja ya no está ABIERTA. No se aplicó el cierre.".to_string());
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;

    Ok(CajaEstado {
        sesion: CajaSesion {
            fecha_cierre: Some(cierre.fecha_cierre),
            monto_final_real: Some(monto_final_real),
            monto_esperado: resumen.monto_esperado_actual,
            estado: "CERRADA".to_string(),
            ..sesion
        },
        ..resumen
    })
}

fn cp850_byte(ch: char) -> u8 {
    match ch {
        'á' => 0xA0,
        'é' => 0x82,
        'í' => 0xA1,
        'ó' => 0xA2,
        'ú' => 0xA3,
        'Á' => 0xB5,
        'É' => 0x90,
        'Í' => 0xD6,
        'Ó' => 0xE0,
        'Ú' => 0xE9,
        'ñ' => 0xA4,
        'Ñ' => 0xA5,
        'ü' => 0x81,
        'Ü' => 0x9A,
        '¿' => 0xA8,
        '¡' => 0xAD,
        '°' => 0xF8,
        'ç' => 0x87,
        'Ç' => 0x80,
        '$' => b'$',
        ch if ch.is_ascii() => ch as u8,
        _ => b' ',
    }
}

fn encode_cp850(text: &str) -> Vec<u8> {
    text.chars().map(cp850_byte).collect()
}

fn escpos_text_line(buffer: &mut Vec<u8>, text: &str) {
    buffer.extend_from_slice(&encode_cp850(text));
    buffer.push(b'\n');
}

fn money_text(value: f64) -> String {
    format!("${:.2}", round_money(value))
}

fn fit_text(value: &str, width: usize) -> String {
    let mut text = value.trim().replace('\n', " ");
    if text.chars().count() > width {
        text = text.chars().take(width.saturating_sub(3)).collect::<String>();
        text.push_str("...");
    }
    text
}

fn ticket_two_columns(left: &str, right: &str, width: usize) -> String {
    let right_len = right.chars().count();
    let max_left = width.saturating_sub(right_len + 1);
    let left = fit_text(left, max_left);
    let left_len = left.chars().count();
    let spaces = width.saturating_sub(left_len + right_len);
    format!("{left}{}{right}", " ".repeat(spaces))
}

fn optional_ticket_text(value: &Option<String>, fallback: &str) -> String {
    value
        .as_ref()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn build_ticket_escpos(ticket: &TicketPayloadInput, paper_width: usize, abrir_cajon: bool) -> Result<Vec<u8>, String> {
    let width = match paper_width {
        32 | 42 | 48 => paper_width,
        _ => return Err("El ancho de papel debe ser 32, 42 o 48 caracteres.".to_string()),
    };
    if ticket.productos.is_empty() {
        return Err("El ticket no contiene productos.".to_string());
    }

    let separator = "-".repeat(width);
    let mut buffer = Vec::new();

    if abrir_cajon {
        // Open the cash drawer connected to the printer: ESC p m t1 t2.
        buffer.extend_from_slice(&[0x1B, 0x70, 0x00, 0x19, 0xFA]);
    }
    // Initialize printer.
    buffer.extend_from_slice(&[0x1B, 0x40]);
    // CP850 gives reliable accents/ñ on most ESC/POS printers configured for Latin America.
    buffer.extend_from_slice(&[0x1B, 0x74, 0x02]);
    // Double-strike improves apparent density on many thermal heads.
    buffer.extend_from_slice(&[0x1B, 0x47, 0x01]);
    buffer.extend_from_slice(&[0x1B, 0x21, 0x00]);
    let _subtotal_recibido = ticket.subtotal;
    let subtotal_iva_incluido = round_money(ticket.total / 1.16);
    let iva_incluido = round_money(ticket.total - subtotal_iva_incluido);
    let metodo_pago = ticket.metodo_pago.trim().to_uppercase();

    // Centered header.
    buffer.extend_from_slice(&[0x1B, 0x61, 0x01]);
    if let Some(logo_bytes) = &ticket.logo_bytes {
        if !logo_bytes.is_empty() {
            buffer.extend_from_slice(logo_bytes);
            buffer.push(b'\n');
        }
    }
    escpos_text_line(
        &mut buffer,
        &fit_text(&optional_ticket_text(&ticket.empresa_nombre, "FERRE-POS"), width),
    );
    escpos_text_line(
        &mut buffer,
        &fit_text(&format!("RFC: {}", optional_ticket_text(&ticket.rfc, "SIN RFC CONFIGURADO")), width),
    );
    escpos_text_line(
        &mut buffer,
        &fit_text(
            &format!(
                "Regimen Fiscal: {}",
                optional_ticket_text(&ticket.regimen_fiscal, "N/D")
            ),
            width,
        ),
    );
    escpos_text_line(
        &mut buffer,
        &fit_text(&format!("C.P.: {}", optional_ticket_text(&ticket.codigo_postal, "N/D")), width),
    );
    escpos_text_line(&mut buffer, &fit_text(&ticket.sucursal, width));
    escpos_text_line(&mut buffer, &fit_text(&format!("FOLIO: {}", ticket.folio), width));
    escpos_text_line(&mut buffer, &separator);

    // Left-aligned body.
    buffer.extend_from_slice(&[0x1B, 0x61, 0x00]);
    escpos_text_line(&mut buffer, &ticket_two_columns("Fecha", &ticket.fecha, width));
    escpos_text_line(&mut buffer, &ticket_two_columns("Cajero", &ticket.cajero, width));
    escpos_text_line(&mut buffer, &ticket_two_columns("Pago", &metodo_pago, width));
    if let Some(estado) = ticket.estado.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
        escpos_text_line(&mut buffer, &ticket_two_columns("Estado", estado, width));
    }
    escpos_text_line(&mut buffer, &separator);

    for producto in &ticket.productos {
        escpos_text_line(&mut buffer, &fit_text(&producto.descripcion, width));
        if let Some(marca) = producto.marca.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
            escpos_text_line(&mut buffer, &fit_text(marca, width));
        }
        let cantidad_precio = format!("{:.3} x {}", producto.cantidad, money_text(producto.precio_unitario));
        escpos_text_line(
            &mut buffer,
            &ticket_two_columns(&cantidad_precio, &money_text(producto.importe), width),
        );
    }

    escpos_text_line(&mut buffer, &separator);
    escpos_text_line(
        &mut buffer,
        &ticket_two_columns("Subtotal", &money_text(subtotal_iva_incluido), width),
    );
    if ticket.descuento > 0.0 {
        escpos_text_line(&mut buffer, &ticket_two_columns("Descuento", &format!("-{}", money_text(ticket.descuento)), width));
    }
    escpos_text_line(&mut buffer, &ticket_two_columns("IVA 16%", &money_text(iva_incluido), width));
    escpos_text_line(&mut buffer, &ticket_two_columns("TOTAL", &money_text(ticket.total), width));
    if let Some(recibido) = ticket.recibido {
        escpos_text_line(&mut buffer, &ticket_two_columns("Efectivo recibido", &money_text(recibido), width));
    }
    if let Some(cambio) = ticket.cambio {
        escpos_text_line(&mut buffer, &ticket_two_columns("Cambio", &money_text(cambio), width));
    }

    escpos_text_line(&mut buffer, &separator);
    if metodo_pago == "CREDITO" {
        escpos_text_line(&mut buffer, "AVISO DE CREDITO: Este comprobante");
        escpos_text_line(&mut buffer, "expira a los 30 dias naturales de su");
        escpos_text_line(&mut buffer, "expedicion. Pasada la fecha limite de");
        escpos_text_line(&mut buffer, "pago, se aplicara una penalizacion de");
        escpos_text_line(&mut buffer, "interes moratorio mensual sobre saldo.");
        escpos_text_line(&mut buffer, "");
        buffer.extend_from_slice(&[0x1B, 0x61, 0x01]);
        escpos_text_line(&mut buffer, "__________________________________");
        escpos_text_line(&mut buffer, "       Firma de Conformidad");
        buffer.extend_from_slice(&[0x1B, 0x61, 0x00]);
        escpos_text_line(&mut buffer, "");
    }

    buffer.extend_from_slice(&[0x1B, 0x61, 0x01]);
    escpos_text_line(&mut buffer, &fit_text("Este ticket forma parte de la venta", width));
    escpos_text_line(&mut buffer, &fit_text("global del dia.", width));

    if let Some(mensaje) = ticket.mensaje.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
        escpos_text_line(&mut buffer, &fit_text(mensaje, width));
    }
    buffer.extend_from_slice(&[0x1B, 0x61, 0x00]);

    buffer.extend_from_slice(b"\n\n\n");
    // Corte parcial: GS V B 0.
    buffer.extend_from_slice(&[0x1D, 0x56, 0x42, 0x00]);

    Ok(buffer)
}

fn escape_powershell_single(value: &str) -> String {
    value.replace('\'', "''")
}

fn run_command_output(command: &mut Command) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("No se pudo consultar el sistema de impresión: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "El sistema rechazó la consulta de impresoras.".to_string()
        } else {
            stderr
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn system_printers() -> Result<Vec<String>, String> {
    #[cfg(target_os = "windows")]
    let raw = run_command_output(
        Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-Printer | Select-Object -ExpandProperty Name"]),
    )
    .or_else(|_| {
        run_command_output(Command::new("wmic").args(["printer", "get", "name"]))
    })?;

    #[cfg(not(target_os = "windows"))]
    let raw = run_command_output(Command::new("lpstat").arg("-a"))?;

    let mut printers: Vec<String> = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.eq_ignore_ascii_case("Name"))
        .map(|line| {
            #[cfg(not(target_os = "windows"))]
            {
                line.split_whitespace().next().unwrap_or(line).to_string()
            }
            #[cfg(target_os = "windows")]
            {
                line.to_string()
            }
        })
        .collect();
    printers.sort();
    printers.dedup();
    Ok(printers)
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct DocInfo1W {
    p_doc_name: *mut u16,
    p_output_file: *mut u16,
    p_datatype: *mut u16,
}

#[cfg(target_os = "windows")]
#[link(name = "Winspool")]
extern "system" {
    fn OpenPrinterW(
        p_printer_name: *mut u16,
        ph_printer: *mut *mut std::ffi::c_void,
        p_default: *mut std::ffi::c_void,
    ) -> i32;
    fn ClosePrinter(h_printer: *mut std::ffi::c_void) -> i32;
    fn StartDocPrinterW(
        h_printer: *mut std::ffi::c_void,
        level: u32,
        p_doc_info: *mut std::ffi::c_void,
    ) -> u32;
    fn EndDocPrinter(h_printer: *mut std::ffi::c_void) -> i32;
    fn StartPagePrinter(h_printer: *mut std::ffi::c_void) -> i32;
    fn EndPagePrinter(h_printer: *mut std::ffi::c_void) -> i32;
    fn WritePrinter(
        h_printer: *mut std::ffi::c_void,
        p_buf: *mut std::ffi::c_void,
        cb_buf: u32,
        pc_written: *mut u32,
    ) -> i32;
}

#[cfg(target_os = "windows")]
fn send_raw_to_windows_printer(printer_name: &str, data: &[u8]) -> Result<(), String> {
    if data.is_empty() {
        return Err("No hay contenido RAW para imprimir.".to_string());
    }

    let mut printer = wide_null(printer_name);
    let mut doc_name = wide_null("Ferre-POS Ticket");
    let mut datatype = wide_null("RAW");
    let mut handle: *mut std::ffi::c_void = std::ptr::null_mut();

    unsafe {
        if OpenPrinterW(printer.as_mut_ptr(), &mut handle, std::ptr::null_mut()) == 0 || handle.is_null() {
            return Err(format!(
                "La impresora {printer_name} no está conectada o no se encuentra disponible."
            ));
        }

        let mut doc_info = DocInfo1W {
            p_doc_name: doc_name.as_mut_ptr(),
            p_output_file: std::ptr::null_mut(),
            p_datatype: datatype.as_mut_ptr(),
        };

        let doc_started = StartDocPrinterW(handle, 1, &mut doc_info as *mut _ as *mut std::ffi::c_void);
        if doc_started == 0 {
            ClosePrinter(handle);
            return Err("Windows rechazó el documento RAW de impresión.".to_string());
        }

        if StartPagePrinter(handle) == 0 {
            EndDocPrinter(handle);
            ClosePrinter(handle);
            return Err("Windows no pudo iniciar la página RAW de impresión.".to_string());
        }

        let mut written = 0u32;
        let ok = WritePrinter(
            handle,
            data.as_ptr() as *mut std::ffi::c_void,
            data.len() as u32,
            &mut written,
        );
        EndPagePrinter(handle);
        EndDocPrinter(handle);
        ClosePrinter(handle);

        if ok == 0 || written != data.len() as u32 {
            return Err("Windows no pudo enviar todos los bytes RAW a la impresora.".to_string());
        }
    }

    Ok(())
}

fn send_raw_to_printer(printer_name: &str, data: &[u8]) -> Result<(), String> {
    let printer_name = printer_name.trim();
    if printer_name.is_empty() {
        return Err("Debes indicar el nombre o ruta de la impresora.".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        send_raw_to_windows_printer(printer_name, data)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_millis();
        let mut path = std::env::temp_dir();
        path.push(format!("ferre_pos_ticket_{millis}.bin"));
        {
            let mut file = File::create(&path).map_err(|error| format!("No se pudo crear el archivo temporal del ticket: {error}"))?;
            file.write_all(data)
                .map_err(|error| format!("No se pudo escribir el ticket temporal: {error}"))?;
        }

        let output = Command::new("lp")
            .args(["-d", printer_name, "-o", "raw"])
            .arg(&path)
            .output();

        let _ = fs::remove_file(&path);

        let output = output.map_err(|error| format!("No se pudo invocar el spooler de impresión: {error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Err(format!(
                "La impresora rechazó el ticket. {}{}",
                stdout,
                if stderr.is_empty() { String::new() } else { format!(" {stderr}") }
            ))
        }
    }
}

fn send_text_to_system_printer(printer_name: &str, contenido: &str) -> Result<(), String> {
    let printer_name = printer_name.trim();
    if printer_name.is_empty() {
        return Err("Debes indicar el nombre de la impresora.".to_string());
    }
    if contenido.trim().is_empty() {
        return Err("No hay contenido para imprimir.".to_string());
    }

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis();
    let mut path = std::env::temp_dir();
    path.push(format!("ferre_pos_print_{millis}.txt"));
    {
        let mut file = File::create(&path).map_err(|error| format!("No se pudo crear el archivo temporal de impresión: {error}"))?;
        file.write_all(contenido.as_bytes())
            .map_err(|error| format!("No se pudo escribir el archivo temporal de impresión: {error}"))?;
    }

    #[cfg(target_os = "windows")]
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Get-Content -Raw -LiteralPath '{}' | Out-Printer -Name '{}'",
                escape_powershell_single(&path.to_string_lossy()),
                escape_powershell_single(printer_name)
            ),
        ])
        .output();

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("lp")
        .args(["-d", printer_name])
        .arg(&path)
        .output();

    let _ = fs::remove_file(&path);

    let output = output.map_err(|error| format!("No se pudo invocar el spooler de impresión: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err(format!(
            "La impresora {printer_name} no está conectada o no se encuentra disponible. {}{}",
            stdout,
            if stderr.is_empty() { String::new() } else { format!(" {stderr}") }
        ))
    }
}

#[tauri::command]
fn get_system_printers(state_sesion: tauri::State<SesionActual>) -> AppResult<Vec<String>> {
    require_superadmin(&state_sesion)?;
    system_printers()
}

#[tauri::command]
fn get_perifericos_config(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<PerifericosConfig> {
    current_session_user(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let config = conn
        .query_row(
            "
            SELECT impresora_tickets, impresora_etiquetas, updated_at
            FROM perifericos_config
            WHERE id = 1
            ",
            [],
            |row| {
                Ok(PerifericosConfig {
                    impresora_tickets: row.get(0)?,
                    impresora_etiquetas: row.get(1)?,
                    updated_at: row.get(2)?,
                })
            },
        )
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    Ok(config.unwrap_or(PerifericosConfig {
        impresora_tickets: String::new(),
        impresora_etiquetas: String::new(),
        updated_at: String::new(),
    }))
}

#[tauri::command]
fn guardar_perifericos_config(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    config: PerifericosConfigInput,
) -> AppResult<PerifericosConfig> {
    require_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let impresora_tickets = config.impresora_tickets.trim().to_string();
    let impresora_etiquetas = config.impresora_etiquetas.trim().to_string();
    conn.execute(
        "
        INSERT INTO perifericos_config (id, impresora_tickets, impresora_etiquetas, updated_at)
        VALUES (1, ?1, ?2, datetime('now'))
        ON CONFLICT(id) DO UPDATE SET
            impresora_tickets = excluded.impresora_tickets,
            impresora_etiquetas = excluded.impresora_etiquetas,
            updated_at = datetime('now')
        ",
        params![impresora_tickets, impresora_etiquetas],
    )
    .map_err(AppError::from)
    .map_err(to_command_error)?;

    get_perifericos_config(state_db, state_sesion)
}

#[tauri::command]
fn imprimir_silencioso(
    state_sesion: tauri::State<SesionActual>,
    input: SilentPrintInput,
) -> AppResult<()> {
    current_session_user(&state_sesion)?;
    send_text_to_system_printer(&input.printer_name, &input.contenido)
}

#[tauri::command]
fn imprimir_ticket_y_abrir_caja(
    state_sesion: tauri::State<SesionActual>,
    printer_name: String,
    paper_width: Option<usize>,
    abrir_cajon: Option<bool>,
    ticket: TicketPayloadInput,
) -> AppResult<()> {
    current_session_user(&state_sesion)?;
    let width = paper_width.unwrap_or(42);
    let bytes = build_ticket_escpos(&ticket, width, abrir_cajon.unwrap_or(true))?;
    send_raw_to_printer(&printer_name, &bytes)
}

#[tauri::command]
fn get_dashboard_stats(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    filtro: Option<DashboardFiltroInput>,
) -> AppResult<DashboardStats> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = require_admin_or_superadmin(&state_sesion)?;
    let filtro_ref = filtro.as_ref();
    let sucursal_id = scoped_sucursal_for_read(
        &user,
        filtro_ref.and_then(|f| normalize_filter(&f.sucursal_id)),
    );
    let fecha_inicio = filtro_ref.and_then(|f| normalize_filter(&f.fecha_inicio));
    let fecha_fin = filtro_ref.and_then(|f| normalize_filter(&f.fecha_fin));

    let (total_vendido, utilidad_neta, transacciones) = if sucursal_id.is_none()
        && fecha_inicio.is_none()
        && fecha_fin.is_none()
    {
        let total: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(total), 0) FROM ventas WHERE estado = 'COMPLETADA' AND DATE(fecha) = DATE('now', 'localtime')",
                [],
                |row| row.get(0),
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;

        let utilidad: f64 = conn
            .query_row(
                "
                SELECT COALESCE(SUM((dv.precio_venta_pactado - COALESCE(NULLIF(dv.costo_unitario_pactado, 0), p.precio_costo)) * dv.cantidad), 0)
                FROM detalle_ventas dv
                INNER JOIN ventas v ON v.id = dv.venta_id
                INNER JOIN productos p ON p.id = dv.producto_id
                WHERE v.estado = 'COMPLETADA' AND DATE(v.fecha) = DATE('now', 'localtime')
                ",
                [],
                |row| row.get(0),
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;

        let trx: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ventas WHERE estado = 'COMPLETADA' AND DATE(fecha) = DATE('now', 'localtime')",
                [],
                |row| row.get(0),
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        (total, utilidad, trx)
    } else if sucursal_id.is_some() && fecha_inicio.is_none() && fecha_fin.is_none() {
        let sid = sucursal_id.clone().unwrap_or_default();
        let total: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(total), 0) FROM ventas WHERE estado = 'COMPLETADA' AND sucursal_id = ?1 AND DATE(fecha) = DATE('now', 'localtime')",
                [&sid],
                |row| row.get(0),
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        let utilidad: f64 = conn
            .query_row(
                "
                SELECT COALESCE(SUM((dv.precio_venta_pactado - COALESCE(NULLIF(dv.costo_unitario_pactado, 0), p.precio_costo)) * dv.cantidad), 0)
                FROM detalle_ventas dv
                INNER JOIN ventas v ON v.id = dv.venta_id
                INNER JOIN productos p ON p.id = dv.producto_id
                WHERE v.estado = 'COMPLETADA' AND v.sucursal_id = ?1 AND DATE(v.fecha) = DATE('now', 'localtime')
                ",
                [&sid],
                |row| row.get(0),
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        let trx: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ventas WHERE estado = 'COMPLETADA' AND sucursal_id = ?1 AND DATE(fecha) = DATE('now', 'localtime')",
                [&sid],
                |row| row.get(0),
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        (total, utilidad, trx)
    } else {
        let sid = sucursal_id.unwrap_or_default();
        let fi = fecha_inicio.unwrap_or_else(|| "0001-01-01T00:00:00.000Z".to_string());
        let ff = fecha_fin.unwrap_or_else(|| "9999-12-31T23:59:59.999Z".to_string());

        if sid.is_empty() {
            let total: f64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(total), 0) FROM ventas WHERE estado = 'COMPLETADA' AND fecha >= ?1 AND fecha <= ?2",
                    params![fi, ff],
                    |row| row.get(0),
                )
                .map_err(AppError::from)
                .map_err(to_command_error)?;
            let utilidad: f64 = conn
                .query_row(
                    "
                    SELECT COALESCE(SUM((dv.precio_venta_pactado - COALESCE(NULLIF(dv.costo_unitario_pactado, 0), p.precio_costo)) * dv.cantidad), 0)
                    FROM detalle_ventas dv
                    INNER JOIN ventas v ON v.id = dv.venta_id
                    INNER JOIN productos p ON p.id = dv.producto_id
                    WHERE v.estado = 'COMPLETADA' AND v.fecha >= ?1 AND v.fecha <= ?2
                    ",
                    params![fi, ff],
                    |row| row.get(0),
                )
                .map_err(AppError::from)
                .map_err(to_command_error)?;
            let trx: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM ventas WHERE estado = 'COMPLETADA' AND fecha >= ?1 AND fecha <= ?2",
                    params![fi, ff],
                    |row| row.get(0),
                )
                .map_err(AppError::from)
                .map_err(to_command_error)?;
            (total, utilidad, trx)
        } else {
            let total: f64 = conn
                .query_row(
                    "SELECT COALESCE(SUM(total), 0) FROM ventas WHERE estado = 'COMPLETADA' AND sucursal_id = ?1 AND fecha >= ?2 AND fecha <= ?3",
                    params![sid, fi, ff],
                    |row| row.get(0),
                )
                .map_err(AppError::from)
                .map_err(to_command_error)?;
            let utilidad: f64 = conn
                .query_row(
                    "
                    SELECT COALESCE(SUM((dv.precio_venta_pactado - COALESCE(NULLIF(dv.costo_unitario_pactado, 0), p.precio_costo)) * dv.cantidad), 0)
                    FROM detalle_ventas dv
                    INNER JOIN ventas v ON v.id = dv.venta_id
                    INNER JOIN productos p ON p.id = dv.producto_id
                    WHERE v.estado = 'COMPLETADA' AND v.sucursal_id = ?1 AND v.fecha >= ?2 AND v.fecha <= ?3
                    ",
                    params![sid, fi, ff],
                    |row| row.get(0),
                )
                .map_err(AppError::from)
                .map_err(to_command_error)?;
            let trx: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM ventas WHERE estado = 'COMPLETADA' AND sucursal_id = ?1 AND fecha >= ?2 AND fecha <= ?3",
                    params![sid, fi, ff],
                    |row| row.get(0),
                )
                .map_err(AppError::from)
                .map_err(to_command_error)?;
            (total, utilidad, trx)
        }
    };

    let ticket_promedio = if transacciones > 0 {
        round_money(total_vendido / transacciones as f64)
    } else {
        0.0
    };
    let margen_porcentaje = if total_vendido > 0.0 {
        round_money((utilidad_neta / total_vendido) * 100.0)
    } else {
        0.0
    };

    Ok(DashboardStats {
        total_vendido: round_money(total_vendido),
        utilidad_neta: round_money(utilidad_neta),
        transacciones,
        ticket_promedio,
        margen_porcentaje,
    })
}

#[tauri::command]
fn get_productos_bajo_stock(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    sucursal_id: Option<String>,
) -> AppResult<Vec<ProductoBajoStock>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = require_admin_or_superadmin(&state_sesion)?;
    let page = query_productos_bajo_stock(&conn, &user, normalize_filter(&sucursal_id), 0, 50)?;
    Ok(page.rows)
}

#[tauri::command]
fn get_productos_bajo_stock_page(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    sucursal_id: Option<String>,
    page: i64,
    page_size: i64,
) -> AppResult<ProductosBajoStockPage> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = require_admin_or_superadmin(&state_sesion)?;
    query_productos_bajo_stock(&conn, &user, normalize_filter(&sucursal_id), page, page_size)
}

#[tauri::command]
fn get_productos_mas_vendidos(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    filtro: Option<DashboardFiltroInput>,
) -> AppResult<Vec<ProductoMasVendido>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = require_admin_or_superadmin(&state_sesion)?;
    let filtro_ref = filtro.as_ref();
    let sucursal_id = scoped_sucursal_for_read(
        &user,
        filtro_ref.and_then(|f| normalize_filter(&f.sucursal_id)),
    );
    let fecha_inicio = filtro_ref.and_then(|f| normalize_filter(&f.fecha_inicio));
    let fecha_fin = filtro_ref.and_then(|f| normalize_filter(&f.fecha_fin));
    let fi = fecha_inicio.unwrap_or_else(|| "0001-01-01T00:00:00.000Z".to_string());
    let ff = fecha_fin.unwrap_or_else(|| "9999-12-31T23:59:59.999Z".to_string());

    let mut resultados = Vec::new();
    if let Some(sid) = sucursal_id {
        let mut stmt = conn
            .prepare(
                "
                SELECT p.id, p.descripcion, p.marca, COALESCE(SUM(dv.cantidad), 0) AS unidades
                FROM detalle_ventas dv
                INNER JOIN ventas v ON v.id = dv.venta_id
                INNER JOIN productos p ON p.id = dv.producto_id
                WHERE v.estado = 'COMPLETADA' AND v.sucursal_id = ?1 AND v.fecha >= ?2 AND v.fecha <= ?3
                GROUP BY p.id, p.descripcion, p.marca
                ORDER BY unidades DESC
                LIMIT 5
                ",
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        let iter = stmt
            .query_map(params![sid, fi, ff], |row| {
                Ok(ProductoMasVendido {
                    producto_id: row.get(0)?,
                    descripcion: row.get(1)?,
                    marca: row.get(2)?,
                    unidades_vendidas: row.get(3)?,
                })
            })
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        for item in iter {
            resultados.push(item.map_err(AppError::from).map_err(to_command_error)?);
        }
    } else {
        let mut stmt = conn
            .prepare(
                "
                SELECT p.id, p.descripcion, p.marca, COALESCE(SUM(dv.cantidad), 0) AS unidades
                FROM detalle_ventas dv
                INNER JOIN ventas v ON v.id = dv.venta_id
                INNER JOIN productos p ON p.id = dv.producto_id
                WHERE v.estado = 'COMPLETADA' AND v.fecha >= ?1 AND v.fecha <= ?2
                GROUP BY p.id, p.descripcion, p.marca
                ORDER BY unidades DESC
                LIMIT 5
                ",
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        let iter = stmt
            .query_map(params![fi, ff], |row| {
                Ok(ProductoMasVendido {
                    producto_id: row.get(0)?,
                    descripcion: row.get(1)?,
                    marca: row.get(2)?,
                    unidades_vendidas: row.get(3)?,
                })
            })
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        for item in iter {
            resultados.push(item.map_err(AppError::from).map_err(to_command_error)?);
        }
    }

    Ok(resultados)
}

fn indicador_scope(
    state_sesion: &tauri::State<SesionActual>,
    filtro: Option<&DashboardFiltroInput>,
) -> Result<(Option<String>, String, String), String> {
    let user = require_admin_or_superadmin(state_sesion)?;
    let sucursal_id = scoped_sucursal_for_read(
        &user,
        filtro.and_then(|f| normalize_filter(&f.sucursal_id)),
    );
    let fecha_inicio = filtro
        .and_then(|f| normalize_filter(&f.fecha_inicio))
        .unwrap_or_else(|| "0001-01-01T00:00:00.000Z".to_string());
    let fecha_fin = filtro
        .and_then(|f| normalize_filter(&f.fecha_fin))
        .unwrap_or_else(|| "9999-12-31T23:59:59.999Z".to_string());
    Ok((sucursal_id, fecha_inicio, fecha_fin))
}

#[tauri::command]
fn get_rentabilidad(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    filtro: Option<DashboardFiltroInput>,
) -> AppResult<RentabilidadResumen> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let (sucursal_id, fi, ff) = indicador_scope(&state_sesion, filtro.as_ref())?;
    let filtro_ref = filtro.as_ref();
    let marca = filtro_ref.and_then(|f| normalize_filter(&f.marca));
    let categoria = filtro_ref.and_then(|f| normalize_filter(&f.categoria));
    let proveedor_id = filtro_ref.and_then(|f| normalize_filter(&f.proveedor_id));
    let metodo_pago = filtro_ref.and_then(|f| normalize_filter(&f.metodo_pago));
    let usuario_id = filtro_ref.and_then(|f| normalize_filter(&f.usuario_id));

    let (venta_total, costo_total): (f64, f64) = if let Some(sid) = sucursal_id.clone() {
        conn.query_row(
            "
            SELECT COALESCE(SUM(dv.cantidad * dv.precio_venta_pactado), 0),
                   COALESCE(SUM(dv.cantidad * dv.costo_unitario_pactado), 0)
            FROM detalle_ventas dv
            INNER JOIN ventas v ON v.id = dv.venta_id
            INNER JOIN productos p ON p.id = dv.producto_id
            WHERE v.estado = 'COMPLETADA' AND v.sucursal_id = ?1 AND v.fecha >= ?2 AND v.fecha <= ?3
              AND (?4 IS NULL OR p.marca = ?4)
              AND (?5 IS NULL OR p.categoria = ?5)
              AND (?6 IS NULL OR p.proveedor_id = ?6)
              AND (?7 IS NULL OR v.metodo_pago = ?7)
              AND (?8 IS NULL OR v.usuario_id = ?8)
            ",
            params![sid, fi, ff, marca, categoria, proveedor_id, metodo_pago, usuario_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    } else {
        conn.query_row(
            "
            SELECT COALESCE(SUM(dv.cantidad * dv.precio_venta_pactado), 0),
                   COALESCE(SUM(dv.cantidad * dv.costo_unitario_pactado), 0)
            FROM detalle_ventas dv
            INNER JOIN ventas v ON v.id = dv.venta_id
            INNER JOIN productos p ON p.id = dv.producto_id
            WHERE v.estado = 'COMPLETADA' AND v.fecha >= ?1 AND v.fecha <= ?2
              AND (?3 IS NULL OR p.marca = ?3)
              AND (?4 IS NULL OR p.categoria = ?4)
              AND (?5 IS NULL OR p.proveedor_id = ?5)
              AND (?6 IS NULL OR v.metodo_pago = ?6)
              AND (?7 IS NULL OR v.usuario_id = ?7)
            ",
            params![fi, ff, marca, categoria, proveedor_id, metodo_pago, usuario_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    }
    .map_err(AppError::from)
    .map_err(to_command_error)?;

    let utilidad = round_money(venta_total - costo_total);
    let margen = if venta_total > 0.0 {
        round_money((utilidad / venta_total) * 100.0)
    } else {
        0.0
    };

    let mut productos = Vec::new();
    if let Some(sid) = sucursal_id {
        let mut stmt = conn
            .prepare(
                "
                SELECT p.id, p.descripcion, p.marca,
                       COALESCE(SUM(dv.cantidad), 0) AS unidades,
                       COALESCE(SUM(dv.cantidad * dv.precio_venta_pactado), 0) AS venta,
                       COALESCE(SUM(dv.cantidad * dv.costo_unitario_pactado), 0) AS costo
                FROM detalle_ventas dv
                INNER JOIN ventas v ON v.id = dv.venta_id
                INNER JOIN productos p ON p.id = dv.producto_id
                WHERE v.estado = 'COMPLETADA' AND v.sucursal_id = ?1 AND v.fecha >= ?2 AND v.fecha <= ?3
                  AND (?4 IS NULL OR p.marca = ?4)
                  AND (?5 IS NULL OR p.categoria = ?5)
                  AND (?6 IS NULL OR p.proveedor_id = ?6)
                  AND (?7 IS NULL OR v.metodo_pago = ?7)
                  AND (?8 IS NULL OR v.usuario_id = ?8)
                GROUP BY p.id, p.descripcion, p.marca
                ORDER BY (venta - costo) DESC
                LIMIT 50
                ",
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        let iter = stmt
            .query_map(params![sid, fi, ff, marca, categoria, proveedor_id, metodo_pago, usuario_id], map_rentabilidad_producto)
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        for item in iter {
            productos.push(item.map_err(AppError::from).map_err(to_command_error)?);
        }
    } else {
        let mut stmt = conn
            .prepare(
                "
                SELECT p.id, p.descripcion, p.marca,
                       COALESCE(SUM(dv.cantidad), 0) AS unidades,
                       COALESCE(SUM(dv.cantidad * dv.precio_venta_pactado), 0) AS venta,
                       COALESCE(SUM(dv.cantidad * dv.costo_unitario_pactado), 0) AS costo
                FROM detalle_ventas dv
                INNER JOIN ventas v ON v.id = dv.venta_id
                INNER JOIN productos p ON p.id = dv.producto_id
                WHERE v.estado = 'COMPLETADA' AND v.fecha >= ?1 AND v.fecha <= ?2
                  AND (?3 IS NULL OR p.marca = ?3)
                  AND (?4 IS NULL OR p.categoria = ?4)
                  AND (?5 IS NULL OR p.proveedor_id = ?5)
                  AND (?6 IS NULL OR v.metodo_pago = ?6)
                  AND (?7 IS NULL OR v.usuario_id = ?7)
                GROUP BY p.id, p.descripcion, p.marca
                ORDER BY (venta - costo) DESC
                LIMIT 50
                ",
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        let iter = stmt
            .query_map(params![fi, ff, marca, categoria, proveedor_id, metodo_pago, usuario_id], map_rentabilidad_producto)
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        for item in iter {
            productos.push(item.map_err(AppError::from).map_err(to_command_error)?);
        }
    }

    Ok(RentabilidadResumen {
        venta_total: round_money(venta_total),
        costo_total: round_money(costo_total),
        utilidad_bruta: utilidad,
        margen_porcentaje: margen,
        productos,
    })
}

fn map_rentabilidad_producto(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProductoRentabilidad> {
    let venta: f64 = row.get(4)?;
    let costo: f64 = row.get(5)?;
    let utilidad = round_money(venta - costo);
    let margen = if venta > 0.0 {
        round_money((utilidad / venta) * 100.0)
    } else {
        0.0
    };
    Ok(ProductoRentabilidad {
        producto_id: row.get(0)?,
        descripcion: row.get(1)?,
        marca: row.get(2)?,
        unidades: row.get(3)?,
        venta_total: round_money(venta),
        costo_total: round_money(costo),
        utilidad,
        margen_porcentaje: margen,
    })
}

#[tauri::command]
fn get_indicador_ventas(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    filtro: Option<DashboardFiltroInput>,
) -> AppResult<IndicadorVentasResumen> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let (sucursal_id, fi, ff) = indicador_scope(&state_sesion, filtro.as_ref())?;
    let filtro_ref = filtro.as_ref();
    let metodo_pago = filtro_ref.and_then(|f| normalize_filter(&f.metodo_pago));
    let usuario_id = filtro_ref.and_then(|f| normalize_filter(&f.usuario_id));

    let (total_vendido, transacciones, canceladas, ventas_credito): (f64, i64, i64, f64) = if let Some(sid) = sucursal_id.clone() {
        conn.query_row(
            "
            SELECT COALESCE(SUM(CASE WHEN estado = 'COMPLETADA' THEN total ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN estado = 'COMPLETADA' THEN 1 ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN estado = 'CANCELADA' THEN 1 ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN estado = 'COMPLETADA' AND metodo_pago = 'CREDITO' THEN total ELSE 0 END), 0)
            FROM ventas
            WHERE sucursal_id = ?1 AND fecha >= ?2 AND fecha <= ?3
              AND (?4 IS NULL OR metodo_pago = ?4)
              AND (?5 IS NULL OR usuario_id = ?5)
            ",
            params![sid, fi, ff, metodo_pago, usuario_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
    } else {
        conn.query_row(
            "
            SELECT COALESCE(SUM(CASE WHEN estado = 'COMPLETADA' THEN total ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN estado = 'COMPLETADA' THEN 1 ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN estado = 'CANCELADA' THEN 1 ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN estado = 'COMPLETADA' AND metodo_pago = 'CREDITO' THEN total ELSE 0 END), 0)
            FROM ventas
            WHERE fecha >= ?1 AND fecha <= ?2
              AND (?3 IS NULL OR metodo_pago = ?3)
              AND (?4 IS NULL OR usuario_id = ?4)
            ",
            params![fi, ff, metodo_pago, usuario_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
    }
    .map_err(AppError::from)
    .map_err(to_command_error)?;

    let mut metodos = Vec::new();
    if let Some(sid) = sucursal_id.clone() {
        let mut stmt = conn.prepare(
            "SELECT metodo_pago, COALESCE(SUM(total), 0), COUNT(*) FROM ventas WHERE estado = 'COMPLETADA' AND sucursal_id = ?1 AND fecha >= ?2 AND fecha <= ?3 AND (?4 IS NULL OR metodo_pago = ?4) AND (?5 IS NULL OR usuario_id = ?5) GROUP BY metodo_pago ORDER BY 2 DESC",
        ).map_err(AppError::from).map_err(to_command_error)?;
        let iter = stmt.query_map(params![sid, fi.clone(), ff.clone(), metodo_pago, usuario_id], |row| {
            Ok(MetodoPagoResumen { metodo_pago: row.get(0)?, total: round_money(row.get(1)?), transacciones: row.get(2)? })
        }).map_err(AppError::from).map_err(to_command_error)?;
        for item in iter { metodos.push(item.map_err(AppError::from).map_err(to_command_error)?); }
    } else {
        let mut stmt = conn.prepare(
            "SELECT metodo_pago, COALESCE(SUM(total), 0), COUNT(*) FROM ventas WHERE estado = 'COMPLETADA' AND fecha >= ?1 AND fecha <= ?2 AND (?3 IS NULL OR metodo_pago = ?3) AND (?4 IS NULL OR usuario_id = ?4) GROUP BY metodo_pago ORDER BY 2 DESC",
        ).map_err(AppError::from).map_err(to_command_error)?;
        let iter = stmt.query_map(params![fi.clone(), ff.clone(), metodo_pago, usuario_id], |row| {
            Ok(MetodoPagoResumen { metodo_pago: row.get(0)?, total: round_money(row.get(1)?), transacciones: row.get(2)? })
        }).map_err(AppError::from).map_err(to_command_error)?;
        for item in iter { metodos.push(item.map_err(AppError::from).map_err(to_command_error)?); }
    }

    let productos_mas_vendidos = get_productos_mas_vendidos(state_db, state_sesion, filtro)?;
    Ok(IndicadorVentasResumen {
        total_vendido: round_money(total_vendido),
        transacciones,
        ticket_promedio: if transacciones > 0 { round_money(total_vendido / transacciones as f64) } else { 0.0 },
        ventas_canceladas: canceladas,
        ventas_credito: round_money(ventas_credito),
        ventas_contado: round_money(total_vendido - ventas_credito),
        metodos,
        productos_mas_vendidos,
    })
}

#[tauri::command]
fn get_indicador_inventario(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    filtro: Option<DashboardFiltroInput>,
) -> AppResult<IndicadorInventarioResumen> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = require_admin_or_superadmin(&state_sesion)?;
    let filtro_ref = filtro.as_ref();
    let sid = scoped_sucursal_for_read(&user, filtro_ref.and_then(|f| normalize_filter(&f.sucursal_id)));
    let marca = filtro_ref.and_then(|f| normalize_filter(&f.marca));
    let categoria = filtro_ref.and_then(|f| normalize_filter(&f.categoria));
    let proveedor_id = filtro_ref.and_then(|f| normalize_filter(&f.proveedor_id));
    let (productos, valor, stock, bajo, sin_stock, sobre): (i64, f64, f64, i64, i64, i64) = if let Some(sucursal) = sid.clone() {
        conn.query_row(
            "
            SELECT COUNT(*),
                   COALESCE(SUM(i.stock * COALESCE(NULLIF(i.costo_promedio, 0), p.precio_costo, 0)), 0),
                   COALESCE(SUM(i.stock), 0),
                   COALESCE(SUM(CASE WHEN i.stock_minimo > 0 AND i.stock <= i.stock_minimo THEN 1 ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN i.stock <= 0 THEN 1 ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN i.stock_minimo > 0 AND i.stock >= i.stock_minimo * 3 THEN 1 ELSE 0 END), 0)
            FROM inventario_sucursal i
            INNER JOIN productos p ON p.id = i.producto_id
            WHERE i.eliminado = 0 AND p.eliminado = 0 AND i.sucursal_id = ?1
              AND (?2 IS NULL OR p.marca = ?2)
              AND (?3 IS NULL OR p.categoria = ?3)
              AND (?4 IS NULL OR p.proveedor_id = ?4)
            ",
            params![sucursal, marca, categoria, proveedor_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
    } else {
        conn.query_row(
            "
            SELECT COUNT(*),
                   COALESCE(SUM(i.stock * COALESCE(NULLIF(i.costo_promedio, 0), p.precio_costo, 0)), 0),
                   COALESCE(SUM(i.stock), 0),
                   COALESCE(SUM(CASE WHEN i.stock_minimo > 0 AND i.stock <= i.stock_minimo THEN 1 ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN i.stock <= 0 THEN 1 ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN i.stock_minimo > 0 AND i.stock >= i.stock_minimo * 3 THEN 1 ELSE 0 END), 0)
            FROM inventario_sucursal i
            INNER JOIN productos p ON p.id = i.producto_id
            WHERE i.eliminado = 0 AND p.eliminado = 0
              AND (?1 IS NULL OR p.marca = ?1)
              AND (?2 IS NULL OR p.categoria = ?2)
              AND (?3 IS NULL OR p.proveedor_id = ?3)
            ",
            params![marca, categoria, proveedor_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
    }
    .map_err(AppError::from)
    .map_err(to_command_error)?;

    let bajo_stock = get_productos_bajo_stock(state_db, state_sesion, sid)?;
    Ok(IndicadorInventarioResumen {
        productos_en_inventario: productos,
        valor_inventario: round_money(valor),
        stock_total: round_money(stock),
        stock_bajo: bajo,
        sin_stock,
        sobre_stock: sobre,
        bajo_stock,
    })
}

#[tauri::command]
fn get_indicador_financiero(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    filtro: Option<DashboardFiltroInput>,
) -> AppResult<IndicadorFinancieroResumen> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let (sucursal_id, fi, ff) = indicador_scope(&state_sesion, filtro.as_ref())?;
    let filtro_ref = filtro.as_ref();
    let metodo_pago_filter = filtro_ref.and_then(|f| normalize_filter(&f.metodo_pago));
    let usuario_id_filter = filtro_ref.and_then(|f| normalize_filter(&f.usuario_id));
    let venta_por_metodo = |metodo: &str| -> Result<f64, String> {
        if metodo_pago_filter.as_deref().is_some_and(|selected| selected != metodo) {
            return Ok(0.0);
        }
        let value: f64 = if let Some(sid) = sucursal_id.clone() {
            conn.query_row(
                "
                SELECT COALESCE(SUM(total), 0)
                FROM ventas
                WHERE estado = 'COMPLETADA'
                  AND metodo_pago = ?1
                  AND sucursal_id = ?2
                  AND fecha >= ?3
                  AND fecha <= ?4
                  AND (?5 IS NULL OR usuario_id = ?5)
                ",
                params![metodo, sid, fi, ff, usuario_id_filter],
                |row| row.get(0),
            )
        } else {
            conn.query_row(
                "
                SELECT COALESCE(SUM(total), 0)
                FROM ventas
                WHERE estado = 'COMPLETADA'
                  AND metodo_pago = ?1
                  AND fecha >= ?2
                  AND fecha <= ?3
                  AND (?4 IS NULL OR usuario_id = ?4)
                ",
                params![metodo, fi, ff, usuario_id_filter],
                |row| row.get(0),
            )
        }
        .map_err(AppError::from)
        .map_err(to_command_error)?;
        Ok(round_money(value))
    };

    let compras: f64 = if let Some(sid) = sucursal_id.clone() {
        conn.query_row(
            "SELECT COALESCE(SUM(total), 0) FROM compras WHERE sucursal_id = ?1 AND fecha >= ?2 AND fecha <= ?3",
            params![sid, fi, ff],
            |row| row.get(0),
        )
    } else {
        conn.query_row(
            "SELECT COALESCE(SUM(total), 0) FROM compras WHERE fecha >= ?1 AND fecha <= ?2",
            params![fi, ff],
            |row| row.get(0),
        )
    }
    .map_err(AppError::from)
    .map_err(to_command_error)?;

    let (ingresos_caja, egresos_caja): (f64, f64) = if let Some(sid) = sucursal_id.clone() {
        conn.query_row(
            "
            SELECT COALESCE(SUM(CASE WHEN cm.tipo = 'INGRESO' THEN cm.monto ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN cm.tipo = 'EGRESO' THEN cm.monto ELSE 0 END), 0)
            FROM caja_movimientos cm
            INNER JOIN cajas_sesiones cs ON cs.id = cm.sesion_id
            WHERE cs.sucursal_id = ?1 AND cm.updated_at >= ?2 AND cm.updated_at <= ?3
              AND (?4 IS NULL OR cs.usuario_id = ?4)
            ",
            params![sid, fi, ff, usuario_id_filter],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    } else {
        conn.query_row(
            "
            SELECT COALESCE(SUM(CASE WHEN cm.tipo = 'INGRESO' THEN cm.monto ELSE 0 END), 0),
                   COALESCE(SUM(CASE WHEN cm.tipo = 'EGRESO' THEN cm.monto ELSE 0 END), 0)
            FROM caja_movimientos cm
            INNER JOIN cajas_sesiones cs ON cs.id = cm.sesion_id
            WHERE cm.updated_at >= ?1 AND cm.updated_at <= ?2
              AND (?3 IS NULL OR cs.usuario_id = ?3)
            ",
            params![fi, ff, usuario_id_filter],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
    }
    .map_err(AppError::from)
    .map_err(to_command_error)?;

    let cuentas_por_cobrar: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(saldo_deudor), 0) FROM clientes WHERE eliminado = 0",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let ventas_efectivo = venta_por_metodo("EFECTIVO")?;
    let ventas_tarjeta = venta_por_metodo("TARJETA")?;
    let ventas_transferencia = venta_por_metodo("TRANSFERENCIA")?;
    let ventas_credito = venta_por_metodo("CREDITO")?;
    Ok(IndicadorFinancieroResumen {
        ingresos_caja: round_money(ingresos_caja),
        egresos_caja: round_money(egresos_caja),
        ventas_efectivo,
        ventas_tarjeta,
        ventas_transferencia,
        ventas_credito,
        compras: round_money(compras),
        cuentas_por_cobrar: round_money(cuentas_por_cobrar),
        flujo_neto_estimado: round_money(ventas_efectivo + ventas_tarjeta + ventas_transferencia + ingresos_caja - egresos_caja - compras),
    })
}

#[tauri::command]
fn get_historial_ventas(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    filtro: Option<HistorialVentasFiltro>,
) -> AppResult<Vec<HistorialVenta>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
    let filtro_ref = filtro.as_ref();
    let fi = filtro_ref
        .and_then(|f| normalize_filter(&f.fecha_inicio))
        .unwrap_or_else(|| "0001-01-01T00:00:00.000Z".to_string());
    let ff = filtro_ref
        .and_then(|f| normalize_filter(&f.fecha_fin))
        .unwrap_or_else(|| "9999-12-31T23:59:59.999Z".to_string());
    let sid = scoped_sucursal_for_read(
        &user,
        filtro_ref.and_then(|f| normalize_filter(&f.sucursal_id)),
    );
    let uid = filtro_ref.and_then(|f| normalize_filter(&f.usuario_id));
    let estado = filtro_ref.and_then(|f| normalize_filter(&f.estado));

    let mut sql = String::from(
        "
        SELECT
          v.id, v.fecha, v.total, v.metodo_pago, v.efectivo_recibido, v.cambio_entregado, v.estado,
          s.id, s.nombre, u.id, u.nombre, c.id, c.nombre
        FROM ventas v
        INNER JOIN sucursales s ON s.id = v.sucursal_id
        INNER JOIN usuarios u ON u.id = v.usuario_id
        LEFT JOIN clientes c ON c.id = v.cliente_id
        WHERE v.fecha >= ? AND v.fecha <= ?
        ",
    );
    let mut params_vec: Vec<String> = vec![fi, ff];

    if let Some(value) = sid {
        sql.push_str(" AND v.sucursal_id = ?");
        params_vec.push(value);
    }
    if let Some(value) = uid {
        sql.push_str(" AND v.usuario_id = ?");
        params_vec.push(value);
    }
    if let Some(value) = estado {
        sql.push_str(" AND v.estado = ?");
        params_vec.push(value.to_ascii_uppercase());
    }

    sql.push_str(" ORDER BY v.fecha DESC");

    let mut stmt = conn.prepare(&sql).map_err(AppError::from).map_err(to_command_error)?;

    let mut historial = Vec::new();
    let mut rows = stmt
        .query(rusqlite::params_from_iter(params_vec.iter()))
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    while let Some(row) = rows.next().map_err(AppError::from).map_err(to_command_error)? {
        historial.push(HistorialVenta {
            id: row.get(0).map_err(AppError::from).map_err(to_command_error)?,
            fecha: row.get(1).map_err(AppError::from).map_err(to_command_error)?,
            total: row.get(2).map_err(AppError::from).map_err(to_command_error)?,
            metodo_pago: row.get(3).map_err(AppError::from).map_err(to_command_error)?,
            efectivo_recibido: row.get(4).map_err(AppError::from).map_err(to_command_error)?,
            cambio_entregado: row.get(5).map_err(AppError::from).map_err(to_command_error)?,
            estado: row.get(6).map_err(AppError::from).map_err(to_command_error)?,
            sucursal_id: row.get(7).map_err(AppError::from).map_err(to_command_error)?,
            sucursal_nombre: row.get(8).map_err(AppError::from).map_err(to_command_error)?,
            usuario_id: row.get(9).map_err(AppError::from).map_err(to_command_error)?,
            usuario_nombre: row.get(10).map_err(AppError::from).map_err(to_command_error)?,
            cliente_id: row.get(11).ok(),
            cliente_nombre: row.get(12).ok(),
        });
    }

    Ok(historial)
}

#[tauri::command]
fn get_historial_ventas_page(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    filtro: Option<HistorialVentasFiltro>,
    page: i64,
    page_size: i64,
) -> AppResult<HistorialVentasPage> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
    let filtro_ref = filtro.as_ref();
    let fi = filtro_ref
        .and_then(|f| normalize_filter(&f.fecha_inicio))
        .unwrap_or_else(|| "0001-01-01T00:00:00.000Z".to_string());
    let ff = filtro_ref
        .and_then(|f| normalize_filter(&f.fecha_fin))
        .unwrap_or_else(|| "9999-12-31T23:59:59.999Z".to_string());
    let sid = scoped_sucursal_for_read(
        &user,
        filtro_ref.and_then(|f| normalize_filter(&f.sucursal_id)),
    );
    let uid = filtro_ref.and_then(|f| normalize_filter(&f.usuario_id));
    let folio = filtro_ref.and_then(|f| normalize_filter(&f.folio));
    let estado = filtro_ref.and_then(|f| normalize_filter(&f.estado));
    let (page, page_size) = normalize_page_args(page, page_size);
    let offset = page * page_size;

    let mut where_sql = String::from(" WHERE v.fecha >= ? AND v.fecha <= ?");
    let mut params_vec: Vec<String> = vec![fi, ff];

    if let Some(value) = sid {
        where_sql.push_str(" AND v.sucursal_id = ?");
        params_vec.push(value);
    }
    if let Some(value) = uid {
        where_sql.push_str(" AND v.usuario_id = ?");
        params_vec.push(value);
    }
    if let Some(value) = folio {
        where_sql.push_str(" AND v.id LIKE ? COLLATE NOCASE");
        params_vec.push(format!("%{}%", value.trim()));
    }
    if let Some(value) = estado {
        where_sql.push_str(" AND v.estado = ?");
        params_vec.push(value.to_ascii_uppercase());
    }

    let count_sql = format!(
        "
        SELECT COUNT(*)
        FROM ventas v
        INNER JOIN sucursales s ON s.id = v.sucursal_id
        INNER JOIN usuarios u ON u.id = v.usuario_id
        LEFT JOIN clientes c ON c.id = v.cliente_id
        {where_sql}
        "
    );
    let total: i64 = conn
        .query_row(&count_sql, params_from_iter(params_vec.iter()), |row| row.get(0))
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut select_params = params_vec.clone();
    select_params.push(page_size.to_string());
    select_params.push(offset.to_string());

    let select_sql = format!(
        "
        SELECT
          v.id, v.fecha, v.total, v.metodo_pago, v.efectivo_recibido, v.cambio_entregado, v.estado,
          s.id, s.nombre, u.id, u.nombre, c.id, c.nombre
        FROM ventas v
        INNER JOIN sucursales s ON s.id = v.sucursal_id
        INNER JOIN usuarios u ON u.id = v.usuario_id
        LEFT JOIN clientes c ON c.id = v.cliente_id
        {where_sql}
        ORDER BY v.fecha DESC
        LIMIT ? OFFSET ?
        "
    );

    let mut stmt = conn.prepare(&select_sql).map_err(AppError::from).map_err(to_command_error)?;
    let mut rows = stmt
        .query(params_from_iter(select_params.iter()))
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let mut historial = Vec::new();
    while let Some(row) = rows.next().map_err(AppError::from).map_err(to_command_error)? {
        historial.push(HistorialVenta {
            id: row.get(0).map_err(AppError::from).map_err(to_command_error)?,
            fecha: row.get(1).map_err(AppError::from).map_err(to_command_error)?,
            total: row.get(2).map_err(AppError::from).map_err(to_command_error)?,
            metodo_pago: row.get(3).map_err(AppError::from).map_err(to_command_error)?,
            efectivo_recibido: row.get(4).map_err(AppError::from).map_err(to_command_error)?,
            cambio_entregado: row.get(5).map_err(AppError::from).map_err(to_command_error)?,
            estado: row.get(6).map_err(AppError::from).map_err(to_command_error)?,
            sucursal_id: row.get(7).map_err(AppError::from).map_err(to_command_error)?,
            sucursal_nombre: row.get(8).map_err(AppError::from).map_err(to_command_error)?,
            usuario_id: row.get(9).map_err(AppError::from).map_err(to_command_error)?,
            usuario_nombre: row.get(10).map_err(AppError::from).map_err(to_command_error)?,
            cliente_id: row.get(11).ok(),
            cliente_nombre: row.get(12).ok(),
        });
    }

    Ok(HistorialVentasPage { rows: historial, total })
}

#[tauri::command]
fn get_detalle_venta(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    venta_id: String,
) -> AppResult<Vec<HistorialVentaDetalle>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
    let venta_sucursal_id: String = conn
        .query_row(
            "SELECT sucursal_id FROM ventas WHERE id = ?1",
            [&venta_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    ensure_can_read_sucursal(&user, &venta_sucursal_id)?;

    let mut stmt = conn
        .prepare(
            "
            SELECT dv.id, dv.venta_id, dv.producto_id, p.descripcion, p.marca,
                   dv.cantidad, dv.precio_venta_pactado, dv.costo_unitario_pactado
            FROM detalle_ventas dv
            INNER JOIN productos p ON p.id = dv.producto_id
            WHERE dv.venta_id = ?1
            ",
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map([venta_id], |row| {
            Ok(HistorialVentaDetalle {
                id: row.get(0)?,
                venta_id: row.get(1)?,
                producto_id: row.get(2)?,
                descripcion: row.get(3)?,
                marca: row.get(4)?,
                cantidad: row.get(5)?,
                precio_venta_pactado: row.get(6)?,
                costo_unitario_pactado: row.get(7)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut detalle = Vec::new();
    for item in iter {
        detalle.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(detalle)
}

fn round_money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn money_to_cents(value: f64) -> i64 {
    (value * 100.0).round() as i64
}

fn cents_to_money(value: i64) -> f64 {
    value as f64 / 100.0
}

fn validate_sat_concept_keys(clave_prod_serv: &str, clave_unidad: &str, descripcion: &str) -> AppResult<()> {
    let clave_prod_serv = clave_prod_serv.trim();
    let clave_unidad = clave_unidad.trim();
    if clave_prod_serv.len() != 8 || !clave_prod_serv.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!(
            "El producto '{}' debe tener Clave Producto/Servicio SAT de 8 digitos.",
            descripcion
        ));
    }
    if clave_unidad.len() != 3 {
        return Err(format!(
            "El producto '{}' debe tener Clave Unidad SAT de exactamente 3 caracteres.",
            descripcion
        ));
    }
    Ok(())
}

fn cfdi_forma_pago(metodo_pago: &str) -> String {
    match metodo_pago {
        "EFECTIVO" => "01",
        "TARJETA" => "04",
        "TRANSFERENCIA" => "03",
        "CREDITO" => "99",
        _ => "99",
    }
    .to_string()
}

fn cfdi_metodo_pago(metodo_pago: &str) -> String {
    if metodo_pago == "CREDITO" {
        "PPD".to_string()
    } else {
        "PUE".to_string()
    }
}

fn validate_empresa_config_fiscal(input: &EmpresaConfigFiscal) -> Result<(), AppError> {
    if input.rfc.trim().is_empty()
        || input.razon_social.trim().is_empty()
        || input.regimen_fiscal.trim().is_empty()
        || input.actualizado_at.trim().is_empty()
    {
        return Err(AppError::Validation(
            "RFC, razón social, régimen fiscal y fecha de actualización son obligatorios.".to_string(),
        ));
    }
    validate_rfc_sat(&normalize_upper_trim(&input.rfc))?;
    validate_regimen_fiscal_sat(&input.regimen_fiscal.trim())?;
    Ok(())
}

#[tauri::command]
fn get_empresa_config(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Option<EmpresaConfigFiscal>> {
    require_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.query_row(
        "
        SELECT rfc, razon_social, regimen_fiscal, registro_patronal, actualizado_at
        FROM empresa_config_fiscal
        WHERE id = 1
        ",
        [],
        |row| {
            Ok(EmpresaConfigFiscal {
                rfc: row.get(0)?,
                razon_social: row.get(1)?,
                regimen_fiscal: row.get(2)?,
                registro_patronal: row.get(3)?,
                actualizado_at: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(AppError::from)
    .map_err(to_command_error)
}

#[tauri::command]
fn guardar_empresa_config(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    config: EmpresaConfigFiscal,
) -> AppResult<EmpresaConfigFiscal> {
    require_superadmin(&state_sesion)?;
    let config = EmpresaConfigFiscal {
        rfc: normalize_upper_trim(&config.rfc),
        razon_social: config.razon_social.trim().to_string(),
        regimen_fiscal: config.regimen_fiscal.trim().to_string(),
        registro_patronal: config
            .registro_patronal
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
        actualizado_at: config.actualizado_at.trim().to_string(),
    };
    validate_empresa_config_fiscal(&config).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;

    conn.execute(
        "
        INSERT INTO empresa_config_fiscal (
            id, rfc, razon_social, regimen_fiscal, registro_patronal, actualizado_at
        ) VALUES (1, ?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(id) DO UPDATE SET
            rfc = excluded.rfc,
            razon_social = excluded.razon_social,
            regimen_fiscal = excluded.regimen_fiscal,
            registro_patronal = excluded.registro_patronal,
            actualizado_at = excluded.actualizado_at
        ",
        params![
            config.rfc,
            config.razon_social,
            config.regimen_fiscal,
            config.registro_patronal,
            config.actualizado_at
        ],
    )
    .map_err(|error| map_write_error(error, "configuración fiscal de empresa"))
    .map_err(to_command_error)?;

    Ok(EmpresaConfigFiscal {
        rfc: config.rfc,
        razon_social: config.razon_social,
        regimen_fiscal: config.regimen_fiscal,
        registro_patronal: config.registro_patronal,
        actualizado_at: config.actualizado_at,
    })
}

#[tauri::command]
fn get_payload_factura(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    venta_id: String,
) -> AppResult<FacturaPayload> {
    if venta_id.trim().is_empty() {
        return Err("Falta el identificador de la venta a facturar.".to_string());
    }

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = require_admin_or_superadmin(&state_sesion)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let (venta_fecha, metodo_pago, estado, cliente_id, sucursal_id): (
        String,
        String,
        String,
        Option<String>,
        String,
    ) = tx
        .query_row(
            "
            SELECT v.fecha, v.metodo_pago, v.estado, v.cliente_id, v.sucursal_id
            FROM ventas v
            WHERE v.id = ?1
            ",
            [&venta_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    ensure_can_read_sucursal(&user, &sucursal_id)?;

    if estado != "COMPLETADA" {
        return Err("Solo se pueden facturar ventas COMPLETADAS.".to_string());
    }

    let factura_estado_existente: Option<String> = tx
        .query_row(
            "SELECT estado FROM facturas_emitidas WHERE venta_id = ?1 ORDER BY fecha_emision DESC LIMIT 1",
            [&venta_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if matches!(factura_estado_existente.as_deref(), Some("TIMBRADA")) {
        return Err("Esta venta ya tiene una factura TIMBRADA.".to_string());
    }

    let empresa_config = tx
        .query_row(
            "
            SELECT rfc, razon_social, regimen_fiscal, registro_patronal, actualizado_at
            FROM empresa_config_fiscal
            WHERE id = 1
            ",
            [],
            |row| {
                Ok(EmpresaConfigFiscal {
                    rfc: row.get(0)?,
                    razon_social: row.get(1)?,
                    regimen_fiscal: row.get(2)?,
                    registro_patronal: row.get(3)?,
                    actualizado_at: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?
        .ok_or_else(|| "Configura los datos fiscales de la empresa emisora antes de facturar.".to_string())?;

    if empresa_config.rfc.trim().is_empty()
        || empresa_config.razon_social.trim().is_empty()
        || empresa_config.regimen_fiscal.trim().is_empty()
    {
        return Err("La configuración fiscal de la empresa está incompleta.".to_string());
    }

    let lugar_expedicion: String = tx
        .query_row(
            "SELECT codigo_postal FROM sucursales WHERE id = ?1 AND eliminado = 0",
            [&sucursal_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if lugar_expedicion.trim().is_empty() {
        return Err("La sucursal de la venta no tiene código postal para LugarExpedicion.".to_string());
    }

    let cliente_id = cliente_id.ok_or_else(|| {
        "La venta no tiene cliente asociado. Asigna cliente antes de generar CFDI.".to_string()
    })?;

    let receptor: CfdiReceptor = tx
        .query_row(
            "
            SELECT cliente_id, rfc, razon_social, regimen_fiscal, codigo_postal
            FROM clientes_datos_fiscales
            WHERE cliente_id = ?1
            ",
            [&cliente_id],
            |row| {
                Ok(CfdiReceptor {
                    cliente_id: row.get(0)?,
                    rfc: row.get(1)?,
                    nombre: row.get(2)?,
                    regimen_fiscal: row.get(3)?,
                    domicilio_fiscal_receptor: row.get(4)?,
                    uso_cfdi: "G03".to_string(),
                })
            },
        )
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?
        .ok_or_else(|| "El cliente no tiene datos fiscales registrados.".to_string())?;

    let mut stmt = tx
        .prepare(
            "
            SELECT
                dv.producto_id,
                p.clave_producto,
                p.descripcion,
                p.unidad,
                p.sat_clave_prod_serv,
                p.sat_clave_unidad,
                dv.cantidad,
                dv.precio_venta_pactado
            FROM detalle_ventas dv
            INNER JOIN productos p ON p.id = dv.producto_id
            WHERE dv.venta_id = ?1
            ORDER BY p.descripcion
            ",
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map([&venta_id], |row| {
            let producto_id: String = row.get(0)?;
            let no_identificacion: String = row.get(1)?;
            let descripcion: String = row.get(2)?;
            let unidad: String = row.get(3)?;
            let clave_prod_serv: String = row.get(4)?;
            let clave_unidad: String = row.get(5)?;
            let cantidad: f64 = row.get(6)?;
            let precio_neto_unitario: f64 = row.get(7)?;
            let valor_unitario = round_money(precio_neto_unitario / 1.16);
            let importe = round_money(cantidad * valor_unitario);
            let total_linea = round_money(cantidad * precio_neto_unitario);
            let iva = round_money(total_linea - importe);

            Ok(CfdiConcepto {
                producto_id,
                clave_prod_serv,
                no_identificacion,
                cantidad,
                clave_unidad,
                unidad,
                descripcion,
                valor_unitario,
                importe,
                objeto_imp: "02".to_string(),
                impuestos: vec![CfdiImpuestoTraslado {
                    base: importe,
                    impuesto: "002".to_string(),
                    tipo_factor: "Tasa".to_string(),
                    tasa_o_cuota: "0.160000".to_string(),
                    importe: iva,
                }],
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut conceptos = Vec::new();
    for item in iter {
        let concepto = item.map_err(AppError::from).map_err(to_command_error)?;
        validate_sat_concept_keys(
            &concepto.clave_prod_serv,
            &concepto.clave_unidad,
            &concepto.descripcion,
        )?;
        conceptos.push(concepto);
    }
    drop(stmt);

    if conceptos.is_empty() {
        return Err("La venta no tiene conceptos para facturar.".to_string());
    }

    let subtotal = round_money(conceptos.iter().map(|concepto| concepto.importe).sum());
    let total_impuestos_trasladados = round_money(
        conceptos
            .iter()
            .flat_map(|concepto| concepto.impuestos.iter())
            .map(|impuesto| impuesto.importe)
            .sum(),
    );
    let total = round_money(subtotal + total_impuestos_trasladados);

    let payload = FacturaPayload {
        version: "4.0".to_string(),
        serie: "POS".to_string(),
        folio: venta_id.clone(),
        fecha: venta_fecha.clone(),
        moneda: "MXN".to_string(),
        tipo_de_comprobante: "I".to_string(),
        exportacion: "01".to_string(),
        metodo_pago: cfdi_metodo_pago(&metodo_pago),
        forma_pago: cfdi_forma_pago(&metodo_pago),
        subtotal,
        total_impuestos_trasladados,
        total,
        emisor: CfdiEmisor {
            rfc: empresa_config.rfc,
            nombre: empresa_config.razon_social,
            regimen_fiscal: empresa_config.regimen_fiscal,
            lugar_expedicion,
        },
        receptor,
        conceptos,
    };

    tx.execute(
        "
        INSERT OR IGNORE INTO facturas_emitidas (
            id, venta_id, uuid, rfc_receptor, monto_total, estado, fecha_emision, pdf_path, xml_path,
            sync_uuid, sincronizado, updated_at
        ) VALUES (?1, ?2, NULL, ?3, ?4, 'PENDIENTE', ?5, NULL, NULL, ?6, 0, datetime('now'))
        ",
        params![
            format!("FAC-{}", venta_id),
            venta_id,
            payload.receptor.rfc,
            payload.total,
            venta_fecha,
            generate_uuid_like()
        ],
    )
    .map_err(|error| map_write_error(error, "factura"))
    .map_err(to_command_error)?;

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(payload)
}

#[tauri::command]
fn actualizar_estado_factura(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    input: ActualizarEstadoFacturaInput,
) -> AppResult<()> {
    let factura_id = input.factura_id.trim().to_string();
    let uuid = normalize_upper_trim(&input.uuid);
    if factura_id.is_empty() || uuid.is_empty() {
        return Err("La factura y el UUID oficial son obligatorios.".to_string());
    }
    validate_uuid_sat_like(&uuid).map_err(to_command_error)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = require_admin_or_superadmin(&state_sesion)?;
    let (sucursal_id, venta_estado): (String, String) = conn
        .query_row(
            "
            SELECT v.sucursal_id, v.estado
            FROM facturas_emitidas fe
            INNER JOIN ventas v ON v.id = fe.venta_id
            WHERE fe.id = ?1
            ",
            [&factura_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    ensure_can_read_sucursal(&user, &sucursal_id)?;
    if venta_estado != "COMPLETADA" {
        return Err("No se puede timbrar una factura cuya venta ya fue cancelada.".to_string());
    }

    let uuid_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM facturas_emitidas WHERE uuid = ?1 AND id <> ?2",
            params![uuid, factura_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if uuid_exists > 0 {
        return Err("El UUID indicado ya está asignado a otra factura.".to_string());
    }

    let affected = conn
        .execute(
            "
            UPDATE facturas_emitidas
            SET estado = 'TIMBRADA',
                uuid = ?2,
                pdf_path = ?3,
                xml_path = ?4,
                sincronizado = 0,
                updated_at = datetime('now')
            WHERE id = ?1
              AND estado = 'PENDIENTE'
            ",
            params![factura_id, uuid, input.pdf_path, input.xml_path],
        )
        .map_err(|error| map_write_error(error, "factura"))
        .map_err(to_command_error)?;

    if affected == 0 {
        return Err("No se encontró una factura PENDIENTE para timbrar.".to_string());
    }

    Ok(())
}

#[tauri::command]
fn get_facturas_emitidas(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Vec<FacturaEmitida>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = require_admin_or_superadmin(&state_sesion)?;
    let sucursal_id = scoped_sucursal_for_read(&user, None);
    let mut sql = String::from(
        "
            SELECT fe.id, fe.venta_id, fe.uuid, fe.rfc_receptor, fe.monto_total, fe.estado, fe.fecha_emision, fe.pdf_path, fe.xml_path
            FROM facturas_emitidas fe
            INNER JOIN ventas v ON v.id = fe.venta_id
        ",
    );
    let mut params_vec: Vec<String> = Vec::new();
    if let Some(value) = sucursal_id {
        sql.push_str(" WHERE v.sucursal_id = ?1");
        params_vec.push(value);
    }
    sql.push_str(" ORDER BY fe.fecha_emision DESC");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map(params_from_iter(params_vec.iter()), |row| {
            Ok(FacturaEmitida {
                id: row.get(0)?,
                venta_id: row.get(1)?,
                uuid: row.get(2)?,
                rfc_receptor: row.get(3)?,
                monto_total: row.get(4)?,
                estado: row.get(5)?,
                fecha_emision: row.get(6)?,
                pdf_path: row.get(7)?,
                xml_path: row.get(8)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut facturas = Vec::new();
    for item in iter {
        facturas.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(facturas)
}

#[tauri::command]
fn get_facturas_emitidas_page(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    page: i64,
    page_size: i64,
) -> AppResult<FacturasEmitidasPage> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = require_admin_or_superadmin(&state_sesion)?;
    let sucursal_id = scoped_sucursal_for_read(&user, None);
    let (page, page_size) = normalize_page_args(page, page_size);
    let offset = page * page_size;

    let mut where_sql = String::new();
    let mut params_vec: Vec<String> = Vec::new();
    if let Some(value) = sucursal_id {
        where_sql.push_str(" WHERE v.sucursal_id = ?");
        params_vec.push(value);
    }

    let count_sql = format!(
        "
        SELECT COUNT(*)
        FROM facturas_emitidas fe
        INNER JOIN ventas v ON v.id = fe.venta_id
        {where_sql}
        "
    );
    let total: i64 = conn
        .query_row(&count_sql, params_from_iter(params_vec.iter()), |row| row.get(0))
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut select_params = params_vec.clone();
    select_params.push(page_size.to_string());
    select_params.push(offset.to_string());

    let select_sql = format!(
        "
        SELECT fe.id, fe.venta_id, fe.uuid, fe.rfc_receptor, fe.monto_total, fe.estado, fe.fecha_emision, fe.pdf_path, fe.xml_path
        FROM facturas_emitidas fe
        INNER JOIN ventas v ON v.id = fe.venta_id
        {where_sql}
        ORDER BY fe.fecha_emision DESC
        LIMIT ? OFFSET ?
        "
    );

    let mut stmt = conn
        .prepare(&select_sql)
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let iter = stmt
        .query_map(params_from_iter(select_params.iter()), |row| {
            Ok(FacturaEmitida {
                id: row.get(0)?,
                venta_id: row.get(1)?,
                uuid: row.get(2)?,
                rfc_receptor: row.get(3)?,
                monto_total: row.get(4)?,
                estado: row.get(5)?,
                fecha_emision: row.get(6)?,
                pdf_path: row.get(7)?,
                xml_path: row.get(8)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut facturas = Vec::new();
    for item in iter {
        facturas.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(FacturasEmitidasPage { rows: facturas, total })
}

#[tauri::command]
fn get_facturas_por_ventas(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    venta_ids: Vec<String>,
) -> AppResult<Vec<FacturaEmitida>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = require_admin_or_superadmin(&state_sesion)?;
    let sucursal_id = scoped_sucursal_for_read(&user, None);
    let ids: Vec<String> = venta_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .take(100)
        .collect();

    if ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = std::iter::repeat("?")
        .take(ids.len())
        .collect::<Vec<_>>()
        .join(",");
    let mut sql = format!(
        "
        SELECT fe.id, fe.venta_id, fe.uuid, fe.rfc_receptor, fe.monto_total, fe.estado, fe.fecha_emision, fe.pdf_path, fe.xml_path
        FROM facturas_emitidas fe
        INNER JOIN ventas v ON v.id = fe.venta_id
        WHERE fe.venta_id IN ({placeholders})
        "
    );
    let mut params_vec = ids;
    if let Some(value) = sucursal_id {
        sql.push_str(" AND v.sucursal_id = ?");
        params_vec.push(value);
    }
    sql.push_str(" ORDER BY fe.fecha_emision DESC");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let iter = stmt
        .query_map(params_from_iter(params_vec.iter()), |row| {
            Ok(FacturaEmitida {
                id: row.get(0)?,
                venta_id: row.get(1)?,
                uuid: row.get(2)?,
                rfc_receptor: row.get(3)?,
                monto_total: row.get(4)?,
                estado: row.get(5)?,
                fecha_emision: row.get(6)?,
                pdf_path: row.get(7)?,
                xml_path: row.get(8)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut facturas = Vec::new();
    for item in iter {
        facturas.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }

    Ok(facturas)
}

#[tauri::command]
fn get_sync_migration_status(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<SyncMigrationStatus> {
    require_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut tablas = Vec::new();
    let mut tablas_con_uuid = Vec::new();

    for table in SYNC_TABLES {
        if table_has_column(&conn, table, "sincronizado").map_err(to_command_error)?
            && table_has_column(&conn, table, "updated_at").map_err(to_command_error)?
        {
            tablas.push((*table).to_string());
        }
    }

    for table in UUID_SYNC_TABLES {
        if table_has_column(&conn, table, "sync_uuid").map_err(to_command_error)? {
            tablas_con_uuid.push((*table).to_string());
        }
    }

    Ok(SyncMigrationStatus {
        tablas,
        tablas_con_uuid,
    })
}

#[tauri::command]
fn get_sync_status(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<SyncStatus> {
    current_session_user(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut pendientes = 0;
    let mut ventas_pendientes = 0;
    let mut tablas_pendientes = Vec::new();

    for table in SYNC_TABLES {
        let count = count_table_sync_pending(&conn, table).map_err(to_command_error)?;
        pendientes += count;
        if *table == "ventas" {
            ventas_pendientes = count;
        }
        if count > 0 {
            tablas_pendientes.push(SyncTableStatus {
                tabla: (*table).to_string(),
                pendientes: count,
            });
        }
    }

    let runtime = conn
        .query_row(
            "
            SELECT ultimo_intento_at, ultimo_exito_at, ultimo_error_at, ultimo_error
            FROM sync_runtime_status
            WHERE id = 1
            ",
            [],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let (ultimo_intento_at, ultimo_exito_at, ultimo_error_at, ultimo_error) =
        runtime.unwrap_or((None, None, None, None));

    Ok(SyncStatus {
        pendientes,
        ventas_pendientes,
        tablas_pendientes,
        ultimo_intento_at,
        ultimo_exito_at,
        ultimo_error_at,
        ultimo_error,
    })
}

#[tauri::command]
fn get_notificaciones(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    solo_no_leidas: Option<bool>,
) -> AppResult<Vec<Notificacion>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
    evaluar_notificaciones_negocio(&conn).map_err(to_command_error)?;

    let only_unread = solo_no_leidas.unwrap_or(false);
    let mut sql = String::from(
        "
        SELECT id, categoria, severidad, titulo, mensaje, entidad_tipo, entidad_id, event_key, leida, creada_at
        FROM notificaciones
        WHERE 1 = 1
        ",
    );
    let mut params_vec: Vec<String> = Vec::new();

    if only_unread {
        sql.push_str(" AND leida = 0");
    }

    if !is_superadmin(&user) {
        sql.push_str(
            "
            AND (
                (entidad_tipo = 'sucursal' AND entidad_id = ?1)
                OR event_key LIKE ?2
                OR (
                    entidad_tipo = 'caja_sesion'
                    AND entidad_id IN (
                        SELECT id FROM cajas_sesiones WHERE sucursal_id = ?3
                    )
                )
                OR (
                    entidad_tipo = 'factura'
                    AND entidad_id IN (
                        SELECT fe.id
                        FROM facturas_emitidas fe
                        INNER JOIN ventas v ON v.id = fe.venta_id
                        WHERE v.sucursal_id = ?4
                    )
                )
            )
            ",
        );
        params_vec.push(user.sucursal_id.clone());
        params_vec.push(format!("%:{}", user.sucursal_id));
        params_vec.push(user.sucursal_id.clone());
        params_vec.push(user.sucursal_id.clone());
    }

    sql.push_str(" ORDER BY leida ASC, creada_at DESC LIMIT 50");

    let mut stmt = conn.prepare(&sql).map_err(AppError::from).map_err(to_command_error)?;
    let rows = stmt
        .query_map(params_from_iter(params_vec.iter()), |row| {
            Ok(Notificacion {
                id: row.get(0)?,
                categoria: row.get(1)?,
                severidad: row.get(2)?,
                titulo: row.get(3)?,
                mensaje: row.get(4)?,
                entidad_tipo: row.get(5)?,
                entidad_id: row.get(6)?,
                event_key: row.get(7)?,
                leida: row.get::<_, i64>(8)? != 0,
                creada_at: row.get(9)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut notificaciones = Vec::new();
    for row in rows {
        notificaciones.push(row.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(notificaciones)
}

#[tauri::command]
fn marcar_notificacion_leida(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
) -> AppResult<()> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;

    if is_superadmin(&user) {
        conn.execute("UPDATE notificaciones SET leida = 1 WHERE id = ?1", [id])
            .map_err(AppError::from)
            .map_err(to_command_error)?;
    } else {
        conn.execute(
            "
            UPDATE notificaciones
            SET leida = 1
            WHERE id = ?1
              AND (
                (entidad_tipo = 'sucursal' AND entidad_id = ?2)
                OR event_key LIKE ?3
                OR (
                    entidad_tipo = 'caja_sesion'
                    AND entidad_id IN (
                        SELECT id FROM cajas_sesiones WHERE sucursal_id = ?4
                    )
                )
                OR (
                    entidad_tipo = 'factura'
                    AND entidad_id IN (
                        SELECT fe.id
                        FROM facturas_emitidas fe
                        INNER JOIN ventas v ON v.id = fe.venta_id
                        WHERE v.sucursal_id = ?5
                    )
                )
              )
            ",
            params![id, user.sucursal_id, format!("%:{}", user.sucursal_id), user.sucursal_id, user.sucursal_id],
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    }
    Ok(())
}

#[tauri::command]
fn marcar_todas_notificaciones_leidas(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<()> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;

    if is_superadmin(&user) {
        conn.execute("UPDATE notificaciones SET leida = 1 WHERE leida = 0", [])
            .map_err(AppError::from)
            .map_err(to_command_error)?;
    } else {
        conn.execute(
            "
            UPDATE notificaciones
            SET leida = 1
            WHERE leida = 0
              AND (
                (entidad_tipo = 'sucursal' AND entidad_id = ?1)
                OR event_key LIKE ?2
                OR (
                    entidad_tipo = 'caja_sesion'
                    AND entidad_id IN (
                        SELECT id FROM cajas_sesiones WHERE sucursal_id = ?3
                    )
                )
                OR (
                    entidad_tipo = 'factura'
                    AND entidad_id IN (
                        SELECT fe.id
                        FROM facturas_emitidas fe
                        INNER JOIN ventas v ON v.id = fe.venta_id
                        WHERE v.sucursal_id = ?4
                    )
                )
              )
            ",
            params![user.sucursal_id, format!("%:{}", user.sucursal_id), user.sucursal_id, user.sucursal_id],
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    }
    Ok(())
}

fn insertar_notificacion(
    conn: &Connection,
    categoria: &str,
    severidad: &str,
    titulo: &str,
    mensaje: &str,
    entidad_tipo: Option<&str>,
    entidad_id: Option<&str>,
    event_key: &str,
) -> Result<(), AppError> {
    conn.execute(
        "
        INSERT INTO notificaciones (
            id, categoria, severidad, titulo, mensaje, entidad_tipo, entidad_id, event_key, leida, creada_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, datetime('now'))
        ON CONFLICT(event_key) DO UPDATE SET
            categoria = excluded.categoria,
            severidad = excluded.severidad,
            titulo = excluded.titulo,
            mensaje = excluded.mensaje,
            entidad_tipo = excluded.entidad_tipo,
            entidad_id = excluded.entidad_id
        ",
        params![
            generate_uuid_like(),
            categoria,
            severidad,
            titulo,
            mensaje,
            entidad_tipo,
            entidad_id,
            event_key
        ],
    )?;
    Ok(())
}

fn evaluar_notificaciones_negocio(conn: &Connection) -> Result<(), AppError> {
    evaluar_alertas_inventario(conn)?;
    evaluar_alertas_caja(conn)?;
    evaluar_alertas_credito(conn)?;
    evaluar_alertas_facturacion(conn)?;
    Ok(())
}

fn evaluar_alertas_inventario(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "
        SELECT
          p.id,
          p.descripcion,
          p.marca,
          i.sucursal_id,
          s.nombre,
          i.stock,
          i.stock_minimo
        FROM inventario_sucursal i
        INNER JOIN productos p ON p.id = i.producto_id
        INNER JOIN sucursales s ON s.id = i.sucursal_id
        WHERE p.eliminado = 0
          AND i.eliminado = 0
          AND s.eliminado = 0
          AND i.stock_minimo > 0
          AND i.stock <= i.stock_minimo
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, f64>(5)?,
            row.get::<_, f64>(6)?,
        ))
    })?;

    for row in rows {
        let (producto_id, descripcion, marca, sucursal_id, sucursal_nombre, stock, stock_minimo) = row?;
        let producto = if marca.trim().is_empty() {
            descripcion
        } else {
            format!("{descripcion} {marca}")
        };
        if stock <= 0.0 {
            insertar_notificacion(
                conn,
                "INVENTARIO",
                "CRITICAL",
                "Producto agotado",
                &format!("'{producto}' llegó a 0 piezas en {sucursal_nombre}. Se sugiere generar orden de compra."),
                Some("producto"),
                Some(&producto_id),
                &format!("inventario:agotado:{producto_id}:{sucursal_id}"),
            )?;
        } else {
            insertar_notificacion(
                conn,
                "INVENTARIO",
                "WARNING",
                "Stock mínimo alcanzado",
                &format!(
                    "'{producto}' bajó de su stock mínimo en {sucursal_nombre}. Quedan {:.2} piezas; mínimo configurado: {:.2}.",
                    stock, stock_minimo
                ),
                Some("producto"),
                Some(&producto_id),
                &format!("inventario:minimo:{producto_id}:{sucursal_id}"),
            )?;
        }
    }

    Ok(())
}

fn evaluar_alertas_caja(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "
        SELECT
          cs.id,
          cs.sucursal_id,
          s.nombre,
          cs.fecha_apertura,
          cs.monto_esperado
        FROM cajas_sesiones cs
        INNER JOIN sucursales s ON s.id = cs.sucursal_id
        WHERE cs.estado = 'ABIERTA'
          AND s.eliminado = 0
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, f64>(4)?,
        ))
    })?;

    for row in rows {
        let (sesion_id, sucursal_id, sucursal_nombre, fecha_apertura, monto_esperado) = row?;
        if monto_esperado >= 5000.0 {
            insertar_notificacion(
                conn,
                "CAJA",
                "CRITICAL",
                "Efectivo máximo alcanzado",
                &format!(
                    "Hay más de $5,000 MXN en caja de {sucursal_nombre}. Monto esperado actual: ${:.2}. Se sugiere realizar un retiro parcial.",
                    monto_esperado
                ),
                Some("caja_sesion"),
                Some(&sesion_id),
                &format!("caja:efectivo-maximo:{sesion_id}"),
            )?;
        }

        let hora = fecha_apertura
            .split('T')
            .nth(1)
            .or_else(|| fecha_apertura.split(' ').nth(1))
            .and_then(|time| time.get(0..2))
            .and_then(|hour| hour.parse::<u32>().ok());
        if let Some(hour) = hora {
            if hour < 6 || hour >= 22 {
                insertar_notificacion(
                    conn,
                    "CAJA",
                    "CRITICAL",
                    "Apertura fuera de horario",
                    &format!("La caja de {sucursal_nombre} fue abierta a las {fecha_apertura}. Verifica si corresponde a una operación autorizada."),
                    Some("sucursal"),
                    Some(&sucursal_id),
                    &format!("caja:fuera-horario:{sesion_id}"),
                )?;
            }
        }
    }

    Ok(())
}

fn evaluar_alertas_credito(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "
        SELECT id, nombre, limite_credito, saldo_deudor
        FROM clientes
        WHERE eliminado = 0
          AND limite_credito > 0
          AND saldo_deudor >= limite_credito * 0.9
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, f64>(2)?,
            row.get::<_, f64>(3)?,
        ))
    })?;

    for row in rows {
        let (cliente_id, nombre, limite_credito, saldo_deudor) = row?;
        let porcentaje = (saldo_deudor / limite_credito) * 100.0;
        let severidad = if saldo_deudor >= limite_credito {
            "CRITICAL"
        } else {
            "WARNING"
        };
        insertar_notificacion(
            conn,
            "CREDITOS",
            severidad,
            "Límite de crédito alcanzado",
            &format!(
                "El cliente '{nombre}' está al {:.0}% de su límite de crédito. Saldo: ${:.2} / Límite: ${:.2}.",
                porcentaje, saldo_deudor, limite_credito
            ),
            Some("cliente"),
            Some(&cliente_id),
            &format!("credito:limite:{cliente_id}"),
        )?;
    }

    Ok(())
}

fn evaluar_alertas_facturacion(conn: &Connection) -> Result<(), AppError> {
    let mut stmt = conn.prepare(
        "
        SELECT fe.id, fe.venta_id, fe.rfc_receptor, fe.monto_total, fe.fecha_emision
        FROM facturas_emitidas fe
        INNER JOIN ventas v ON v.id = fe.venta_id
        INNER JOIN sucursales s ON s.id = v.sucursal_id
        WHERE fe.estado = 'PENDIENTE'
          AND v.estado = 'COMPLETADA'
          AND s.eliminado = 0
          AND datetime(fe.fecha_emision) <= datetime('now', '-1 day')
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, f64>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    for row in rows {
        let (factura_id, venta_id, rfc_receptor, monto_total, fecha_emision) = row?;
        insertar_notificacion(
            conn,
            "FACTURACION",
            "WARNING",
            "Factura pendiente de timbrar",
            &format!(
                "La factura de la venta {venta_id} sigue pendiente desde {fecha_emision}. RFC receptor: {rfc_receptor}. Total: ${:.2}.",
                monto_total
            ),
            Some("factura"),
            Some(&factura_id),
            &format!("facturacion:pendiente:{factura_id}"),
        )?;
    }

    Ok(())
}

fn current_timestamp_string() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs();
    seconds.to_string()
}

fn backup_dir() -> Result<PathBuf, AppError> {
    let dir = std::env::current_dir()
        .map_err(|error| AppError::Db(format!("No se pudo resolver la carpeta local: {error}")))?
        .join("respaldos");
    fs::create_dir_all(&dir)
        .map_err(|error| AppError::Db(format!("No se pudo crear la carpeta de respaldos: {error}")))?;
    Ok(dir)
}

fn value_ref_to_json(value: ValueRef<'_>) -> JsonValue {
    match value {
        ValueRef::Null => JsonValue::Null,
        ValueRef::Integer(value) => JsonValue::from(value),
        ValueRef::Real(value) => JsonValue::from(value),
        ValueRef::Text(value) => JsonValue::from(String::from_utf8_lossy(value).to_string()),
        ValueRef::Blob(value) => JsonValue::from(String::from_utf8_lossy(value).to_string()),
    }
}

fn json_to_sql_value(value: &JsonValue) -> Result<Value, AppError> {
    match value {
        JsonValue::Null => Ok(Value::Null),
        JsonValue::Bool(value) => Ok(Value::Integer(if *value { 1 } else { 0 })),
        JsonValue::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(Value::Integer(value))
            } else if let Some(value) = value.as_f64() {
                Ok(Value::Real(value))
            } else {
                Err(AppError::Validation("Número inválido en respaldo.".to_string()))
            }
        }
        JsonValue::String(value) => Ok(Value::Text(value.clone())),
        _ => Err(AppError::Validation(
            "El respaldo contiene valores no compatibles con SQLite.".to_string(),
        )),
    }
}

fn is_safe_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn export_table(conn: &Connection, table: &str) -> Result<Vec<JsonValue>, AppError> {
    let mut stmt = conn.prepare(&format!("SELECT * FROM {table}"))?;
    let column_names: Vec<String> = stmt.column_names().iter().map(|value| value.to_string()).collect();
    let rows = stmt.query_map([], |row| {
        let mut object = JsonMap::new();
        for (index, name) in column_names.iter().enumerate() {
            let value = if name == "sincronizado" || name == "eliminado" {
                match row.get_ref(index)? {
                    ValueRef::Integer(value) => JsonValue::Bool(value != 0),
                    other => value_ref_to_json(other),
                }
            } else {
                value_ref_to_json(row.get_ref(index)?)
            };
            object.insert(name.clone(), value);
        }
        Ok(JsonValue::Object(object))
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

fn build_backup(conn: &Connection) -> Result<BackupLocal, AppError> {
    let mut tablas = HashMap::new();
    for table in BACKUP_TABLES {
        tablas.insert((*table).to_string(), export_table(conn, table)?);
    }
    for table in LOCAL_ONLY_BACKUP_TABLES {
        tablas.insert((*table).to_string(), export_table(conn, table)?);
    }

    Ok(BackupLocal {
        version: "1".to_string(),
        generado_at: current_timestamp_string(),
        tablas,
    })
}

fn validate_backup_payload(backup: &BackupLocal) -> Result<(), AppError> {
    if !matches!(backup.version.as_str(), "1" | "supabase-rest-v1") {
        return Err(AppError::Validation(format!(
            "Versión de respaldo no compatible: {}.",
            backup.version
        )));
    }

    let missing_tables: Vec<&str> = BACKUP_TABLES
        .iter()
        .copied()
        .filter(|table| !backup.tablas.contains_key(*table))
        .collect();
    if !missing_tables.is_empty() {
        return Err(AppError::Validation(format!(
            "El respaldo está incompleto. Faltan tablas críticas: {}.",
            missing_tables.join(", ")
        )));
    }

    Ok(())
}

fn map_backup_column_to_local(table: &str, column: &str, is_remote_backup: bool) -> Option<String> {
    if is_remote_backup && UUID_SYNC_TABLES.contains(&table) && column == "uuid" {
        return Some("sync_uuid".to_string());
    }
    if is_remote_backup && table == "facturas_emitidas" && column == "uuid_sat" {
        return Some("uuid".to_string());
    }
    if is_remote_backup && column.ends_with("_uuid") {
        return None;
    }
    Some(column.to_string())
}

fn apply_backup_to_conn(conn: &mut Connection, backup: BackupLocal) -> Result<(), AppError> {
    validate_backup_payload(&backup)?;

    let is_remote_backup = backup.version == "supabase-rest-v1";
    let mut columns_by_table: HashMap<String, Vec<String>> = HashMap::new();
    for table in BACKUP_TABLES {
        columns_by_table.insert((*table).to_string(), table_columns(conn, table)?);
    }
    if !is_remote_backup {
        for table in LOCAL_ONLY_BACKUP_TABLES {
            columns_by_table.insert((*table).to_string(), table_columns(conn, table)?);
        }
    }

    conn.execute_batch("PRAGMA foreign_keys = OFF")?;
    let apply_result = (|| -> Result<(), AppError> {
        let tx = conn.transaction()?;

        if !is_remote_backup {
            for table in LOCAL_ONLY_BACKUP_TABLES.iter().rev() {
                tx.execute(&format!("DELETE FROM {table}"), [])?;
            }
        }
        for table in BACKUP_TABLES.iter().rev() {
            tx.execute(&format!("DELETE FROM {table}"), [])?;
        }

        for table in BACKUP_TABLES.iter().chain(
            if is_remote_backup {
                [].iter()
            } else {
                LOCAL_ONLY_BACKUP_TABLES.iter()
            },
        ) {
            let Some(rows) = backup.tablas.get(*table) else {
                continue;
            };

            for row in rows {
                let JsonValue::Object(object) = row else {
                    return Err(AppError::Validation(format!(
                        "La tabla {table} contiene un renglón inválido en el respaldo."
                    )));
                };

                let local_columns = columns_by_table
                    .get(*table)
                    .ok_or_else(|| AppError::Db(format!("No se pudieron leer columnas de {table}.")))?;
                let mut columns = Vec::new();
                let mut values = Vec::new();
                for (column, value) in object {
                    let Some(local_column) = map_backup_column_to_local(table, column, is_remote_backup) else {
                        continue;
                    };
                    if !is_safe_identifier(&local_column) {
                        return Err(AppError::Validation(format!(
                            "El respaldo contiene una columna inválida: {local_column}."
                        )));
                    }
                    if !local_columns.contains(&local_column) {
                        continue;
                    }
                    columns.push(local_column);
                    values.push(json_to_sql_value(value)?);
                }

                if columns.is_empty() {
                    continue;
                }

                let placeholders = vec!["?"; columns.len()].join(", ");
                let sql = format!(
                    "INSERT OR REPLACE INTO {table} ({}) VALUES ({})",
                    columns.join(", "),
                    placeholders
                );
                tx.execute(&sql, params_from_iter(values.iter()))?;
            }
        }

        let fk_violations: i64 = tx.query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_check",
            [],
            |row| row.get(0),
        )?;
        if fk_violations > 0 {
            return Err(AppError::Validation(format!(
                "El respaldo no se aplicó porque contiene {fk_violations} referencias inválidas."
            )));
        }

        tx.commit()?;
        Ok(())
    })();

    let enable_result = conn.execute_batch("PRAGMA foreign_keys = ON").map_err(AppError::from);
    apply_result?;
    enable_result?;
    Ok(())
}

fn query_json_rows_with_local_ids(conn: &Connection, sql: &str) -> Result<Vec<(String, JsonValue)>, AppError> {
    let mut stmt = conn.prepare(sql)?;
    let column_names: Vec<String> = stmt.column_names().iter().map(|value| value.to_string()).collect();
    let rows = stmt.query_map([], |row| {
        let mut local_id = String::new();
        let mut object = JsonMap::new();
        for (index, name) in column_names.iter().enumerate() {
            let value = if name == "__local_id" {
                local_id = match row.get_ref(index)? {
                    ValueRef::Text(value) => String::from_utf8_lossy(value).to_string(),
                    ValueRef::Integer(value) => value.to_string(),
                    other => value_ref_to_json(other).to_string(),
                };
                continue;
            } else if name == "sincronizado" || name == "eliminado" {
                match row.get_ref(index)? {
                    ValueRef::Integer(value) => JsonValue::Bool(value != 0),
                    other => value_ref_to_json(other),
                }
            } else {
                value_ref_to_json(row.get_ref(index)?)
            };
            object.insert(name.clone(), value);
        }
        Ok((local_id, JsonValue::Object(object)))
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

fn current_sqlite_timestamp(conn: &Connection) -> Result<String, AppError> {
    conn.query_row("SELECT datetime('now')", [], |row| row.get(0))
        .map_err(AppError::from)
}

fn count_sync_pending(conn: &Connection) -> Result<i64, AppError> {
    let mut total = 0;
    for table in SYNC_TABLES {
        total += count_table_sync_pending(conn, table)?;
    }
    Ok(total)
}

fn count_table_sync_pending(conn: &Connection, table: &str) -> Result<i64, AppError> {
    conn.query_row(
        &format!("SELECT COUNT(*) FROM {table} WHERE sincronizado = 0"),
        [],
        |row| row.get::<_, i64>(0),
    )
    .map_err(AppError::from)
}

fn mark_table_ids_synced(conn: &Connection, table: &str, ids: &[String], synced_at: &str) -> Result<(), AppError> {
    if ids.is_empty() {
        return Ok(());
    }

    if table == "inventario_sucursal" {
        for id in ids {
            let Some((producto_id, sucursal_id)) = id.split_once('|') else {
                continue;
            };
            conn.execute(
                "
                UPDATE inventario_sucursal
                SET sincronizado = 1,
                    updated_at = ?1
                WHERE producto_id = ?2 AND sucursal_id = ?3
                ",
                params![synced_at, producto_id, sucursal_id],
            )?;
        }
        return Ok(());
    }
    if table == "promocion_sucursales" {
        for id in ids {
            let Some((promocion_id, sucursal_id)) = id.split_once('|') else {
                continue;
            };
            conn.execute(
                "
                UPDATE promocion_sucursales
                SET sincronizado = 1,
                    updated_at = ?1
                WHERE promocion_id = ?2 AND sucursal_id = ?3
                ",
                params![synced_at, promocion_id, sucursal_id],
            )?;
        }
        return Ok(());
    }

    let key_column = match table {
        "clientes_datos_fiscales" => "cliente_id",
        "empresa_config_fiscal" => "id",
        "movimientos_inventario" => "uuid",
        _ => "id",
    };
    let placeholders = vec!["?"; ids.len()].join(", ");
    let sql = format!(
        "UPDATE {table} SET sincronizado = 1, updated_at = ? WHERE {key_column} IN ({placeholders})"
    );
    let mut params_values: Vec<Value> = vec![Value::Text(synced_at.to_string())];
    params_values.extend(ids.iter().cloned().map(Value::Text));
    conn.execute(&sql, params_from_iter(params_values.iter()))?;
    Ok(())
}

fn url_encode_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        let ch = byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            encoded.push(ch);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn max_synced_updated_at(conn: &Connection, table: &str) -> Result<String, AppError> {
    conn.query_row(
        &format!(
            "SELECT COALESCE(MAX(updated_at), '1970-01-01') FROM {table} WHERE sincronizado = 1"
        ),
        [],
        |row| row.get(0),
    )
    .map_err(AppError::from)
}

fn pull_conflict_columns(table: &str) -> &'static [&'static str] {
    match table {
        "inventario_sucursal" => &["producto_id", "sucursal_id"],
        "promocion_sucursales" => &["promocion_id", "sucursal_id"],
        "clientes_datos_fiscales" => &["cliente_id"],
        "movimientos_inventario" => &["uuid"],
        _ => &["id"],
    }
}

fn get_local_sucursal_ids(conn: &Connection) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare("SELECT id FROM sucursales WHERE eliminado = 0 ORDER BY id")?;
    let iter = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut ids = Vec::new();
    for item in iter {
        ids.push(item?);
    }
    Ok(ids)
}

fn push_remote_row_value(
    table: &str,
    column: &str,
    value: &JsonValue,
    columns: &mut Vec<String>,
    values: &mut Vec<Value>,
    local_columns: &[String],
) -> Result<(), AppError> {
    let Some(local_column) = map_backup_column_to_local(table, column, true) else {
        return Ok(());
    };
    if !is_safe_identifier(&local_column) || !local_columns.contains(&local_column) {
        return Ok(());
    }
    columns.push(local_column);
    values.push(json_to_sql_value(value)?);
    Ok(())
}

fn json_updated_at(value: Option<&JsonValue>) -> String {
    value
        .and_then(JsonValue::as_str)
        .unwrap_or("1970-01-01")
        .to_string()
}

fn sync_key_from_values(conflict_columns: &[&str], columns: &[String], values: &[Value]) -> String {
    conflict_columns
        .iter()
        .filter_map(|key| {
            let position = columns.iter().position(|column| column == key)?;
            let value = match &values[position] {
                Value::Null => "NULL".to_string(),
                Value::Integer(value) => value.to_string(),
                Value::Real(value) => value.to_string(),
                Value::Text(value) => value.clone(),
                Value::Blob(value) => format!("{value:?}"),
            };
            Some(format!("{key}={value}"))
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn should_apply_remote_row(
    tx: &Transaction<'_>,
    table: &str,
    conflict_columns: &[&str],
    columns: &[String],
    values: &[Value],
    remote_updated_at: &str,
) -> Result<bool, AppError> {
    let mut where_parts = Vec::new();
    let mut key_values = Vec::new();
    for key in conflict_columns {
        let Some(position) = columns.iter().position(|column| column == key) else {
            return Ok(false);
        };
        where_parts.push(format!("{key} = ?"));
        key_values.push(values[position].clone());
    }

    if where_parts.is_empty() {
        return Ok(false);
    }

    let sql = format!(
        "SELECT sincronizado, updated_at FROM {table} WHERE {}",
        where_parts.join(" AND ")
    );
    let local_state = tx
        .query_row(&sql, params_from_iter(key_values.iter()), |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .optional()?;

    let Some((sincronizado, local_updated_at)) = local_state else {
        return Ok(true);
    };

    let local_is_newer: i64 = tx
        .query_row(
            "SELECT CASE WHEN datetime(?1) > datetime(?2) THEN 1 ELSE 0 END",
            params![local_updated_at, remote_updated_at],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if sincronizado == 0 && local_is_newer == 1 {
        return Ok(false);
    }

    Ok(true)
}

fn apply_remote_delta_rows(conn: &mut Connection, table: &str, rows: Vec<JsonValue>) -> Result<usize, AppError> {
    if rows.is_empty() {
        return Ok(0);
    }

    let local_columns = table_columns(conn, table)?;
    let conflict_columns = pull_conflict_columns(table);
    let tx = conn.transaction()?;
    let mut applied = 0;

    for row in rows {
        let JsonValue::Object(mut object) = row else {
            return Err(AppError::Validation(format!(
                "Supabase devolvió un renglón inválido para {table}."
            )));
        };

        if local_columns.contains(&"sincronizado".to_string()) {
            object.insert("sincronizado".to_string(), JsonValue::from(true));
        }

        let mut columns = Vec::new();
        let mut values = Vec::new();
        for (column, value) in &object {
            push_remote_row_value(table, column, value, &mut columns, &mut values, &local_columns)?;
        }

        if columns.is_empty() || conflict_columns.iter().any(|key| !columns.iter().any(|column| column == key)) {
            continue;
        }

        let remote_updated_at = json_updated_at(object.get("updated_at"));
        if local_columns.contains(&"sincronizado".to_string())
            && local_columns.contains(&"updated_at".to_string())
            && !should_apply_remote_row(&tx, table, conflict_columns, &columns, &values, &remote_updated_at)?
        {
            let sync_key = sync_key_from_values(conflict_columns, &columns, &values);
            insertar_notificacion(
                &tx,
                "SINCRONIZACION",
                "WARNING",
                "Conflicto de sincronización resuelto",
                &format!(
                    "Se conservó el cambio local pendiente en {table} ({sync_key}) porque es más reciente que la versión recibida de Supabase."
                ),
                Some(table),
                Some(&sync_key),
                &format!("sync:conflict:{table}:{sync_key}:{remote_updated_at}"),
            )?;
            continue;
        }

        let placeholders = vec!["?"; columns.len()].join(", ");
        let update_columns: Vec<String> = columns
            .iter()
            .filter(|column| !conflict_columns.iter().any(|key| key == &column.as_str()))
            .map(|column| format!("{column} = excluded.{column}"))
            .collect();
        let conflict_target = conflict_columns.join(", ");
        let sql = if update_columns.is_empty() {
            format!(
                "INSERT INTO {table} ({}) VALUES ({}) ON CONFLICT({conflict_target}) DO NOTHING",
                columns.join(", "),
                placeholders
            )
        } else {
            format!(
                "INSERT INTO {table} ({}) VALUES ({}) ON CONFLICT({conflict_target}) DO UPDATE SET {}",
                columns.join(", "),
                placeholders,
                update_columns.join(", ")
            )
        };
        tx.execute(&sql, params_from_iter(values.iter()))?;

        let mut where_parts = Vec::new();
        let mut key_values = Vec::new();
        for key in conflict_columns {
            let Some(position) = columns.iter().position(|column| column == key) else {
                continue;
            };
            where_parts.push(format!("{key} = ?"));
            key_values.push(values[position].clone());
        }

        if !where_parts.is_empty() && local_columns.contains(&"sincronizado".to_string()) {
            let mut sync_values = vec![Value::Integer(1)];
            if local_columns.contains(&"updated_at".to_string()) {
                sync_values.push(Value::Text(remote_updated_at));
                sync_values.extend(key_values);
                tx.execute(
                    &format!(
                        "UPDATE {table} SET sincronizado = ?, updated_at = ? WHERE {}",
                        where_parts.join(" AND ")
                    ),
                    params_from_iter(sync_values.iter()),
                )?;
            } else {
                sync_values.extend(key_values);
                tx.execute(
                    &format!(
                        "UPDATE {table} SET sincronizado = ? WHERE {}",
                        where_parts.join(" AND ")
                    ),
                    params_from_iter(sync_values.iter()),
                )?;
            }
        }

        applied += 1;
    }

    tx.commit()?;
    Ok(applied)
}

fn build_delta_pull_endpoint(
    config: &SupabaseConfig,
    table: &str,
    since: &str,
    sucursal_ids: &[String],
    limit: usize,
) -> String {
    let mut endpoint = format!(
        "{}/rest/v1/{}?select=*&updated_at={}&order=updated_at.asc&limit={limit}",
        config.url.trim_end_matches('/'),
        table,
        url_encode_component(&format!("gt.{since}")),
    );

    if !sucursal_ids.is_empty() {
        let ids_filter = format!("in.({})", sucursal_ids.join(","));
        match table {
            "inventario_sucursal"
            | "usuarios"
            | "compras"
            | "detalle_compras"
            | "ventas"
            | "detalle_ventas"
            | "creditos_abonos"
            | "cajas_sesiones"
            | "caja_movimientos"
            | "mermas_ajustes"
            | "movimientos_inventario"
            | "facturas_emitidas" => {
                endpoint.push_str("&sucursal_id=");
                endpoint.push_str(&url_encode_component(&ids_filter));
            }
            "traspasos" => {
                endpoint.push_str("&or=");
                endpoint.push_str(&url_encode_component(&format!(
                    "(sucursal_origen_id.{ids_filter},sucursal_destino_id.{ids_filter})"
                )));
            }
            // Supabase stores only sucursal_origen_id on detalle_traspasos. Filtering here
            // would let destination branches pull the transfer header but not its detail rows.
            "detalle_traspasos" => {}
            _ => {}
        }
    }

    endpoint
}

fn pull_delta_table(
    conn: &mut Connection,
    client: &reqwest::blocking::Client,
    config: &SupabaseConfig,
    table: &str,
    sucursal_ids: &[String],
) -> AppResult<usize> {
    let since = max_synced_updated_at(conn, table).map_err(to_command_error)?;
    let endpoint = build_delta_pull_endpoint(config, table, &since, sucursal_ids, AUTO_SYNC_BATCH_SIZE);
    let response = supabase_request_builder(client, &endpoint, &config.anon_key)
        .send()
        .map_err(|error| format!("No se pudo descargar cambios de {table}: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "Supabase rechazó la descarga incremental de {table}. Código HTTP: {status}. {body}"
        ));
    }

    let rows: Vec<JsonValue> = response
        .json()
        .map_err(|error| format!("La respuesta incremental de {table} no es JSON válido: {error}"))?;

    apply_remote_delta_rows(conn, table, rows).map_err(to_command_error)
}

fn download_full_table_rows(
    client: &reqwest::blocking::Client,
    config: &SupabaseConfig,
    table: &str,
) -> AppResult<Vec<JsonValue>> {
    const PAGE_SIZE: usize = 1000;
    let mut offset = 0;
    let mut all_rows = Vec::new();

    loop {
        let endpoint = format!(
            "{}/rest/v1/{}?select=*&order=updated_at.asc&limit={PAGE_SIZE}&offset={offset}",
            config.url.trim_end_matches('/'),
            table
        );
        let response = supabase_request_builder(client, &endpoint, &config.anon_key)
            .send()
            .map_err(|error| format!("No se pudo descargar la tabla {table}: {error}"))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(format!(
                "Supabase rechazó la descarga de {table}. Código HTTP: {status}. {body}"
            ));
        }

        let rows: Vec<JsonValue> = response
            .json()
            .map_err(|error| format!("La respuesta de Supabase para {table} no es JSON válido: {error}"))?;
        let row_count = rows.len();
        all_rows.extend(rows);

        if row_count < PAGE_SIZE {
            break;
        }
        offset += PAGE_SIZE;
    }

    Ok(all_rows)
}

fn remote_row_local_id(table: &str, row: &JsonValue) -> Option<String> {
    let object = row.as_object()?;
    match table {
        "inventario_sucursal" => Some(format!(
            "{}|{}",
            object.get("producto_id")?.as_str()?,
            object.get("sucursal_id")?.as_str()?
        )),
        "promocion_sucursales" => Some(format!(
            "{}|{}",
            object.get("promocion_id")?.as_str()?,
            object.get("sucursal_id")?.as_str()?
        )),
        "clientes_datos_fiscales" => object.get("cliente_id")?.as_str().map(str::to_string),
        "empresa_config_fiscal" => object.get("id").map(|value| {
            value
                .as_i64()
                .map(|number| number.to_string())
                .or_else(|| value.as_str().map(str::to_string))
                .unwrap_or_default()
        }),
        "movimientos_inventario" => object.get("uuid")?.as_str().map(str::to_string),
        _ => object.get("id")?.as_str().map(str::to_string),
    }
}

fn remote_row_updated_at(row: &JsonValue) -> Option<String> {
    row.as_object()?
        .get("updated_at")?
        .as_str()
        .map(str::to_string)
}

fn mark_uploaded_rows_synced(
    conn: &Connection,
    table: &str,
    ids: &[String],
    remote_rows: &[JsonValue],
) -> Result<(), AppError> {
    let fallback_synced_at = current_sqlite_timestamp(conn)?;
    let remote_timestamps: HashMap<String, String> = remote_rows
        .iter()
        .filter_map(|row| Some((remote_row_local_id(table, row)?, remote_row_updated_at(row)?)))
        .collect();

    for id in ids {
        let synced_at = remote_timestamps
            .get(id)
            .map(String::as_str)
            .unwrap_or(&fallback_synced_at);
        mark_table_ids_synced(conn, table, std::slice::from_ref(id), synced_at)?;
    }
    Ok(())
}

fn sync_upload_plans(limit: Option<usize>, only_pending: bool) -> Vec<(&'static str, &'static str, String)> {
    let limit_clause = limit
        .map(|value| format!("LIMIT {value}"))
        .unwrap_or_default();
    let pending = if only_pending { "WHERE sincronizado = 0" } else { "" };
    let dc_pending = if only_pending { "WHERE dc.sincronizado = 0" } else { "" };
    let dv_pending = if only_pending { "WHERE dv.sincronizado = 0" } else { "" };
    let ca_pending = if only_pending { "WHERE ca.sincronizado = 0" } else { "" };
    let cm_pending = if only_pending { "WHERE cm.sincronizado = 0" } else { "" };
    let dt_pending = if only_pending { "WHERE dt.sincronizado = 0" } else { "" };
    let fe_pending = if only_pending { "WHERE fe.sincronizado = 0" } else { "" };
    vec![
        (
            "sucursales",
            "id",
            format!(
                "
                SELECT id AS __local_id, id, nombre, direccion, telefono, codigo_postal,
                       eliminado, 1 AS sincronizado, updated_at
                FROM sucursales
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "empresa_config_fiscal",
            "id",
            format!(
                "
                SELECT id AS __local_id, id, rfc, razon_social, regimen_fiscal, registro_patronal,
                       actualizado_at, 1 AS sincronizado, updated_at
                FROM empresa_config_fiscal
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "proveedores",
            "id",
            format!(
                "
                SELECT id AS __local_id, id, nombre, contacto_nombre, telefono, email, direccion,
                       eliminado, 1 AS sincronizado, updated_at
                FROM proveedores
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "marcas",
            "id",
            format!(
                "
                SELECT id AS __local_id, id, nombre, eliminado, 1 AS sincronizado, updated_at
                FROM marcas
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "categorias",
            "id",
            format!(
                "
                SELECT id AS __local_id, id, nombre, eliminado, 1 AS sincronizado, updated_at
                FROM categorias
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "unidades",
            "id",
            format!(
                "
                SELECT id AS __local_id, id, nombre, clave_sat, eliminado, 1 AS sincronizado, updated_at
                FROM unidades
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "usuarios",
            "id",
            format!(
                "
                SELECT id AS __local_id, id, email, nombre, role, sucursal_id, password_hash,
                       eliminado, 1 AS sincronizado, updated_at
                FROM usuarios
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "productos",
            "id",
            format!(
                "
                SELECT id AS __local_id, id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
                       marca, categoria, unidad, precio_costo, costo_promedio, precio_venta, sat_clave_prod_serv,
                       sat_clave_unidad, precio_1, precio_2, precio_3, precio_4, mayoreo_apartir,
                       a_granel, no_en_catalogo, ventas_negativas, caducidad, fotos, descripcion_catalogo,
                       eliminado, 1 AS sincronizado, updated_at
                FROM productos
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "inventario_sucursal",
            "producto_id,sucursal_id",
            format!(
                "
                SELECT producto_id || '|' || sucursal_id AS __local_id, producto_id, sucursal_id,
                       stock, stock_minimo, costo_promedio, precio_venta, 1 AS sincronizado, updated_at
                FROM inventario_sucursal
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "promociones",
            "id",
            format!(
                "
                SELECT id AS __local_id, id, nombre, tipo_descuento, valor, fecha_inicio, fecha_fin,
                       activo, producto_id, categoria_id, marca, eliminado, 1 AS sincronizado, updated_at
                FROM promociones
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "promocion_sucursales",
            "promocion_id,sucursal_id",
            format!(
                "
                SELECT promocion_id || '|' || sucursal_id AS __local_id, promocion_id, sucursal_id,
                       eliminado, 1 AS sincronizado, updated_at
                FROM promocion_sucursales
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "clientes",
            "id",
            format!(
                "
                SELECT id AS __local_id, id, nombre, telefono, direccion, limite_credito, saldo_deudor,
                       eliminado, 1 AS sincronizado, updated_at
                FROM clientes
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "clientes_datos_fiscales",
            "cliente_id",
            format!(
                "
                SELECT cliente_id AS __local_id, cliente_id, rfc, razon_social, regimen_fiscal,
                       codigo_postal, 1 AS sincronizado, updated_at
                FROM clientes_datos_fiscales
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "compras",
            "uuid",
            format!(
                "
                SELECT id AS __local_id, sync_uuid AS uuid, id, proveedor_id, sucursal_id, fecha,
                       total, 1 AS sincronizado, updated_at
                FROM compras
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "detalle_compras",
            "uuid",
            format!(
                "
                SELECT dc.id AS __local_id, dc.sync_uuid AS uuid, dc.id, dc.compra_id,
                       c.sucursal_id, dc.producto_id, dc.cantidad, dc.precio_costo_pactado,
                       dc.costo_promedio_resultante,
                       1 AS sincronizado, dc.updated_at
                FROM detalle_compras dc
                INNER JOIN compras c ON c.id = dc.compra_id
                {dc_pending}
                ORDER BY dc.updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "ventas",
            "uuid",
            format!(
                "
                SELECT id AS __local_id, sync_uuid AS uuid, id, usuario_id, sucursal_id, fecha, total,
                       metodo_pago, efectivo_recibido, cambio_entregado, cliente_id,
                       cliente_rapido_nombre, cliente_rapido_telefono, cliente_rapido_domicilio,
                       requiere_factura,
                       usuario_autorizo_cancelacion_id, motivo_cancelacion, fecha_cancelacion,
                       estado, 1 AS sincronizado, updated_at
                FROM ventas
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "detalle_ventas",
            "uuid",
            format!(
                "
                SELECT dv.id AS __local_id, dv.sync_uuid AS uuid, dv.id, dv.venta_id, v.sucursal_id,
                       dv.producto_id, dv.cantidad, dv.precio_venta_pactado,
                       dv.costo_unitario_pactado, dv.tipo_precio_vendido, dv.precio_original,
                       dv.descuento_aplicado, 1 AS sincronizado, dv.updated_at
                FROM detalle_ventas dv
                INNER JOIN ventas v ON v.id = dv.venta_id
                {dv_pending}
                ORDER BY dv.updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "creditos_abonos",
            "uuid",
            format!(
                "
                SELECT ca.id AS __local_id, ca.sync_uuid AS uuid, ca.id, ca.cliente_id, ca.monto,
                       ca.fecha, ca.usuario_id, u.sucursal_id, 1 AS sincronizado, ca.updated_at
                FROM creditos_abonos ca
                INNER JOIN usuarios u ON u.id = ca.usuario_id
                {ca_pending}
                ORDER BY ca.updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "cajas_sesiones",
            "uuid",
            format!(
                "
                SELECT id AS __local_id, sync_uuid AS uuid, id, usuario_id, sucursal_id,
                       fecha_apertura, monto_inicial, fecha_cierre, monto_final_real,
                       monto_esperado, estado, 1 AS sincronizado, updated_at
                FROM cajas_sesiones
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "caja_movimientos",
            "uuid",
            format!(
                "
                SELECT cm.id AS __local_id, cm.sync_uuid AS uuid, cm.id, cm.sesion_id, cs.sucursal_id,
                       cm.tipo, cm.monto, cm.motivo, 1 AS sincronizado, cm.updated_at
                FROM caja_movimientos cm
                INNER JOIN cajas_sesiones cs ON cs.id = cm.sesion_id
                {cm_pending}
                ORDER BY cm.updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "traspasos",
            "uuid",
            format!(
                "
                SELECT id AS __local_id, sync_uuid AS uuid, id, sucursal_origen_id,
                       sucursal_destino_id, usuario_id, fecha, estado, usuario_recibio_id,
                       fecha_recepcion, observaciones_recepcion, 1 AS sincronizado, updated_at
                FROM traspasos
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "detalle_traspasos",
            "uuid",
            format!(
                "
                SELECT dt.id AS __local_id, dt.sync_uuid AS uuid, dt.id, dt.traspaso_id,
                       t.sucursal_origen_id, dt.producto_id, dt.cantidad, 1 AS sincronizado,
                       dt.updated_at
                FROM detalle_traspasos dt
                INNER JOIN traspasos t ON t.id = dt.traspaso_id
                {dt_pending}
                ORDER BY dt.updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "mermas_ajustes",
            "uuid",
            format!(
                "
                SELECT id AS __local_id, sync_uuid AS uuid, id, producto_id, sucursal_id,
                       usuario_id, cantidad, tipo_movimiento, motivo, fecha, 1 AS sincronizado,
                       updated_at
                FROM mermas_ajustes
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "facturas_emitidas",
            "uuid",
            format!(
                "
                SELECT fe.id AS __local_id, fe.sync_uuid AS uuid, fe.id, fe.venta_id, v.sucursal_id,
                       fe.uuid AS uuid_sat, fe.rfc_receptor, fe.monto_total, fe.estado, fe.fecha_emision,
                       fe.pdf_path, fe.xml_path, 1 AS sincronizado, fe.updated_at
                FROM facturas_emitidas fe
                INNER JOIN ventas v ON v.id = fe.venta_id
                {fe_pending}
                ORDER BY fe.updated_at
                {limit_clause}
                "
            ),
        ),
        (
            "movimientos_inventario",
            "uuid",
            format!(
                "
                SELECT uuid AS __local_id, uuid, producto_id, sucursal_id, tipo, referencia_tipo,
                       referencia_id, cantidad, costo_unitario, usuario_id, fecha, 1 AS sincronizado,
                       updated_at
                FROM movimientos_inventario
                {pending}
                ORDER BY updated_at
                {limit_clause}
                "
            ),
        ),
    ]
}

fn run_auto_pull_once(pool: &DbPool) -> AppResult<usize> {
    let mut conn = pool.get().map_err(AppError::from).map_err(to_command_error)?;
    let Some(config) = get_supabase_config_from_conn(&conn).map_err(to_command_error)? else {
        return Ok(0);
    };

    if !config.is_connected || config.url.trim().is_empty() || config.anon_key.trim().is_empty() {
        return Ok(0);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("No se pudo preparar la conexión HTTP: {error}"))?;
    let sucursal_ids = get_local_sucursal_ids(&conn).map_err(to_command_error)?;
    let mut total = 0;

    for table in PULL_TABLES {
        total += pull_delta_table(&mut conn, &client, &config, table, &sucursal_ids)?;
    }

    Ok(total)
}

fn upload_pending_batch_table(
    conn: &Connection,
    client: &reqwest::blocking::Client,
    config: &SupabaseConfig,
    table: &str,
    conflict_target: &str,
    sql: &str,
) -> AppResult<usize> {
    let rows_with_ids = query_json_rows_with_local_ids(conn, sql).map_err(to_command_error)?;
    if rows_with_ids.is_empty() {
        return Ok(0);
    }

    let ids: Vec<String> = rows_with_ids.iter().map(|(id, _)| id.clone()).collect();
    let rows: Vec<JsonValue> = rows_with_ids.into_iter().map(|(_, row)| row).collect();
    let endpoint = format!(
        "{}/rest/v1/{}?on_conflict={}",
        config.url.trim_end_matches('/'),
        table,
        conflict_target
    );
    let response = supabase_upsert_builder(client, &endpoint, &config.anon_key)
        .json(&rows)
        .send()
        .map_err(|error| format!("No se pudo subir lote de {table}: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "Supabase rechazó el lote de {table}. Código HTTP: {status}. {body}"
        ));
    }

    let remote_rows: Vec<JsonValue> = response
        .json()
        .map_err(|error| format!("Supabase confirmó {table}, pero no devolvió JSON válido: {error}"))?;
    mark_uploaded_rows_synced(conn, table, &ids, &remote_rows).map_err(to_command_error)?;
    Ok(rows.len())
}

fn run_auto_sync_once(pool: &DbPool) -> AppResult<usize> {
    let conn = pool.get().map_err(AppError::from).map_err(to_command_error)?;
    ensure_sync_uuids(&conn).map_err(to_command_error)?;
    evaluar_notificaciones_negocio(&conn).map_err(to_command_error)?;
    let pendientes_antes = count_sync_pending(&conn).map_err(to_command_error)?;
    let ventas_pendientes_antes = count_table_sync_pending(&conn, "ventas").map_err(to_command_error)?;
    let Some(config) = get_supabase_config_from_conn(&conn).map_err(to_command_error)? else {
        return Ok(0);
    };

    if !config.is_connected || config.url.trim().is_empty() || config.anon_key.trim().is_empty() {
        return Ok(0);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("No se pudo preparar la conexión HTTP: {error}"))?;
    let plans = sync_upload_plans(Some(AUTO_SYNC_BATCH_SIZE), true);
    let mut total = 0;
    for (table, conflict_target, sql) in plans {
        total += upload_pending_batch_table(&conn, &client, &config, table, conflict_target, &sql)?;
    }
    if total > 0 && pendientes_antes > 0 {
        let pendientes_despues = count_sync_pending(&conn).map_err(to_command_error)?;
        let subidos = pendientes_antes.saturating_sub(pendientes_despues).max(total as i64);
        let ventas_texto = if ventas_pendientes_antes > 0 {
            format!(" Incluye hasta {ventas_pendientes_antes} ventas pendientes.")
        } else {
            String::new()
        };
        insertar_notificacion(
            &conn,
            "SINCRONIZACION",
            "INFO",
            "Sincronización completada",
            &format!("Se subieron con éxito {subidos} cambios locales a Supabase.{ventas_texto}"),
            None,
            None,
            &format!("sync:upload-ok:{}", current_timestamp_string()),
        )
        .map_err(to_command_error)?;
    }
    Ok(total)
}

fn update_sync_runtime_status(conn: &Connection, error: Option<&str>) -> Result<(), AppError> {
    let now = current_sqlite_timestamp(conn)?;
    if let Some(error) = error {
        let error = error.chars().take(700).collect::<String>();
        conn.execute(
            "
            INSERT INTO sync_runtime_status (id, ultimo_intento_at, ultimo_error_at, ultimo_error)
            VALUES (1, ?1, ?1, ?2)
            ON CONFLICT(id) DO UPDATE SET
              ultimo_intento_at = excluded.ultimo_intento_at,
              ultimo_error_at = excluded.ultimo_error_at,
              ultimo_error = excluded.ultimo_error
            ",
            params![now, error],
        )?;
    } else {
        conn.execute(
            "
            INSERT INTO sync_runtime_status (id, ultimo_intento_at, ultimo_exito_at, ultimo_error_at, ultimo_error)
            VALUES (1, ?1, ?1, NULL, NULL)
            ON CONFLICT(id) DO UPDATE SET
              ultimo_intento_at = excluded.ultimo_intento_at,
              ultimo_exito_at = excluded.ultimo_exito_at,
              ultimo_error_at = NULL,
              ultimo_error = NULL
            ",
            params![now],
        )?;
    }
    Ok(())
}

fn record_sync_worker_status(pool: &DbPool, error: Option<&str>) {
    match pool.get() {
        Ok(conn) => {
            if let Err(error) = update_sync_runtime_status(&conn, error) {
                eprintln!("[sync-worker] no se pudo actualizar estado runtime: {error}");
            }
        }
        Err(error) => eprintln!("[sync-worker] no se pudo obtener conexión para estado runtime: {error}"),
    }
}

fn start_sync_worker(pool: DbPool) {
    tauri::async_runtime::spawn(async move {
        loop {
            let pool = pool.clone();
            let result = tauri::async_runtime::spawn_blocking(move || {
                let mut errors = Vec::new();
                let uploaded = match run_auto_sync_once(&pool) {
                    Ok(total) => total,
                    Err(error) => {
                        errors.push(format!("subida: {error}"));
                        0
                    }
                };
                let downloaded = match run_auto_pull_once(&pool) {
                    Ok(total) => total,
                    Err(error) => {
                        errors.push(format!("bajada: {error}"));
                        0
                    }
                };

                if errors.is_empty() {
                    record_sync_worker_status(&pool, None);
                    Ok::<(usize, usize), String>((uploaded, downloaded))
                } else {
                    let error = errors.join(" | ");
                    record_sync_worker_status(&pool, Some(&error));
                    Err(error)
                }
            })
            .await;
            match result {
                Ok(Ok((uploaded, downloaded))) if uploaded > 0 || downloaded > 0 => {
                    eprintln!(
                        "[sync-worker] subida: {uploaded} registros, bajada: {downloaded} registros"
                    );
                }
                Ok(Ok(_)) => {}
                Ok(Err(error)) => {
                    eprintln!("[sync-worker] sincronización pospuesta: {error}");
                }
                Err(error) => {
                    eprintln!("[sync-worker] tarea de sincronización falló: {error}");
                }
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

fn normalize_supabase_url(url: &str) -> String {
    url.trim()
        .trim_end_matches('/')
        .trim_end_matches("/rest/v1")
        .trim_end_matches('/')
        .to_string()
}

fn assert_supabase_config_values(url: &str, anon_key: &str) -> Result<(String, String), AppError> {
    let url = normalize_supabase_url(url);
    let anon_key = anon_key.trim().to_string();

    if url.is_empty() || anon_key.is_empty() {
        return Err(AppError::Validation("Project URL y Anon Public Key son obligatorios.".to_string()));
    }
    if !url.starts_with("https://") {
        return Err(AppError::Validation("La URL de Supabase debe iniciar con https://.".to_string()));
    }

    Ok((url, anon_key))
}

fn supabase_request_builder(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    key: &str,
) -> reqwest::blocking::RequestBuilder {
    let builder = client.get(endpoint);
    supabase_auth_builder(builder, key)
}

fn supabase_auth_builder(
    builder: reqwest::blocking::RequestBuilder,
    key: &str,
) -> reqwest::blocking::RequestBuilder {
    let builder = builder.header("apikey", key);
    if key.starts_with("sb_publishable_") {
        builder
    } else {
        builder.header("Authorization", format!("Bearer {key}"))
    }
}

fn supabase_upsert_builder(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    key: &str,
) -> reqwest::blocking::RequestBuilder {
    supabase_auth_builder(client.post(endpoint), key)
        .header("Content-Type", "application/json")
        .header("Prefer", "resolution=merge-duplicates,return=representation")
}

fn get_supabase_config_from_conn(conn: &Connection) -> Result<Option<SupabaseConfig>, AppError> {
    conn.query_row(
        "
        SELECT url, anon_key, is_connected
        FROM supabase_config
        WHERE id = 1
        ",
        [],
        |row| {
            let is_connected: i64 = row.get(2)?;
            Ok(SupabaseConfig {
                url: row.get(0)?,
                anon_key: row.get(1)?,
                is_connected: is_connected == 1,
            })
        },
    )
    .optional()
    .map_err(AppError::from)
}

fn ensure_no_open_cash_sessions(conn: &Connection) -> AppResult<()> {
    let open_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM cajas_sesiones WHERE estado = 'ABIERTA'",
            [],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if open_count > 0 {
        return Err("No se puede restaurar o sobrescribir datos mientras existan cajas abiertas. Cierra los turnos primero.".to_string());
    }

    Ok(())
}

#[tauri::command]
fn get_supabase_config(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<SupabaseConfig> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    require_superadmin_or_initial_setup(&conn, &state_sesion)?;
    let config = get_supabase_config_from_conn(&conn)
        .map_err(to_command_error)?
        .unwrap_or(SupabaseConfig {
            url: String::new(),
            anon_key: String::new(),
            is_connected: false,
        });
    Ok(config)
}

#[tauri::command]
fn test_and_save_supabase_connect(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    url: String,
    anon_key: String,
) -> AppResult<SupabaseConfig> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    require_superadmin_or_initial_setup(&conn, &state_sesion)?;

    let (url, anon_key) = assert_supabase_config_values(&url, &anon_key).map_err(to_command_error)?;
    let health_url = format!("{url}/rest/v1/sucursales?select=id&limit=1");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|error| format!("No se pudo preparar la conexión HTTP: {error}"))?;

    let response = supabase_request_builder(&client, &health_url, &anon_key)
        .send()
        .map_err(|error| format!("No se pudo conectar con Supabase: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "Supabase rechazó la conexión. Código HTTP: {status}. {body}"
        ));
    }

    conn.execute(
        "
        INSERT INTO supabase_config (id, url, anon_key, is_connected)
        VALUES (1, ?1, ?2, 1)
        ON CONFLICT(id) DO UPDATE SET
            url = excluded.url,
            anon_key = excluded.anon_key,
            is_connected = 1
        ",
        params![url, anon_key],
    )
    .map_err(|error| map_write_error(error, "configuración de Supabase"))
    .map_err(to_command_error)?;

    get_supabase_config_from_conn(&conn)
        .map_err(to_command_error)?
        .ok_or_else(|| "No se pudo leer la configuración guardada.".to_string())
}

#[tauri::command]
fn disconnect_supabase(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<()> {
    require_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.execute(
        "
        INSERT INTO supabase_config (id, url, anon_key, is_connected)
        VALUES (1, '', '', 0)
        ON CONFLICT(id) DO UPDATE SET
            url = '',
            anon_key = '',
            is_connected = 0
        ",
        [],
    )
    .map_err(|error| map_write_error(error, "configuración de Supabase"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn crear_respaldo_local(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<String> {
    require_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let backup = build_backup(&conn).map_err(to_command_error)?;
    let file_name = format!("ferre_pos_backup_{}.json", backup.generado_at);
    let path = backup_dir().map_err(to_command_error)?.join(file_name);
    let content = serde_json::to_string_pretty(&backup)
        .map_err(|error| format!("No se pudo serializar el respaldo: {error}"))?;

    fs::write(&path, content).map_err(|error| format!("No se pudo guardar el respaldo: {error}"))?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn aplicar_respaldo_local(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    backup_json: String,
) -> AppResult<()> {
    require_superadmin(&state_sesion)?;
    if backup_json.trim().is_empty() {
        return Err("El archivo de respaldo está vacío.".to_string());
    }

    let backup: BackupLocal = serde_json::from_str(&backup_json)
        .map_err(|error| format!("El archivo de respaldo no es válido: {error}"))?;
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_no_open_cash_sessions(&conn)?;
    apply_backup_to_conn(&mut conn, backup).map_err(to_command_error)
}

#[tauri::command]
fn sincronizar_hacia_nube(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<SyncUploadResult> {
    require_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_sync_uuids(&conn).map_err(to_command_error)?;
    let config = get_supabase_config_from_conn(&conn)
        .map_err(to_command_error)?
        .ok_or_else(|| "Configura Supabase antes de sincronizar hacia la nube.".to_string())?;

    if !config.is_connected || config.url.trim().is_empty() || config.anon_key.trim().is_empty() {
        return Err("Supabase no está conectado.".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(45))
        .build()
        .map_err(|error| format!("No se pudo preparar la conexión HTTP: {error}"))?;

    let upload_plan = sync_upload_plans(None, true);

    let mut por_tabla = HashMap::new();
    let mut total_registros = 0;

    for (table, conflict_target, sql) in upload_plan {
        let uploaded = upload_pending_batch_table(&conn, &client, &config, table, conflict_target, &sql)?;
        if uploaded > 0 {
            por_tabla.insert(table.to_string(), uploaded);
            total_registros += uploaded;
        }
    }

    update_sync_runtime_status(&conn, None).map_err(to_command_error)?;

    Ok(SyncUploadResult {
        total_registros,
        por_tabla,
    })
}

#[tauri::command]
fn subir_base_local_completa_a_nube(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<SyncUploadResult> {
    require_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    ensure_sync_uuids(&conn).map_err(to_command_error)?;
    let config = get_supabase_config_from_conn(&conn)
        .map_err(to_command_error)?
        .ok_or_else(|| "Configura Supabase antes de subir la base local completa.".to_string())?;

    if !config.is_connected || config.url.trim().is_empty() || config.anon_key.trim().is_empty() {
        return Err("Supabase no está conectado.".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| format!("No se pudo preparar la conexión HTTP: {error}"))?;

    let upload_plan = sync_upload_plans(None, false);
    let mut por_tabla = HashMap::new();
    let mut total_registros = 0;

    for (table, conflict_target, sql) in upload_plan {
        let uploaded = upload_pending_batch_table(&conn, &client, &config, table, conflict_target, &sql)?;
        if uploaded > 0 {
            por_tabla.insert(table.to_string(), uploaded);
            total_registros += uploaded;
        }
    }

    update_sync_runtime_status(&conn, None).map_err(to_command_error)?;

    Ok(SyncUploadResult {
        total_registros,
        por_tabla,
    })
}

#[tauri::command]
fn sincronizar_desde_nube(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<()> {
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    require_superadmin_or_initial_setup(&conn, &state_sesion)?;
    ensure_no_open_cash_sessions(&conn)?;
    let config = get_supabase_config_from_conn(&conn)
        .map_err(to_command_error)?
        .ok_or_else(|| "Configura Supabase antes de sincronizar desde la nube.".to_string())?;

    if !config.is_connected || config.url.trim().is_empty() || config.anon_key.trim().is_empty() {
        return Err("Supabase no está conectado.".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("No se pudo preparar la conexión HTTP: {error}"))?;
    let mut tablas = HashMap::new();

    for table in BACKUP_TABLES {
        let rows = download_full_table_rows(&client, &config, table)?;
        tablas.insert((*table).to_string(), rows);
    }

    let backup = BackupLocal {
        version: "supabase-rest-v1".to_string(),
        generado_at: current_timestamp_string(),
        tablas,
    };
    apply_backup_to_conn(&mut conn, backup).map_err(to_command_error)?;
    update_sync_runtime_status(&conn, None).map_err(to_command_error)
}

#[tauri::command]
fn cancelar_venta(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    venta_id: String,
    usuario_autorizo_id: String,
    usuario_autorizo_clave: String,
    motivo_cancelacion: String,
    fecha_cancelacion: String,
) -> AppResult<()> {
    if venta_id.trim().is_empty() {
        return Err("Falta el identificador de la venta.".to_string());
    }
    if usuario_autorizo_id.trim().is_empty()
        || usuario_autorizo_clave.trim().is_empty()
        || motivo_cancelacion.trim().is_empty()
        || fecha_cancelacion.trim().is_empty()
    {
        return Err("La cancelación requiere usuario autorizador, contraseña/PIN, motivo y fecha.".to_string());
    }

    let actor = current_session_user(&state_sesion)?;
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let (_usuario_id, sucursal_id, metodo_pago, total, estado, cliente_id): (
        String,
        String,
        String,
        f64,
        String,
        Option<String>,
    ) = tx
        .query_row(
            "SELECT usuario_id, sucursal_id, metodo_pago, total, estado, cliente_id FROM ventas WHERE id = ?1",
            [&venta_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if estado == "CANCELADA" {
        return Err("La venta ya fue cancelada previamente.".to_string());
    }
    ensure_can_read_sucursal(&actor, &sucursal_id)?;

    let factura_estado: Option<String> = tx
        .query_row(
            "SELECT estado FROM facturas_emitidas WHERE venta_id = ?1 ORDER BY fecha_emision DESC LIMIT 1",
            [&venta_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if matches!(factura_estado.as_deref(), Some("TIMBRADA")) {
        return Err("No puedes cancelar una venta con factura TIMBRADA. Primero cancela el CFDI ante el SAT/PAC.".to_string());
    }

    let (usuario_autorizo_role, usuario_autorizo_sucursal_id, usuario_autorizo_password): (String, String, String) = tx
        .query_row(
            "SELECT role, sucursal_id, password_hash FROM usuarios WHERE id = ?1 AND eliminado = 0",
            [&usuario_autorizo_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if !matches!(usuario_autorizo_role.as_str(), "ADMIN" | "SUPERADMIN") {
        return Err("Solo un ADMIN o SUPERADMIN puede autorizar cancelaciones.".to_string());
    }
    if usuario_autorizo_role != "SUPERADMIN" && usuario_autorizo_sucursal_id != sucursal_id {
        return Err("El ADMIN autorizador solo puede cancelar ventas de su propia sucursal.".to_string());
    }
    let password_ok = verify_password_and_migrate(
        &tx,
        &usuario_autorizo_id,
        &usuario_autorizo_clave,
        &usuario_autorizo_password,
    )
    .map_err(to_command_error)?;
    if !password_ok {
        return Err("Contraseña/PIN de autorización inválido.".to_string());
    }

    let caja_id: String = tx
        .query_row(
            "
            SELECT id
            FROM cajas_sesiones
            WHERE sucursal_id = ?1
              AND usuario_id = ?2
              AND estado = 'ABIERTA'
            ORDER BY fecha_apertura DESC
            LIMIT 1
            ",
            params![&sucursal_id, &actor.id],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?
        .ok_or_else(|| "No se puede cancelar una venta sin caja ABIERTA para el usuario en sesión.".to_string())?;

    let affected_venta = tx.execute(
        "
        UPDATE ventas
        SET estado = 'CANCELADA',
            usuario_autorizo_cancelacion_id = ?2,
            motivo_cancelacion = ?3,
            fecha_cancelacion = ?4,
            sincronizado = 0,
            updated_at = datetime('now')
        WHERE id = ?1
          AND estado = 'COMPLETADA'
        ",
        params![
            venta_id,
            usuario_autorizo_id,
            motivo_cancelacion,
            fecha_cancelacion
        ],
    )
        .map_err(|error| map_write_error(error, "venta"))
        .map_err(to_command_error)?;
    if affected_venta != 1 {
        return Err("La venta cambió de estado antes de cancelar. Actualiza e intenta de nuevo.".to_string());
    }

    tx.execute(
        "
        UPDATE facturas_emitidas
        SET estado = 'CANCELADA',
            sincronizado = 0,
            updated_at = datetime('now')
        WHERE venta_id = ?1
          AND estado = 'PENDIENTE'
        ",
        [&venta_id],
    )
    .map_err(|error| map_write_error(error, "factura"))
    .map_err(to_command_error)?;

    let mut stmt = tx
        .prepare("SELECT producto_id, cantidad, costo_unitario_pactado FROM detalle_ventas WHERE venta_id = ?1")
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let iter = stmt
        .query_map([&venta_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut items: Vec<(String, f64, f64)> = Vec::new();
    for item in iter {
        items.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    drop(stmt);

    for (producto_id, cantidad, costo_unitario) in items {
        tx.execute(
            "
            INSERT INTO inventario_sucursal (
                producto_id, sucursal_id, stock, stock_minimo, costo_promedio,
                sincronizado, updated_at
            )
            VALUES (?1, ?2, ?3, 0, ?4, 0, datetime('now'))
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
              stock = inventario_sucursal.stock + excluded.stock,
              sincronizado = 0,
              updated_at = datetime('now')
            ",
            params![producto_id, sucursal_id, cantidad, costo_unitario],
        )
        .map_err(|error| map_write_error(error, "inventario"))
        .map_err(to_command_error)?;

        insertar_movimiento_inventario(
            &tx,
            &producto_id,
            &sucursal_id,
            "CANCELACION_VENTA",
            "VENTA",
            &venta_id,
            cantidad,
            Some(costo_unitario),
            Some(&usuario_autorizo_id),
            &fecha_cancelacion,
        )
        .map_err(to_command_error)?;
    }

    if metodo_pago == "CREDITO" {
        if let Some(cid) = cliente_id {
            tx.execute(
                "
                UPDATE clientes
                SET saldo_deudor = CASE
                    WHEN saldo_deudor - ?1 < 0 THEN 0
                    ELSE saldo_deudor - ?1
                END,
                sincronizado = 0,
                updated_at = datetime('now')
                WHERE id = ?2
                ",
                params![total, cid],
            )
            .map_err(|error| map_write_error(error, "cliente"))
            .map_err(to_command_error)?;
        }
    }

    let movimiento_id = format!("DEV-{}-{}", venta_id, fecha_cancelacion);
    tx.execute(
        "
        INSERT INTO caja_movimientos (id, sesion_id, tipo, monto, motivo, sync_uuid, sincronizado, updated_at)
        VALUES (?1, ?2, 'EGRESO', ?3, ?4, ?5, 0, datetime('now'))
        ",
        params![
            movimiento_id,
            caja_id,
            total,
            format!("DEVOLUCIÓN EN VENTA #{} - {}", venta_id, motivo_cancelacion),
            generate_uuid_like()
        ],
    )
    .map_err(|error| map_write_error(error, "movimiento de caja"))
    .map_err(to_command_error)?;

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn registrar_traspaso(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    traspaso: RegistrarTraspasoInput,
) -> AppResult<()> {
    let traspaso = RegistrarTraspasoInput {
        detalles: consolidar_detalles_traspaso(&traspaso.detalles),
        ..traspaso
    };
    validate_registrar_traspaso_input(&traspaso).map_err(to_command_error)?;

    let actor = require_admin_or_superadmin(&state_sesion)?;
    if actor.id != traspaso.usuario_id {
        return Err("No puedes registrar traspasos a nombre de otro usuario.".to_string());
    }
    if !is_superadmin(&actor) && actor.sucursal_id != traspaso.sucursal_origen_id {
        return Err("Operación inválida: solo puedes traspasar mercancía desde tu sucursal.".to_string());
    }

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let sucursal_origen_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1 AND eliminado = 0",
            [&traspaso.sucursal_origen_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let sucursal_destino_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1 AND eliminado = 0",
            [&traspaso.sucursal_destino_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if sucursal_origen_exists == 0 || sucursal_destino_exists == 0 {
        return Err("Sucursal origen o destino no existe.".to_string());
    }

    let usuario_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM usuarios WHERE id = ?1 AND eliminado = 0",
            [&traspaso.usuario_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if usuario_exists == 0 {
        return Err("El usuario que registra el traspaso no existe.".to_string());
    }

    tx.execute(
        "
        INSERT INTO traspasos (
            id, sucursal_origen_id, sucursal_destino_id, usuario_id, fecha, estado,
            sync_uuid, sincronizado, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, 'EN_TRANSITO', ?6, 0, datetime('now'))
        ",
        params![
            traspaso.id,
            traspaso.sucursal_origen_id,
            traspaso.sucursal_destino_id,
            traspaso.usuario_id,
            traspaso.fecha,
            generate_uuid_like()
        ],
    )
    .map_err(|error| map_write_error(error, "traspaso"))
    .map_err(to_command_error)?;

    for detalle in &traspaso.detalles {
        let (_stock_actual, costo_unitario) =
            inventario_costo_promedio(&tx, &detalle.producto_id, &traspaso.sucursal_origen_id)
                .map_err(to_command_error)?;

        tx.execute(
            "
            INSERT INTO detalle_traspasos (
                id, traspaso_id, producto_id, cantidad,
                sync_uuid, sincronizado, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, 0, datetime('now'))
            ",
            params![
                detalle.id,
                traspaso.id,
                detalle.producto_id,
                detalle.cantidad,
                generate_uuid_like()
            ],
        )
        .map_err(|error| map_write_error(error, "detalle de traspaso"))
        .map_err(to_command_error)?;

        let affected = tx
            .execute(
            "
            UPDATE inventario_sucursal
            SET stock = stock - ?1,
                sincronizado = 0,
                updated_at = datetime('now')
            WHERE producto_id = ?2 AND sucursal_id = ?3 AND stock >= ?1
            ",
            params![detalle.cantidad, detalle.producto_id, traspaso.sucursal_origen_id],
            )
            .map_err(|error| map_write_error(error, "inventario origen"))
            .map_err(to_command_error)?;

        if affected != 1 {
            return Err(format!(
                "Stock insuficiente para producto {} en sucursal origen. Operación cancelada.",
                detalle.producto_id
            ));
        }

        insertar_movimiento_inventario(
            &tx,
            &detalle.producto_id,
            &traspaso.sucursal_origen_id,
            "TRASPASO_SALIDA",
            "TRASPASO",
            &traspaso.id,
            -detalle.cantidad,
            Some(costo_unitario),
            Some(&traspaso.usuario_id),
            &traspaso.fecha,
        )
        .map_err(to_command_error)?;
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn recibir_traspaso(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    input: RecibirTraspasoInput,
) -> AppResult<()> {
    if input.traspaso_id.trim().is_empty()
        || input.usuario_recibio_id.trim().is_empty()
        || input.fecha_recepcion.trim().is_empty()
    {
        return Err("Datos incompletos para recibir el traspaso.".to_string());
    }

    let usuario_sesion = current_session_user(&state_sesion)?;
    if usuario_sesion.id != input.usuario_recibio_id {
        return Err("Operación inválida: el usuario receptor debe coincidir con la sesión activa.".to_string());
    }

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let (sucursal_destino_id, estado): (String, String) = tx
        .query_row(
            "SELECT sucursal_destino_id, estado FROM traspasos WHERE id = ?1",
            [&input.traspaso_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if estado != "EN_TRANSITO" {
        return Err(format!("El traspaso no puede recibirse porque está en estado {estado}."));
    }

    if !is_superadmin(&usuario_sesion) && usuario_sesion.sucursal_id != sucursal_destino_id {
        return Err("Operación inválida: solo la sucursal destino puede recibir este traspaso.".to_string());
    }

    let usuario_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM usuarios WHERE id = ?1 AND eliminado = 0",
            [&input.usuario_recibio_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if usuario_exists == 0 {
        return Err("El usuario receptor no existe.".to_string());
    }

    let mut stmt = tx
        .prepare(
            "
            SELECT dt.producto_id, dt.cantidad,
                   COALESCE((
                       SELECT mi.costo_unitario
                       FROM movimientos_inventario mi
                       WHERE mi.referencia_tipo = 'TRASPASO'
                         AND mi.referencia_id = dt.traspaso_id
                         AND mi.producto_id = dt.producto_id
                         AND mi.tipo = 'TRASPASO_SALIDA'
                       ORDER BY mi.fecha DESC
                       LIMIT 1
                   ), p.costo_promedio, p.precio_costo, 0) AS costo_unitario
            FROM detalle_traspasos dt
            INNER JOIN productos p ON p.id = dt.producto_id
            WHERE dt.traspaso_id = ?1
            ",
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let rows = stmt
        .query_map([&input.traspaso_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, f64>(2)?,
            ))
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let mut detalles = Vec::new();
    for row in rows {
        detalles.push(row.map_err(AppError::from).map_err(to_command_error)?);
    }
    drop(stmt);

    for (producto_id, cantidad, costo_unitario) in detalles {
        tx.execute(
            "
            INSERT INTO inventario_sucursal (
                producto_id, sucursal_id, stock, stock_minimo, costo_promedio,
                sincronizado, updated_at
            )
            VALUES (?1, ?2, ?3, 0, ?4, 0, datetime('now'))
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
              stock = inventario_sucursal.stock + excluded.stock,
              costo_promedio = CASE
                  WHEN inventario_sucursal.stock + excluded.stock > 0 THEN
                      ((inventario_sucursal.stock * inventario_sucursal.costo_promedio) + (excluded.stock * excluded.costo_promedio))
                      / (inventario_sucursal.stock + excluded.stock)
                  ELSE excluded.costo_promedio
              END,
              sincronizado = 0,
              updated_at = datetime('now')
            ",
            params![producto_id, sucursal_destino_id, cantidad, costo_unitario],
        )
        .map_err(|error| map_write_error(error, "inventario destino"))
        .map_err(to_command_error)?;

        insertar_movimiento_inventario(
            &tx,
            &producto_id,
            &sucursal_destino_id,
            "TRASPASO_ENTRADA",
            "TRASPASO",
            &input.traspaso_id,
            cantidad,
            Some(costo_unitario),
            Some(&input.usuario_recibio_id),
            &input.fecha_recepcion,
        )
        .map_err(to_command_error)?;
    }

    let affected = tx.execute(
        "
        UPDATE traspasos
        SET estado = 'RECIBIDO',
            usuario_recibio_id = ?2,
            fecha_recepcion = ?3,
            observaciones_recepcion = ?4,
            sincronizado = 0,
            updated_at = datetime('now')
        WHERE id = ?1 AND estado = 'EN_TRANSITO'
        ",
        params![
            input.traspaso_id,
            input.usuario_recibio_id,
            input.fecha_recepcion,
            input.observaciones_recepcion.unwrap_or_default()
        ],
    )
    .map_err(|error| map_write_error(error, "traspaso"))
    .map_err(to_command_error)?;
    if affected != 1 {
        return Err("El traspaso cambió de estado antes de confirmar la recepción. Actualiza e intenta de nuevo.".to_string());
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn get_historial_traspasos(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Vec<HistorialTraspaso>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
    let sucursal_id = scoped_sucursal_for_read(&user, None);
    let mut sql = String::from(
        "
            SELECT
              t.id,
              t.sucursal_origen_id,
              so.nombre,
              t.sucursal_destino_id,
              sd.nombre,
              t.usuario_id,
              u.nombre,
              t.fecha,
              t.estado,
              t.usuario_recibio_id,
              ur.nombre,
              t.fecha_recepcion,
              t.observaciones_recepcion
            FROM traspasos t
            INNER JOIN sucursales so ON so.id = t.sucursal_origen_id
            INNER JOIN sucursales sd ON sd.id = t.sucursal_destino_id
            INNER JOIN usuarios u ON u.id = t.usuario_id
            LEFT JOIN usuarios ur ON ur.id = t.usuario_recibio_id
        ",
    );
    let mut params_vec: Vec<String> = Vec::new();
    if let Some(value) = sucursal_id {
        sql.push_str(" WHERE t.sucursal_origen_id = ?1 OR t.sucursal_destino_id = ?1");
        params_vec.push(value);
    }
    sql.push_str(" ORDER BY t.fecha DESC");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map(params_from_iter(params_vec.iter()), |row| {
            Ok(HistorialTraspaso {
                id: row.get(0)?,
                sucursal_origen_id: row.get(1)?,
                sucursal_origen_nombre: row.get(2)?,
                sucursal_destino_id: row.get(3)?,
                sucursal_destino_nombre: row.get(4)?,
                usuario_id: row.get(5)?,
                usuario_nombre: row.get(6)?,
                fecha: row.get(7)?,
                estado: row.get(8)?,
                usuario_recibio_id: row.get(9)?,
                usuario_recibio_nombre: row.get(10)?,
                fecha_recepcion: row.get(11)?,
                observaciones_recepcion: row.get(12)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut historial = Vec::new();
    for item in iter {
        historial.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(historial)
}

#[tauri::command]
fn get_historial_traspasos_page(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    page: i64,
    page_size: i64,
    estado: Option<String>,
) -> AppResult<HistorialTraspasosPage> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
    let sucursal_id = scoped_sucursal_for_read(&user, None);
    let (page, page_size) = normalize_page_args(page, page_size);
    let offset = page * page_size;

    let mut conditions: Vec<String> = Vec::new();
    let mut params_vec: Vec<String> = Vec::new();
    if let Some(value) = sucursal_id {
        conditions.push("(t.sucursal_origen_id = ? OR t.sucursal_destino_id = ?)".to_string());
        params_vec.push(value.clone());
        params_vec.push(value);
    }
    if let Some(value) = estado {
        let estado = value.trim().to_uppercase();
        if !estado.is_empty() {
            conditions.push("t.estado = ?".to_string());
            params_vec.push(estado);
        }
    }
    let where_sql = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM traspasos t{where_sql}");
    let total: i64 = conn
        .query_row(&count_sql, params_from_iter(params_vec.iter()), |row| row.get(0))
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut select_params = params_vec.clone();
    select_params.push(page_size.to_string());
    select_params.push(offset.to_string());
    let select_sql = format!(
        "
            SELECT
              t.id,
              t.sucursal_origen_id,
              so.nombre,
              t.sucursal_destino_id,
              sd.nombre,
              t.usuario_id,
              u.nombre,
              t.fecha,
              t.estado,
              t.usuario_recibio_id,
              ur.nombre,
              t.fecha_recepcion,
              t.observaciones_recepcion
            FROM traspasos t
            INNER JOIN sucursales so ON so.id = t.sucursal_origen_id
            INNER JOIN sucursales sd ON sd.id = t.sucursal_destino_id
            INNER JOIN usuarios u ON u.id = t.usuario_id
            LEFT JOIN usuarios ur ON ur.id = t.usuario_recibio_id
            {where_sql}
            ORDER BY t.fecha DESC
            LIMIT ? OFFSET ?
        "
    );

    let mut stmt = conn
        .prepare(&select_sql)
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map(params_from_iter(select_params.iter()), |row| {
            Ok(HistorialTraspaso {
                id: row.get(0)?,
                sucursal_origen_id: row.get(1)?,
                sucursal_origen_nombre: row.get(2)?,
                sucursal_destino_id: row.get(3)?,
                sucursal_destino_nombre: row.get(4)?,
                usuario_id: row.get(5)?,
                usuario_nombre: row.get(6)?,
                fecha: row.get(7)?,
                estado: row.get(8)?,
                usuario_recibio_id: row.get(9)?,
                usuario_recibio_nombre: row.get(10)?,
                fecha_recepcion: row.get(11)?,
                observaciones_recepcion: row.get(12)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut rows = Vec::new();
    for item in iter {
        rows.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }

    Ok(HistorialTraspasosPage { rows, total })
}

#[tauri::command]
fn registrar_merma_ajuste(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    movimiento: RegistrarMermaAjusteInput,
) -> AppResult<()> {
    let actor = require_admin_or_superadmin(&state_sesion)?;
    if actor.id != movimiento.usuario_id {
        return Err("No puedes registrar ajustes a nombre de otro usuario.".to_string());
    }
    ensure_can_read_sucursal(&actor, &movimiento.sucursal_id)?;

    validate_registrar_merma_ajuste_input(&movimiento).map_err(to_command_error)?;

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let producto_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM productos WHERE id = ?1 AND eliminado = 0",
            [&movimiento.producto_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if producto_exists == 0 {
        return Err("El producto seleccionado no existe.".to_string());
    }

    let sucursal_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1 AND eliminado = 0",
            [&movimiento.sucursal_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if sucursal_exists == 0 {
        return Err("La sucursal seleccionada no existe.".to_string());
    }

    let usuario_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM usuarios WHERE id = ?1 AND eliminado = 0",
            [&movimiento.usuario_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if usuario_exists == 0 {
        return Err("El usuario que registra el ajuste no existe.".to_string());
    }

    let (_stock_actual, costo_unitario) =
        inventario_costo_promedio(&tx, &movimiento.producto_id, &movimiento.sucursal_id)
            .map_err(to_command_error)?;

    let tipo_movimiento = match movimiento.tipo_movimiento.as_str() {
        "AJUSTE" => "AJUSTE_SALIDA",
        value => value,
    };

    let tipo_kardex = match tipo_movimiento {
        "AJUSTE_ENTRADA" => "AJUSTE_ENTRADA",
        "AJUSTE_SALIDA" => "AJUSTE_SALIDA",
        _ => "MERMA",
    };
    let cantidad_kardex = if tipo_kardex == "AJUSTE_ENTRADA" {
        movimiento.cantidad
    } else {
        -movimiento.cantidad
    };

    if tipo_kardex == "AJUSTE_ENTRADA" {
        tx.execute(
            "
            INSERT INTO inventario_sucursal (
                producto_id, sucursal_id, stock, stock_minimo, costo_promedio,
                sincronizado, updated_at
            )
            VALUES (?1, ?2, ?3, 0, ?4, 0, datetime('now'))
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
              stock = inventario_sucursal.stock + excluded.stock,
              sincronizado = 0,
              updated_at = datetime('now')
            ",
            params![movimiento.producto_id, movimiento.sucursal_id, movimiento.cantidad, costo_unitario],
        )
        .map_err(|error| map_write_error(error, "inventario"))
        .map_err(to_command_error)?;
    } else {
        let affected = tx
            .execute(
                "
                UPDATE inventario_sucursal
                SET stock = stock - ?1,
                    sincronizado = 0,
                    updated_at = datetime('now')
                WHERE producto_id = ?2 AND sucursal_id = ?3 AND stock >= ?1
                ",
                params![movimiento.cantidad, movimiento.producto_id, movimiento.sucursal_id],
            )
            .map_err(|error| map_write_error(error, "inventario"))
            .map_err(to_command_error)?;
        if affected != 1 {
            return Err(format!(
                "Stock insuficiente. Operación cancelada para producto {}.",
                movimiento.producto_id
            ));
        }
    }

    tx.execute(
        "
        INSERT INTO mermas_ajustes (
            id, producto_id, sucursal_id, usuario_id, cantidad, tipo_movimiento, motivo, fecha,
            sync_uuid, sincronizado, updated_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, datetime('now'))
        ",
        params![
            movimiento.id,
            movimiento.producto_id,
            movimiento.sucursal_id,
            movimiento.usuario_id,
            movimiento.cantidad,
            tipo_movimiento,
            movimiento.motivo.trim(),
            movimiento.fecha,
            generate_uuid_like()
        ],
    )
    .map_err(|error| map_write_error(error, "merma/ajuste"))
    .map_err(to_command_error)?;

    insertar_movimiento_inventario(
        &tx,
        &movimiento.producto_id,
        &movimiento.sucursal_id,
        tipo_kardex,
        "MERMA_AJUSTE",
        &movimiento.id,
        cantidad_kardex,
        Some(costo_unitario),
        Some(&movimiento.usuario_id),
        &movimiento.fecha,
    )
    .map_err(to_command_error)?;

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn get_historial_mermas(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Vec<HistorialMerma>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
    let sucursal_id = scoped_sucursal_for_read(&user, None);
    let mut sql = String::from(
        "
            SELECT
              m.id,
              m.producto_id,
              p.descripcion,
              p.marca,
              m.sucursal_id,
              s.nombre,
              m.usuario_id,
              u.nombre,
              m.cantidad,
              m.tipo_movimiento,
              m.motivo,
              m.fecha,
              COALESCE(mi.costo_unitario, p.costo_promedio, p.precio_costo),
              (m.cantidad * COALESCE(mi.costo_unitario, p.costo_promedio, p.precio_costo)) AS costo_total
            FROM mermas_ajustes m
            INNER JOIN productos p ON p.id = m.producto_id
            INNER JOIN sucursales s ON s.id = m.sucursal_id
            INNER JOIN usuarios u ON u.id = m.usuario_id
            LEFT JOIN movimientos_inventario mi
              ON mi.referencia_tipo = 'MERMA_AJUSTE'
             AND mi.referencia_id = m.id
             AND mi.producto_id = m.producto_id
        ",
    );
    let mut params_vec: Vec<String> = Vec::new();
    if let Some(value) = sucursal_id {
        sql.push_str(" WHERE m.sucursal_id = ?1");
        params_vec.push(value);
    }
    sql.push_str(" ORDER BY m.fecha DESC");

    let mut stmt = conn
        .prepare(&sql)
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map(params_from_iter(params_vec.iter()), |row| {
            Ok(HistorialMerma {
                id: row.get(0)?,
                producto_id: row.get(1)?,
                producto_descripcion: row.get(2)?,
                marca: row.get(3)?,
                sucursal_id: row.get(4)?,
                sucursal_nombre: row.get(5)?,
                usuario_id: row.get(6)?,
                usuario_nombre: row.get(7)?,
                cantidad: row.get(8)?,
                tipo_movimiento: row.get(9)?,
                motivo: row.get(10)?,
                fecha: row.get(11)?,
                costo_unitario: row.get(12)?,
                costo_total_perdido: row.get(13)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut historial = Vec::new();
    for item in iter {
        historial.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    Ok(historial)
}

#[tauri::command]
fn get_historial_mermas_page(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    page: i64,
    page_size: i64,
) -> AppResult<HistorialMermasPage> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
    let sucursal_id = scoped_sucursal_for_read(&user, None);
    let (page, page_size) = normalize_page_args(page, page_size);
    let offset = page * page_size;

    let mut where_sql = String::new();
    let mut params_vec: Vec<String> = Vec::new();
    if let Some(value) = sucursal_id {
        where_sql.push_str(" WHERE m.sucursal_id = ?");
        params_vec.push(value);
    }

    let count_sql = format!("SELECT COUNT(*) FROM mermas_ajustes m{where_sql}");
    let total: i64 = conn
        .query_row(&count_sql, params_from_iter(params_vec.iter()), |row| row.get(0))
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut select_params = params_vec.clone();
    select_params.push(page_size.to_string());
    select_params.push(offset.to_string());
    let select_sql = format!(
        "
            SELECT
              m.id,
              m.producto_id,
              p.descripcion,
              p.marca,
              m.sucursal_id,
              s.nombre,
              m.usuario_id,
              u.nombre,
              m.cantidad,
              m.tipo_movimiento,
              m.motivo,
              m.fecha,
              COALESCE(mi.costo_unitario, p.costo_promedio, p.precio_costo),
              (m.cantidad * COALESCE(mi.costo_unitario, p.costo_promedio, p.precio_costo)) AS costo_total
            FROM mermas_ajustes m
            INNER JOIN productos p ON p.id = m.producto_id
            INNER JOIN sucursales s ON s.id = m.sucursal_id
            INNER JOIN usuarios u ON u.id = m.usuario_id
            LEFT JOIN movimientos_inventario mi
              ON mi.referencia_tipo = 'MERMA_AJUSTE'
             AND mi.referencia_id = m.id
             AND mi.producto_id = m.producto_id
            {where_sql}
            ORDER BY m.fecha DESC
            LIMIT ? OFFSET ?
        "
    );

    let mut stmt = conn
        .prepare(&select_sql)
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let iter = stmt
        .query_map(params_from_iter(select_params.iter()), |row| {
            Ok(HistorialMerma {
                id: row.get(0)?,
                producto_id: row.get(1)?,
                producto_descripcion: row.get(2)?,
                marca: row.get(3)?,
                sucursal_id: row.get(4)?,
                sucursal_nombre: row.get(5)?,
                usuario_id: row.get(6)?,
                usuario_nombre: row.get(7)?,
                cantidad: row.get(8)?,
                tipo_movimiento: row.get(9)?,
                motivo: row.get(10)?,
                fecha: row.get(11)?,
                costo_unitario: row.get(12)?,
                costo_total_perdido: row.get(13)?,
            })
        })
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut rows = Vec::new();
    for item in iter {
        rows.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }

    Ok(HistorialMermasPage { rows, total })
}

#[tauri::command]
fn registrar_venta(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    venta: RegistrarVentaInput,
) -> AppResult<()> {
    let venta = RegistrarVentaInput {
        detalles: consolidar_detalles_venta(&venta.detalles).map_err(to_command_error)?,
        ..venta
    };
    validate_registrar_venta_input(&venta).map_err(to_command_error)?;

    let actor = current_session_user(&state_sesion)?;
    if actor.id != venta.usuario_id || actor.sucursal_id != venta.sucursal_id {
        return Err("No puedes registrar ventas para otro usuario o sucursal.".to_string());
    }

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let usuario_role: String = tx
        .query_row(
            "SELECT role FROM usuarios WHERE id = ?1 AND eliminado = 0",
            [&venta.usuario_id],
            |row| row.get(0),
        )
        .map_err(|_| "El usuario de la venta ya no existe.".to_string())?;

    let sucursal_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1 AND eliminado = 0",
            [&venta.sucursal_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if sucursal_exists == 0 {
        return Err("La sucursal de la venta ya no existe.".to_string());
    }

    let caja_abierta: i64 = tx
        .query_row(
            "
            SELECT COUNT(*)
            FROM cajas_sesiones
            WHERE usuario_id = ?1 AND sucursal_id = ?2 AND estado = 'ABIERTA'
            ",
            params![venta.usuario_id, venta.sucursal_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if caja_abierta == 0 {
        return Err("No puedes vender sin una caja ABIERTA. Abre caja para continuar.".to_string());
    }

    let mut detalles_calculados: Vec<(String, String, f64, f64, f64, String, f64, f64, bool)> = Vec::new();
    let mut total_centavos = 0_i64;
    for detalle in &venta.detalles {
        let mut producto: ProductoConStock = tx
            .query_row(
                "
                SELECT
                    p.id,
                    p.codigo_barras,
                    p.codigo_proveedor,
                    p.proveedor_id,
                    p.clave_producto,
                    p.descripcion,
                    p.marca,
                    p.categoria,
                    p.unidad,
                    COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS precio_costo_local,
                    COALESCE(NULLIF(i.costo_promedio, 0), NULLIF(p.costo_promedio, 0), p.precio_costo) AS costo_promedio,
                    COALESCE(NULLIF(i.precio_venta, 0), p.precio_venta) AS precio_venta_local,
                    p.sat_clave_prod_serv,
                    p.sat_clave_unidad,
                    i.sucursal_id,
                    i.stock,
                    i.stock_minimo,
                    COALESCE(p.precio_1, 0),
                    COALESCE(p.precio_2, 0),
                    COALESCE(p.precio_3, 0),
                    COALESCE(p.precio_4, 0),
                    COALESCE(p.mayoreo_apartir, 0),
                    COALESCE(p.a_granel, 0),
                    COALESCE(p.no_en_catalogo, 0),
                    COALESCE(p.ventas_negativas, 0),
                    p.caducidad,
                    TRIM(COALESCE(p.fotos, '')),
                    TRIM(COALESCE(p.descripcion_catalogo, ''))
                FROM productos p
                INNER JOIN inventario_sucursal i ON i.producto_id = p.id
                WHERE p.id = ?1
                  AND i.sucursal_id = ?2
                  AND p.eliminado = 0
                  AND i.eliminado = 0
                ",
                params![detalle.producto_id, venta.sucursal_id],
                |row| {
                    Ok(ProductoConStock {
                        id: row.get(0)?,
                        codigo_barras: row.get(1)?,
                        codigo_proveedor: row.get(2)?,
                        proveedor_id: row.get(3)?,
                        clave_producto: row.get(4)?,
                        descripcion: row.get(5)?,
                        marca: row.get(6)?,
                        categoria: row.get(7)?,
                        unidad: row.get(8)?,
                        precio_costo: row.get(9)?,
                        costo_promedio: row.get(10)?,
                        precio_venta: row.get(11)?,
                        sat_clave_prod_serv: row.get(12)?,
                        sat_clave_unidad: row.get(13)?,
                        sucursal_id: row.get(14)?,
                        stock: row.get(15)?,
                        stock_minimo: row.get(16)?,
                        precio_original: None,
                        precio_descontado: None,
                        nombre_promo: None,
                        promocion_id: None,
                        promo_tipo_descuento: None,
                        promo_valor: None,
                        precio_1: row.get(17)?,
                        precio_2: row.get(18)?,
                        precio_3: row.get(19)?,
                        precio_4: row.get(20)?,
                        mayoreo_apartir: row.get(21)?,
                        a_granel: row.get::<_, i64>(22)? == 1,
                        no_en_catalogo: row.get::<_, i64>(23)? == 1,
                        ventas_negativas: row.get::<_, i64>(24)? == 1,
                        caducidad: row.get(25)?,
                        fotos: row.get(26)?,
                        descripcion_catalogo: row.get(27)?,
                    })
                },
            )
            .map_err(|_| format!("El producto {} no existe en el inventario de la sucursal.", detalle.producto_id))?;

        let es_venta_diversa = producto.clave_producto == "VENTA_DIVERSA";
        let (precio_servidor, tipo_precio_vendido, precio_original, descuento_aplicado) = if es_venta_diversa {
            if !is_admin_or_superadmin_role(&usuario_role) {
                return Err("Solo ADMIN o SUPERADMIN pueden cobrar artículo diverso.".to_string());
            }
            if detalle.precio_venta_pactado <= 0.0 {
                return Err("El artículo diverso requiere un precio mayor a cero.".to_string());
            }
            let precio = round_money(detalle.precio_venta_pactado);
            (precio, "DIVERSO".to_string(), precio, 0.0)
        } else {
            let (precio_base, tipo_base) = precio_base_por_cantidad(&producto, detalle.cantidad);
            producto.precio_venta = precio_base;
            apply_active_promotion(&tx, &mut producto).map_err(to_command_error)?;
            let precio_final = round_money(producto.precio_venta);
            let tipo = if producto.promocion_id.is_some() {
                format!("{tipo_base}+PROMO")
            } else {
                tipo_base
            };
            (precio_final, tipo, precio_base, round_money((precio_base - precio_final).max(0.0)))
        };
        let costo_unitario = round_money(producto.costo_promedio);
        total_centavos += money_to_cents(detalle.cantidad * precio_servidor);
        detalles_calculados.push((
            detalle.id.clone(),
            detalle.producto_id.clone(),
            detalle.cantidad,
            precio_servidor,
            costo_unitario,
            tipo_precio_vendido,
            precio_original,
            descuento_aplicado,
            producto.ventas_negativas,
        ));
    }
    let total = cents_to_money(total_centavos);
    let (efectivo_recibido, cambio_entregado) = if venta.metodo_pago == "EFECTIVO" {
        let recibido = normalize_money(
            venta.efectivo_recibido.unwrap_or(total),
            "El efectivo recibido",
            false,
        )
        .map_err(to_command_error)?;
        if recibido + 0.0001 < total {
            return Err("El efectivo recibido es insuficiente para cubrir el total de la venta.".to_string());
        }
        let cambio = round_money(recibido - total);
        if let Some(cambio_cliente) = venta.cambio_entregado {
            if (round_money(cambio_cliente) - cambio).abs() > 0.01 {
                return Err("El cambio recibido no coincide con el total calculado por el backend.".to_string());
            }
        }
        (Some(recibido), Some(cambio))
    } else {
        (None, None)
    };

    if venta.metodo_pago == "CREDITO" {
        if !matches!(usuario_role.as_str(), "ADMIN" | "SUPERADMIN") {
            return Err("Solo ADMIN o SUPERADMIN pueden registrar ventas a crédito.".to_string());
        }
        let cliente_id = venta
            .cliente_id
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Selecciona un cliente para venta a crédito.".to_string())?;

        let (limite_credito, saldo_deudor): (f64, f64) = tx
            .query_row(
                "SELECT limite_credito, saldo_deudor FROM clientes WHERE id = ?1 AND eliminado = 0",
                [&cliente_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(|_| "El cliente seleccionado no existe.".to_string())?;

        if limite_credito <= 0.0 {
            return Err("El cliente no tiene crédito autorizado.".to_string());
        }
        if round_money(saldo_deudor + total) > round_money(limite_credito) {
            return Err("La venta supera el límite de crédito del cliente.".to_string());
        }

        let affected = tx.execute(
            "
            UPDATE clientes
            SET saldo_deudor = ROUND(saldo_deudor + ?1, 2),
                sincronizado = 0,
                updated_at = datetime('now')
            WHERE id = ?2
              AND eliminado = 0
              AND limite_credito > 0
              AND ROUND(saldo_deudor + ?1, 2) <= ROUND(limite_credito, 2)
            ",
            params![total, cliente_id],
        )
        .map_err(|error| map_write_error(error, "cliente"))
        .map_err(to_command_error)?;
        if affected != 1 {
            return Err("No se pudo autorizar el crédito porque el saldo del cliente cambió. Actualiza e intenta de nuevo.".to_string());
        }
    }

    tx.execute(
        "
        INSERT INTO ventas (
            id, usuario_id, sucursal_id, fecha, total, metodo_pago, efectivo_recibido,
            cambio_entregado, cliente_id, estado, cliente_rapido_nombre, cliente_rapido_telefono,
            cliente_rapido_domicilio, requiere_factura, sync_uuid, sincronizado, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'COMPLETADA', ?10, ?11, ?12, ?13, ?14, 0, datetime('now'))
        ",
        params![
            venta.id,
            venta.usuario_id,
            venta.sucursal_id,
            venta.fecha,
            total,
            venta.metodo_pago,
            efectivo_recibido,
            cambio_entregado,
            venta.cliente_id,
            venta
                .cliente_rapido_nombre
                .as_ref()
                .map(|v| normalize_title_trim(v))
                .filter(|v| !v.is_empty()),
            venta
                .cliente_rapido_telefono
                .as_ref()
                .map(|v| normalize_plain_trim(v))
                .filter(|v| !v.is_empty()),
            venta
                .cliente_rapido_domicilio
                .as_ref()
                .map(|v| normalize_title_trim(v))
                .filter(|v| !v.is_empty()),
            if venta.requiere_factura { 1 } else { 0 },
            generate_uuid_like()
        ],
    )
    .map_err(|error| map_write_error(error, "venta"))
    .map_err(to_command_error)?;

    for (
        detalle_id,
        producto_id,
        cantidad,
        precio_servidor,
        costo_unitario,
        tipo_precio_vendido,
        precio_original,
        descuento_aplicado,
        permite_ventas_negativas,
    ) in &detalles_calculados {
        tx.execute(
            "
            INSERT INTO detalle_ventas (
                id, venta_id, producto_id, cantidad, precio_venta_pactado, costo_unitario_pactado,
                tipo_precio_vendido, precio_original, descuento_aplicado, sync_uuid, sincronizado, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, datetime('now'))
            ",
            params![
                detalle_id,
                venta.id,
                producto_id,
                cantidad,
                precio_servidor,
                costo_unitario,
                tipo_precio_vendido,
                precio_original,
                descuento_aplicado,
                generate_uuid_like()
            ],
        )
        .map_err(|error| map_write_error(error, "detalle de venta"))
        .map_err(to_command_error)?;

        let affected = if *permite_ventas_negativas {
            tx.execute(
                "
                UPDATE inventario_sucursal
                SET stock = stock - ?1,
                    sincronizado = 0,
                    updated_at = datetime('now')
                WHERE producto_id = ?2 AND sucursal_id = ?3
                ",
                params![cantidad, producto_id, venta.sucursal_id],
            )
        } else {
            tx.execute(
                "
                UPDATE inventario_sucursal
                SET stock = stock - ?1,
                    sincronizado = 0,
                    updated_at = datetime('now')
                WHERE producto_id = ?2 AND sucursal_id = ?3 AND stock >= ?1
                ",
                params![cantidad, producto_id, venta.sucursal_id],
            )
        }
        .map_err(|error| map_write_error(error, "inventario"))
        .map_err(to_command_error)?;

        if affected != 1 {
            return Err(format!(
                "Stock insuficiente para producto {}. Operación cancelada.",
                producto_id
            ));
        }

        insertar_movimiento_inventario(
            &tx,
            producto_id,
            &venta.sucursal_id,
            "VENTA",
            "VENTA",
            &venta.id,
            -*cantidad,
            Some(*costo_unitario),
            Some(&venta.usuario_id),
            &venta.fecha,
        )
        .map_err(to_command_error)?;
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn create_sucursal(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    sucursal: Sucursal,
) -> AppResult<()> {
    require_superadmin(&state_sesion)?;
    validate_sucursal(&sucursal).map_err(to_command_error)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.execute(
        "INSERT INTO sucursales (id, nombre, direccion, telefono, codigo_postal) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![sucursal.id, sucursal.nombre, sucursal.direccion, sucursal.telefono, sucursal.codigo_postal],
    )
    .map_err(|error| map_write_error(error, "sucursal"))
    .map_err(to_command_error)?;

    Ok(())
}

#[tauri::command]
fn update_sucursal(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
    sucursal: Sucursal,
) -> AppResult<()> {
    require_superadmin(&state_sesion)?;
    validate_sucursal(&sucursal).map_err(to_command_error)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let affected = conn
        .execute(
            "UPDATE sucursales SET nombre = ?1, direccion = ?2, telefono = ?3, codigo_postal = ?4 WHERE id = ?5 AND eliminado = 0",
            params![sucursal.nombre, sucursal.direccion, sucursal.telefono, sucursal.codigo_postal, id],
        )
        .map_err(|error| map_write_error(error, "sucursal"))
        .map_err(to_command_error)?;

    if affected == 0 {
        return Err("No se encontró la sucursal que intentas actualizar.".to_string());
    }

    Ok(())
}

#[tauri::command]
fn delete_sucursal(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    id: String,
) -> AppResult<()> {
    require_superadmin(&state_sesion)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let active_users: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM usuarios WHERE sucursal_id = ?1 AND eliminado = 0",
            [&id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if active_users > 0 {
        return Err("No se puede eliminar la sucursal porque tiene usuarios activos.".to_string());
    }

    let affected = conn
        .execute(
            "UPDATE sucursales SET eliminado = 1, sincronizado = 0, updated_at = datetime('now') WHERE id = ?1 AND eliminado = 0",
            [&id],
        )
        .map_err(|error| map_write_error(error, "sucursal"))
        .map_err(to_command_error)?;

    if affected == 0 {
        return Err("No se encontró la sucursal que intentas eliminar.".to_string());
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let manager = SqliteConnectionManager::file("ferreteria.db").with_init(|conn| {
        conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA busy_timeout = 5000;
            PRAGMA foreign_keys = ON;
            ",
        )
    });

    let pool = Pool::builder()
        .max_size(4)
        .build(manager)
        .expect("No se pudo crear el pool de conexiones SQLite");

    {
        let conn = pool.get().expect("No se pudo abrir DB");
        init_db(&conn).expect("No se pudo inicializar el esquema de DB");
    }

    let worker_pool = pool.clone();

    tauri::Builder::default()
        .manage(DbState(pool))
        .manage(SesionActual(Mutex::new(None)))
        .setup(move |_| {
            start_sync_worker(worker_pool.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_sesion_actual,
            cerrar_sesion_local,
            update_mi_perfil,
            necesita_configuracion_inicial,
            crear_configuracion_inicial,
            iniciar_sesion,
            get_usuarios,
            create_usuario,
            update_usuario,
            delete_usuario,
            get_sucursales,
            get_proveedores,
            create_proveedor,
            update_proveedor,
            delete_provider,
            get_marcas,
            create_marca,
            update_marca,
            delete_marca,
            get_categorias,
            create_categoria,
            update_categoria,
            delete_categoria,
            get_unidades,
            create_unidad,
            update_unidad,
            delete_unidad,
            get_clientes,
            create_cliente,
            update_cliente,
            delete_cliente,
            get_cliente_datos_fiscales,
            guardar_cliente_datos_fiscales,
            registrar_abono,
            create_sucursal,
            update_sucursal,
            delete_sucursal,
            get_productos_por_sucursal,
            get_productos_por_sucursal_page,
            buscar_productos_por_sucursal,
            buscar_productos_para_compra,
            asegurar_producto_venta_diversa,
            get_productos_catalogo,
            get_productos_catalogo_page,
            importar_datos_universal_visual,
            importar_articulos_legacy_visual,
            seleccionar_archivo_csv_importacion,
            analizar_csv_importacion,
            importar_csv_productos_mapeado,
            create_producto_catalogo,
            update_producto_catalogo,
            guardar_inventario_sucursal,
            eliminar_inventario_sucursal,
            get_promociones,
            get_productos_para_promociones,
            guardar_promocion,
            eliminar_promocion,
            create_producto,
            update_producto,
            delete_producto,
            registrar_compra,
            get_caja_actual,
            abrir_caja,
            registrar_movimiento_caja,
            cerrar_caja,
            get_system_printers,
            get_perifericos_config,
            guardar_perifericos_config,
            imprimir_silencioso,
            imprimir_ticket_y_abrir_caja,
            get_dashboard_stats,
            get_rentabilidad,
            get_indicador_ventas,
            get_indicador_inventario,
            get_indicador_financiero,
            get_productos_bajo_stock,
            get_productos_bajo_stock_page,
            get_productos_mas_vendidos,
            get_historial_ventas,
            get_historial_ventas_page,
            get_detalle_venta,
            get_empresa_config,
            guardar_empresa_config,
            get_supabase_config,
            test_and_save_supabase_connect,
            disconnect_supabase,
            crear_respaldo_local,
            aplicar_respaldo_local,
            sincronizar_hacia_nube,
            subir_base_local_completa_a_nube,
            sincronizar_desde_nube,
            get_sync_status,
            get_notificaciones,
            marcar_notificacion_leida,
            marcar_todas_notificaciones_leidas,
            get_sync_migration_status,
            get_payload_factura,
            actualizar_estado_factura,
            get_facturas_emitidas,
            get_facturas_emitidas_page,
            get_facturas_por_ventas,
            cancelar_venta,
            registrar_traspaso,
            recibir_traspaso,
            get_historial_traspasos,
            get_historial_traspasos_page,
            registrar_merma_ajuste,
            get_historial_mermas,
            get_historial_mermas_page,
            registrar_venta
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}


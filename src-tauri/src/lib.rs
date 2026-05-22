use bcrypt::{hash, verify, DEFAULT_COST};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::types::{Value, ValueRef};
use rusqlite::{params, params_from_iter, Connection, Error as SqliteError, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;
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
];

const SYNC_TABLES: &[&str] = &[
    "sucursales",
    "empresa_config_fiscal",
    "usuarios",
    "proveedores",
    "marcas",
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
    "unidades",
    "productos",
    "clientes",
];

const AUTO_SYNC_BATCH_SIZE: usize = 50;
const DELTA_PULL_TABLES: &[&str] = &[
    "sucursales",
    "proveedores",
    "marcas",
    "unidades",
    "usuarios",
    "productos",
    "inventario_sucursal",
    "clientes",
    "clientes_datos_fiscales",
];

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
pub struct DashboardFiltroInput {
    sucursal_id: Option<String>,
    fecha_inicio: Option<String>,
    fecha_fin: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    total_vendido: f64,
    utilidad_neta: f64,
    transacciones: i64,
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
pub struct SyncStatus {
    pendientes: i64,
    ventas_pendientes: i64,
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
            cantidad, costo_unitario, usuario_id, fecha
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
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
    producto.codigo_barras = producto.codigo_barras.trim().to_string();
    producto.codigo_proveedor = normalize_upper_trim(&producto.codigo_proveedor);
    producto.clave_producto = normalize_upper_trim(&producto.clave_producto);
    producto.sat_clave_prod_serv = normalize_upper_trim(&producto.sat_clave_prod_serv);
    producto.sat_clave_unidad = normalize_upper_trim(&producto.sat_clave_unidad);
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
        if detalle.cantidad <= 0.0 {
            return Err(AppError::Validation("La cantidad debe ser mayor que cero.".to_string()));
        }
        if detalle.precio_costo_pactado < 0.0 {
            return Err(AppError::Validation("El precio costo pactado no puede ser negativo.".to_string()));
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
            });
        }
    }

    Ok(consolidados)
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
    if input.monto <= 0.0 {
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
        if detalle.cantidad <= 0.0 {
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
    if input.cantidad <= 0.0 {
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
    if input.monto_inicial < 0.0 {
        return Err(AppError::Validation("El fondo inicial no puede ser negativo.".to_string()));
    }
    Ok(())
}

fn validate_movimiento_caja_input(input: &MovimientoCajaInput) -> Result<(), AppError> {
    if input.id.trim().is_empty() || input.sesion_id.trim().is_empty() {
        return Err(AppError::Validation("Datos incompletos para el movimiento de caja.".to_string()));
    }
    if input.tipo != "INGRESO" && input.tipo != "EGRESO" {
        return Err(AppError::Validation("El tipo de movimiento debe ser INGRESO o EGRESO.".to_string()));
    }
    if input.monto <= 0.0 {
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
    if input.monto_final_real < 0.0 {
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

    if !is_superadmin(actor) {
        let target_user = target.ok_or_else(|| "No se encontró el usuario indicado.".to_string())?;
        if target_user.sucursal_id != actor.sucursal_id {
            return Err("Operación inválida: Solo puedes administrar usuarios de tu sucursal.".to_string());
        }
        if target_user.role != "USUARIO" {
            return Err("Operación inválida: Un administrador solo puede modificar usuarios operativos.".to_string());
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
            cliente_id TEXT NULL,
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
    migrate_ventas_add_cancelacion_auditoria(conn)?;
    migrate_facturacion_cfdi40(conn)?;
    migrate_supabase_config(conn)?;
    migrate_notificaciones(conn)?;
    migrate_marcas_unidades(conn)?;
    migrate_inventario_empresarial(conn)?;
    migrate_add_sincronizacion_fields(conn)?;
    Ok(())
}

fn migrate_marcas_unidades(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS marcas (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE
        );
        CREATE TABLE IF NOT EXISTS unidades (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL UNIQUE,
            clave_sat TEXT NOT NULL DEFAULT ''
        );
        CREATE INDEX IF NOT EXISTS idx_marcas_nombre ON marcas(nombre);
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

    create_sync_dirty_triggers(conn)?;

    Ok(())
}

fn sync_dirty_predicate(table: &str) -> Option<&'static str> {
    match table {
        "inventario_sucursal" => Some("producto_id = NEW.producto_id AND sucursal_id = NEW.sucursal_id"),
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
fn get_proveedores(state_db: tauri::State<DbState>) -> AppResult<Vec<Proveedor>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, nombre, contacto_nombre, telefono, email, direccion FROM proveedores WHERE eliminado = 0 ORDER BY nombre")
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
fn create_proveedor(state_db: tauri::State<DbState>, proveedor: Proveedor) -> AppResult<()> {
    validate_proveedor(&proveedor).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.execute(
        "INSERT INTO proveedores (id, nombre, contacto_nombre, telefono, email, direccion) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
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
fn update_proveedor(state_db: tauri::State<DbState>, id: String, proveedor: Proveedor) -> AppResult<()> {
    validate_proveedor(&proveedor).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let affected = conn
        .execute(
            "UPDATE proveedores SET nombre = ?1, contacto_nombre = ?2, telefono = ?3, email = ?4, direccion = ?5 WHERE id = ?6 AND eliminado = 0",
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
fn delete_provider(state_db: tauri::State<DbState>, id: String) -> AppResult<()> {
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
fn get_marcas(state_db: tauri::State<DbState>) -> AppResult<Vec<Marca>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, nombre FROM marcas WHERE eliminado = 0 ORDER BY nombre")
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
fn create_marca(state_db: tauri::State<DbState>, marca: Marca) -> AppResult<()> {
    validate_marca(&marca).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.execute(
        "INSERT INTO marcas (id, nombre) VALUES (?1, ?2)",
        params![marca.id, marca.nombre.trim()],
    )
    .map_err(|error| map_write_error(error, "marca"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn update_marca(state_db: tauri::State<DbState>, id: String, marca: Marca) -> AppResult<()> {
    validate_marca(&marca).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let affected = conn
        .execute(
            "UPDATE marcas SET nombre = ?1 WHERE id = ?2 AND eliminado = 0",
            params![marca.nombre.trim(), id],
        )
        .map_err(|error| map_write_error(error, "marca"))
        .map_err(to_command_error)?;
    if affected == 0 {
        return Err("No se encontró la marca que intentas actualizar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn delete_marca(state_db: tauri::State<DbState>, id: String) -> AppResult<()> {
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
fn get_unidades(state_db: tauri::State<DbState>) -> AppResult<Vec<UnidadMedida>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, nombre, clave_sat FROM unidades WHERE eliminado = 0 ORDER BY nombre")
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
fn create_unidad(state_db: tauri::State<DbState>, unidad: UnidadMedida) -> AppResult<()> {
    validate_unidad(&unidad).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.execute(
        "INSERT INTO unidades (id, nombre, clave_sat) VALUES (?1, ?2, ?3)",
        params![unidad.id, unidad.nombre.trim(), normalize_upper_trim(&unidad.clave_sat)],
    )
    .map_err(|error| map_write_error(error, "unidad"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn update_unidad(state_db: tauri::State<DbState>, id: String, unidad: UnidadMedida) -> AppResult<()> {
    validate_unidad(&unidad).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let affected = conn
        .execute(
            "UPDATE unidades SET nombre = ?1, clave_sat = ?2 WHERE id = ?3 AND eliminado = 0",
            params![unidad.nombre.trim(), normalize_upper_trim(&unidad.clave_sat), id],
        )
        .map_err(|error| map_write_error(error, "unidad"))
        .map_err(to_command_error)?;
    if affected == 0 {
        return Err("No se encontró la unidad que intentas actualizar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn delete_unidad(state_db: tauri::State<DbState>, id: String) -> AppResult<()> {
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
fn get_clientes(state_db: tauri::State<DbState>) -> AppResult<Vec<Cliente>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, nombre, telefono, direccion, limite_credito, saldo_deudor FROM clientes WHERE eliminado = 0 ORDER BY nombre")
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
fn create_cliente(state_db: tauri::State<DbState>, cliente: Cliente) -> AppResult<()> {
    validate_cliente(&cliente).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.execute(
        "INSERT INTO clientes (id, nombre, telefono, direccion, limite_credito, saldo_deudor) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            cliente.id,
            cliente.nombre,
            cliente.telefono,
            cliente.direccion,
            cliente.limite_credito,
            cliente.saldo_deudor
        ],
    )
    .map_err(|error| map_write_error(error, "cliente"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn update_cliente(state_db: tauri::State<DbState>, id: String, cliente: Cliente) -> AppResult<()> {
    validate_cliente(&cliente).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let affected = conn
        .execute(
            "UPDATE clientes SET nombre = ?1, telefono = ?2, direccion = ?3, limite_credito = ?4 WHERE id = ?5 AND eliminado = 0",
            params![
                cliente.nombre,
                cliente.telefono,
                cliente.direccion,
                cliente.limite_credito,
                id
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
fn delete_cliente(state_db: tauri::State<DbState>, id: String) -> AppResult<()> {
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
    cliente_id: String,
) -> AppResult<Option<ClienteDatosFiscales>> {
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
    datos: ClienteDatosFiscales,
) -> AppResult<ClienteDatosFiscales> {
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
            cliente_id, rfc, razon_social, regimen_fiscal, codigo_postal
        ) VALUES (?1, ?2, ?3, ?4, ?5)
        ON CONFLICT(cliente_id) DO UPDATE SET
            rfc = excluded.rfc,
            razon_social = excluded.razon_social,
            regimen_fiscal = excluded.regimen_fiscal,
            codigo_postal = excluded.codigo_postal
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
fn registrar_abono(state_db: tauri::State<DbState>, abono: AbonoCreditoInput) -> AppResult<()> {
    validate_abono_credito(&abono).map_err(to_command_error)?;
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let saldo_actual: f64 = tx
        .query_row(
            "SELECT saldo_deudor FROM clientes WHERE id = ?1 AND eliminado = 0",
            [&abono.cliente_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if abono.monto > saldo_actual {
        return Err("El abono no puede ser mayor al saldo deudor actual.".to_string());
    }

    tx.execute(
        "INSERT INTO creditos_abonos (id, cliente_id, monto, fecha, usuario_id) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![abono.id, abono.cliente_id, abono.monto, abono.fecha, abono.usuario_id],
    )
    .map_err(|error| map_write_error(error, "abono"))
    .map_err(to_command_error)?;

    tx.execute(
        "UPDATE clientes SET saldo_deudor = saldo_deudor - ?1 WHERE id = ?2",
        params![abono.monto, abono.cliente_id],
    )
    .map_err(|error| map_write_error(error, "cliente"))
    .map_err(to_command_error)?;

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn get_productos_por_sucursal(
    state_db: tauri::State<DbState>,
    sucursal_id: String,
) -> AppResult<Vec<ProductoConStock>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare(
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
                i.stock_minimo
            FROM productos p
            INNER JOIN inventario_sucursal i ON i.producto_id = p.id
            WHERE i.sucursal_id = ?1
              AND p.eliminado = 0
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
    sucursal_id: String,
    query: String,
) -> AppResult<Vec<ProductoConStock>> {
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
                i.stock_minimo
            FROM productos p
            INNER JOIN inventario_sucursal i ON i.producto_id = p.id
            WHERE i.sucursal_id = ?1
              AND p.eliminado = 0
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
fn get_productos_catalogo(state_db: tauri::State<DbState>) -> AppResult<Vec<Producto>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare(
            "
            SELECT id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
                   marca, categoria, unidad, precio_costo, costo_promedio, precio_venta,
                   sat_clave_prod_serv, sat_clave_unidad
            FROM productos
            WHERE eliminado = 0
            ORDER BY descripcion
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
fn create_producto_catalogo(state_db: tauri::State<DbState>, producto: Producto) -> AppResult<()> {
    let mut producto = sanitize_producto(producto);
    producto.precio_costo = 0.0;
    producto.costo_promedio = 0.0;
    producto.precio_venta = 0.0;
    validate_producto(&producto).map_err(to_command_error)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let proveedor_id = resolve_valid_producto_proveedor_id(&conn, &producto.proveedor_id).map_err(to_command_error)?;
    conn.execute(
        "
        INSERT INTO productos (
            id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
            marca, categoria, unidad, precio_costo, costo_promedio, precio_venta, sat_clave_prod_serv, sat_clave_unidad
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, 0, 0, ?10, ?11)
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
            producto.marca,
            producto.categoria,
            producto.unidad,
            producto.sat_clave_prod_serv,
            producto.sat_clave_unidad
        ],
    )
    .map_err(|error| map_write_error(error, "producto"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn update_producto_catalogo(state_db: tauri::State<DbState>, producto_id: String, producto: Producto) -> AppResult<()> {
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
                sat_clave_unidad = ?10
            WHERE id = ?11 AND eliminado = 0
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
                producto.marca,
                producto.categoria,
                producto.unidad,
                producto.sat_clave_prod_serv,
                producto.sat_clave_unidad,
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
    producto_id: String,
    inventario: InventarioSucursalInput,
) -> AppResult<()> {
    if producto_id.trim().is_empty() {
        return Err("Selecciona un producto válido.".to_string());
    }
    validate_inventario_input(&inventario).map_err(to_command_error)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let producto_exists: i64 = conn
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

    let sucursal_exists: i64 = conn
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

    conn.execute(
        "
        INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
          stock = excluded.stock,
          stock_minimo = excluded.stock_minimo,
          costo_promedio = excluded.costo_promedio,
          precio_venta = excluded.precio_venta
        ",
        params![
            producto_id,
            inventario.sucursal_id,
            inventario.stock,
            inventario.stock_minimo,
            inventario.costo_promedio,
            inventario.precio_venta
        ],
    )
    .map_err(|error| map_write_error(error, "inventario"))
    .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn create_producto(
    state_db: tauri::State<DbState>,
    producto: Producto,
    inventario: InventarioSucursalInput,
) -> AppResult<()> {
    let producto = sanitize_producto(producto);
    validate_producto(&producto).map_err(to_command_error)?;
    validate_inventario_input(&inventario).map_err(to_command_error)?;

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let proveedor_id = resolve_valid_producto_proveedor_id(&conn, &producto.proveedor_id).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    tx.execute(
        "
        INSERT INTO productos (
            id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
            marca, categoria, unidad, precio_costo, costo_promedio, precio_venta, sat_clave_prod_serv, sat_clave_unidad
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
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
            producto.marca,
            producto.categoria,
            producto.unidad,
            producto.precio_costo,
            if producto.costo_promedio > 0.0 { producto.costo_promedio } else { producto.precio_costo },
            producto.precio_venta,
            producto.sat_clave_prod_serv,
            producto.sat_clave_unidad
        ],
    )
    .map_err(|error| map_write_error(error, "producto"))
    .map_err(to_command_error)?;

    tx.execute(
        "
        INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
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
    producto_id: String,
    producto: Producto,
    inventario: InventarioSucursalInput,
) -> AppResult<()> {
    let producto = sanitize_producto(producto);
    validate_producto(&producto).map_err(to_command_error)?;
    validate_inventario_input(&inventario).map_err(to_command_error)?;

    if producto_id.trim().is_empty() {
        return Err("Falta el identificador del producto a actualizar.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let proveedor_id = resolve_valid_producto_proveedor_id(&conn, &producto.proveedor_id).map_err(to_command_error)?;
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
            sat_clave_unidad = ?13
        WHERE id = ?14 AND eliminado = 0
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
            producto.marca,
            producto.categoria,
            producto.unidad,
            producto.precio_costo,
            producto.costo_promedio,
            producto.precio_venta,
            producto.sat_clave_prod_serv,
            producto.sat_clave_unidad,
            producto_id
        ],
    )
    .map_err(|error| map_write_error(error, "producto"))
    .map_err(to_command_error)?;

    conn.execute(
        "
        INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
          stock = excluded.stock,
          stock_minimo = excluded.stock_minimo,
          costo_promedio = CASE WHEN excluded.costo_promedio > 0 THEN excluded.costo_promedio ELSE inventario_sucursal.costo_promedio END,
          precio_venta = CASE WHEN excluded.precio_venta > 0 THEN excluded.precio_venta ELSE inventario_sucursal.precio_venta END
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
fn delete_producto(state_db: tauri::State<DbState>, id: String) -> AppResult<()> {
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
    compra: RegistrarCompraInput,
) -> AppResult<()> {
    validate_registrar_compra_input(&compra).map_err(to_command_error)?;

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

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
    for detalle in &compra.detalles {
        total += detalle.cantidad * detalle.precio_costo_pactado;
    }

    tx.execute(
        "INSERT INTO compras (id, proveedor_id, sucursal_id, fecha, total) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![compra.id, compra.proveedor_id, compra.sucursal_id, compra.fecha, total],
    )
    .map_err(|error| map_write_error(error, "compra"))
    .map_err(to_command_error)?;

    for detalle in &compra.detalles {
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
        let nuevo_costo_promedio = if nuevo_stock > 0.0 {
            ((stock_actual * costo_actual) + (detalle.cantidad * detalle.precio_costo_pactado)) / nuevo_stock
        } else {
            detalle.precio_costo_pactado
        };

        tx.execute(
            "INSERT INTO detalle_compras (id, compra_id, producto_id, cantidad, precio_costo_pactado, costo_promedio_resultante) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                detalle.id,
                compra.id,
                detalle.producto_id,
                detalle.cantidad,
                detalle.precio_costo_pactado,
                nuevo_costo_promedio
            ],
        )
        .map_err(|error| map_write_error(error, "detalle de compra"))
        .map_err(to_command_error)?;

        tx.execute(
            "
            INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta)
            VALUES (?1, ?2, ?3, 0, ?4, COALESCE((SELECT precio_venta FROM productos WHERE id = ?1), 0))
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
              stock = inventario_sucursal.stock + excluded.stock,
              costo_promedio = excluded.costo_promedio
            ",
            params![detalle.producto_id, compra.sucursal_id, detalle.cantidad, nuevo_costo_promedio],
        )
        .map_err(|error| map_write_error(error, "inventario"))
        .map_err(to_command_error)?;

        tx.execute(
            "
            UPDATE productos
            SET precio_costo = ?1,
                costo_promedio = ?2
            WHERE id = ?3
            ",
            params![detalle.precio_costo_pactado, nuevo_costo_promedio, detalle.producto_id],
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
            Some(detalle.precio_costo_pactado),
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
    usuario_id: String,
    sucursal_id: String,
) -> AppResult<Option<CajaEstado>> {
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
    apertura: AbrirCajaInput,
) -> AppResult<CajaEstado> {
    validate_abrir_caja_input(&apertura).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;

    let abierta_actual: i64 = conn
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

    conn.execute(
        "
        INSERT INTO cajas_sesiones (
            id, usuario_id, sucursal_id, fecha_apertura, monto_inicial, fecha_cierre, monto_final_real, monto_esperado, estado
        ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?5, 'ABIERTA')
        ",
        params![
            apertura.id,
            apertura.usuario_id,
            apertura.sucursal_id,
            apertura.fecha_apertura,
            apertura.monto_inicial
        ],
    )
    .map_err(|error| map_write_error(error, "sesión de caja"))
    .map_err(to_command_error)?;

    let sesion = CajaSesion {
        id: apertura.id,
        usuario_id: apertura.usuario_id,
        sucursal_id: apertura.sucursal_id,
        fecha_apertura: apertura.fecha_apertura,
        monto_inicial: apertura.monto_inicial,
        fecha_cierre: None,
        monto_final_real: None,
        monto_esperado: apertura.monto_inicial,
        estado: "ABIERTA".to_string(),
    };

    calcular_resumen_caja(&conn, &sesion).map_err(to_command_error)
}

#[tauri::command]
fn registrar_movimiento_caja(
    state_db: tauri::State<DbState>,
    movimiento: MovimientoCajaInput,
) -> AppResult<CajaEstado> {
    validate_movimiento_caja_input(&movimiento).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;

    let sesion = conn
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

    conn.execute(
        "INSERT INTO caja_movimientos (id, sesion_id, tipo, monto, motivo) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            movimiento.id,
            movimiento.sesion_id,
            movimiento.tipo,
            movimiento.monto,
            movimiento.motivo
        ],
    )
    .map_err(|error| map_write_error(error, "movimiento de caja"))
    .map_err(to_command_error)?;

    let resumen = calcular_resumen_caja(&conn, &sesion).map_err(to_command_error)?;
    conn.execute(
        "UPDATE cajas_sesiones SET monto_esperado = ?1 WHERE id = ?2",
        params![resumen.monto_esperado_actual, sesion.id],
    )
    .map_err(AppError::from)
    .map_err(to_command_error)?;

    Ok(resumen)
}

#[tauri::command]
fn cerrar_caja(
    state_db: tauri::State<DbState>,
    cierre: CerrarCajaInput,
) -> AppResult<CajaEstado> {
    validate_cerrar_caja_input(&cierre).map_err(to_command_error)?;
    let conn = get_conn(&state_db).map_err(to_command_error)?;

    let sesion = conn
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

    let resumen = calcular_resumen_caja(&conn, &sesion).map_err(to_command_error)?;
    conn.execute(
        "
        UPDATE cajas_sesiones
        SET fecha_cierre = ?1,
            monto_final_real = ?2,
            monto_esperado = ?3,
            estado = 'CERRADA'
        WHERE id = ?4
        ",
        params![
            cierre.fecha_cierre,
            cierre.monto_final_real,
            resumen.monto_esperado_actual,
            cierre.sesion_id
        ],
    )
    .map_err(|error| map_write_error(error, "cierre de caja"))
    .map_err(to_command_error)?;

    Ok(CajaEstado {
        sesion: CajaSesion {
            fecha_cierre: Some(cierre.fecha_cierre),
            monto_final_real: Some(cierre.monto_final_real),
            monto_esperado: resumen.monto_esperado_actual,
            estado: "CERRADA".to_string(),
            ..sesion
        },
        ..resumen
    })
}

#[tauri::command]
fn get_dashboard_stats(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    filtro: Option<DashboardFiltroInput>,
) -> AppResult<DashboardStats> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
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

    Ok(DashboardStats {
        total_vendido,
        utilidad_neta,
        transacciones,
    })
}

#[tauri::command]
fn get_productos_bajo_stock(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    sucursal_id: Option<String>,
) -> AppResult<Vec<ProductoBajoStock>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
    let sid = scoped_sucursal_for_read(&user, normalize_filter(&sucursal_id));

    let mut resultados = Vec::new();
    if let Some(sucursal) = sid {
        let mut stmt = conn
            .prepare(
                "
                SELECT p.id, p.descripcion, p.marca, s.id, s.nombre, i.stock, i.stock_minimo
                FROM inventario_sucursal i
                INNER JOIN productos p ON p.id = i.producto_id
                INNER JOIN sucursales s ON s.id = i.sucursal_id
                WHERE i.stock <= i.stock_minimo AND i.sucursal_id = ?1
                ORDER BY (i.stock - i.stock_minimo) ASC, p.descripcion
                ",
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        let iter = stmt
            .query_map([sucursal], |row| {
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
        for item in iter {
            resultados.push(item.map_err(AppError::from).map_err(to_command_error)?);
        }
    } else {
        let mut stmt = conn
            .prepare(
                "
                SELECT p.id, p.descripcion, p.marca, s.id, s.nombre, i.stock, i.stock_minimo
                FROM inventario_sucursal i
                INNER JOIN productos p ON p.id = i.producto_id
                INNER JOIN sucursales s ON s.id = i.sucursal_id
                WHERE i.stock <= i.stock_minimo
                ORDER BY (i.stock - i.stock_minimo) ASC, p.descripcion
                ",
            )
            .map_err(AppError::from)
            .map_err(to_command_error)?;
        let iter = stmt
            .query_map([], |row| {
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
        for item in iter {
            resultados.push(item.map_err(AppError::from).map_err(to_command_error)?);
        }
    }

    Ok(resultados)
}

#[tauri::command]
fn get_productos_mas_vendidos(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
    filtro: Option<DashboardFiltroInput>,
) -> AppResult<Vec<ProductoMasVendido>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
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

    let mut sql = String::from(
        "
        SELECT
          v.id, v.fecha, v.total, v.metodo_pago, v.estado,
          s.id, s.nombre, u.id, u.nombre, c.id, c.nombre
        FROM ventas v
        INNER JOIN sucursales s ON s.id = v.sucursal_id
        INNER JOIN usuarios u ON u.id = v.usuario_id
        LEFT JOIN clientes c ON c.id = v.cliente_id
        WHERE v.fecha >= ?1 AND v.fecha <= ?2
        ",
    );
    let mut params_vec: Vec<String> = vec![fi, ff];

    if let Some(value) = sid {
        sql.push_str(" AND v.sucursal_id = ?3");
        params_vec.push(value);
    }
    if let Some(value) = uid {
        if params_vec.len() == 2 {
            sql.push_str(" AND v.usuario_id = ?3");
        } else {
            sql.push_str(" AND v.usuario_id = ?4");
        }
        params_vec.push(value);
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
            estado: row.get(4).map_err(AppError::from).map_err(to_command_error)?,
            sucursal_id: row.get(5).map_err(AppError::from).map_err(to_command_error)?,
            sucursal_nombre: row.get(6).map_err(AppError::from).map_err(to_command_error)?,
            usuario_id: row.get(7).map_err(AppError::from).map_err(to_command_error)?,
            usuario_nombre: row.get(8).map_err(AppError::from).map_err(to_command_error)?,
            cliente_id: row.get(9).ok(),
            cliente_nombre: row.get(10).ok(),
        });
    }

    Ok(historial)
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
fn get_empresa_config(state_db: tauri::State<DbState>) -> AppResult<Option<EmpresaConfigFiscal>> {
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
    config: EmpresaConfigFiscal,
) -> AppResult<EmpresaConfigFiscal> {
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
    let user = current_session_user(&state_sesion)?;
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
            let valor_unitario: f64 = row.get(7)?;
            let importe = round_money(cantidad * valor_unitario);
            let iva = round_money(importe * 0.16);

            Ok(CfdiConcepto {
                producto_id,
                clave_prod_serv,
                no_identificacion,
                cantidad,
                clave_unidad,
                unidad,
                descripcion,
                valor_unitario: round_money(valor_unitario),
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
        if concepto.clave_prod_serv.trim().is_empty() || concepto.clave_unidad.trim().is_empty() {
            return Err(format!(
                "El producto '{}' no tiene claves SAT completas.",
                concepto.descripcion
            ));
        }
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
            id, venta_id, uuid, rfc_receptor, monto_total, estado, fecha_emision, pdf_path, xml_path
        ) VALUES (?1, ?2, NULL, ?3, ?4, 'PENDIENTE', ?5, NULL, NULL)
        ",
        params![
            format!("FAC-{}", venta_id),
            venta_id,
            payload.receptor.rfc,
            payload.total,
            venta_fecha
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
    if input.factura_id.trim().is_empty() || input.uuid.trim().is_empty() {
        return Err("La factura y el UUID oficial son obligatorios.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
    let sucursal_id: String = conn
        .query_row(
            "
            SELECT v.sucursal_id
            FROM facturas_emitidas fe
            INNER JOIN ventas v ON v.id = fe.venta_id
            WHERE fe.id = ?1
            ",
            [&input.factura_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    ensure_can_read_sucursal(&user, &sucursal_id)?;

    let affected = conn
        .execute(
            "
            UPDATE facturas_emitidas
            SET estado = 'TIMBRADA',
                uuid = ?2,
                pdf_path = ?3,
                xml_path = ?4
            WHERE id = ?1
            ",
            params![input.factura_id, input.uuid, input.pdf_path, input.xml_path],
        )
        .map_err(|error| map_write_error(error, "factura"))
        .map_err(to_command_error)?;

    if affected == 0 {
        return Err("No se encontró la factura a actualizar.".to_string());
    }

    Ok(())
}

#[tauri::command]
fn get_facturas_emitidas(
    state_db: tauri::State<DbState>,
    state_sesion: tauri::State<SesionActual>,
) -> AppResult<Vec<FacturaEmitida>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let user = current_session_user(&state_sesion)?;
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
fn get_sync_migration_status(state_db: tauri::State<DbState>) -> AppResult<SyncMigrationStatus> {
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
fn get_sync_status(state_db: tauri::State<DbState>) -> AppResult<SyncStatus> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let pendientes = count_sync_pending(&conn).map_err(to_command_error)?;
    let ventas_pendientes = count_table_sync_pending(&conn, "ventas").map_err(to_command_error)?;
    Ok(SyncStatus {
        pendientes,
        ventas_pendientes,
    })
}

#[tauri::command]
fn get_notificaciones(state_db: tauri::State<DbState>, solo_no_leidas: Option<bool>) -> AppResult<Vec<Notificacion>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    evaluar_notificaciones_negocio(&conn).map_err(to_command_error)?;

    let only_unread = solo_no_leidas.unwrap_or(false);
    let sql = if only_unread {
        "
        SELECT id, categoria, severidad, titulo, mensaje, entidad_tipo, entidad_id, event_key, leida, creada_at
        FROM notificaciones
        WHERE leida = 0
        ORDER BY leida ASC, creada_at DESC
        LIMIT 50
        "
    } else {
        "
        SELECT id, categoria, severidad, titulo, mensaje, entidad_tipo, entidad_id, event_key, leida, creada_at
        FROM notificaciones
        ORDER BY leida ASC, creada_at DESC
        LIMIT 50
        "
    };

    let mut stmt = conn.prepare(sql).map_err(AppError::from).map_err(to_command_error)?;
    let rows = stmt
        .query_map([], |row| {
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
fn marcar_notificacion_leida(state_db: tauri::State<DbState>, id: String) -> AppResult<()> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.execute("UPDATE notificaciones SET leida = 1 WHERE id = ?1", [id])
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn marcar_todas_notificaciones_leidas(state_db: tauri::State<DbState>) -> AppResult<()> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    conn.execute("UPDATE notificaciones SET leida = 1 WHERE leida = 0", [])
        .map_err(AppError::from)
        .map_err(to_command_error)?;
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
        INSERT OR IGNORE INTO notificaciones (
            id, categoria, severidad, titulo, mensaje, entidad_tipo, entidad_id, event_key, leida, creada_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, datetime('now'))
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
        SELECT id, venta_id, rfc_receptor, monto_total, fecha_emision
        FROM facturas_emitidas
        WHERE estado = 'PENDIENTE'
          AND datetime(fecha_emision) <= datetime('now', '-1 day')
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

    Ok(BackupLocal {
        version: "1".to_string(),
        generado_at: current_timestamp_string(),
        tablas,
    })
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
    let is_remote_backup = backup.version == "supabase-rest-v1";
    let mut columns_by_table: HashMap<String, Vec<String>> = HashMap::new();
    for table in BACKUP_TABLES {
        columns_by_table.insert((*table).to_string(), table_columns(conn, table)?);
    }

    let tx = conn.transaction()?;
    tx.execute("PRAGMA foreign_keys = OFF", [])?;

    for table in BACKUP_TABLES.iter().rev() {
        tx.execute(&format!("DELETE FROM {table}"), [])?;
    }

    for table in BACKUP_TABLES {
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

    tx.execute("PRAGMA foreign_keys = ON", [])?;
    tx.commit()?;
    Ok(())
}

fn query_json_rows(conn: &Connection, sql: &str) -> Result<Vec<JsonValue>, AppError> {
    let mut stmt = conn.prepare(sql)?;
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

fn mark_table_synced(conn: &Connection, table: &str, synced_at: &str) -> Result<(), AppError> {
    conn.execute(
        &format!("UPDATE {table} SET sincronizado = 1, updated_at = ?1 WHERE sincronizado = 0"),
        [synced_at],
    )?;
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
        "clientes_datos_fiscales" => &["cliente_id"],
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
                let remote_updated_at = object
                    .get("updated_at")
                    .and_then(JsonValue::as_str)
                    .unwrap_or("1970-01-01")
                    .to_string();
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

    if matches!(table, "inventario_sucursal" | "usuarios") && !sucursal_ids.is_empty() {
        endpoint.push_str("&sucursal_id=");
        endpoint.push_str(&url_encode_component(&format!("in.({})", sucursal_ids.join(","))));
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

fn run_auto_pull_once(pool: &DbPool) -> AppResult<usize> {
    let mut conn = pool.get().map_err(AppError::from).map_err(to_command_error)?;
    let config = get_supabase_config_from_conn(&conn)
        .map_err(to_command_error)?
        .ok_or_else(|| "Supabase no está configurado.".to_string())?;

    if !config.is_connected || config.url.trim().is_empty() || config.anon_key.trim().is_empty() {
        return Ok(0);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("No se pudo preparar la conexión HTTP: {error}"))?;
    let sucursal_ids = get_local_sucursal_ids(&conn).map_err(to_command_error)?;
    let mut total = 0;

    for table in DELTA_PULL_TABLES {
        total += pull_delta_table(&mut conn, &client, &config, table, &sucursal_ids)?;
    }

    Ok(total)
}

fn upload_pending_table(
    conn: &Connection,
    client: &reqwest::blocking::Client,
    config: &SupabaseConfig,
    table: &str,
    conflict_target: &str,
    sql: &str,
) -> AppResult<usize> {
    let rows = query_json_rows(conn, sql).map_err(to_command_error)?;
    if rows.is_empty() {
        return Ok(0);
    }

    let endpoint = format!(
        "{}/rest/v1/{}?on_conflict={}",
        config.url.trim_end_matches('/'),
        table,
        conflict_target
    );
    let response = supabase_upsert_builder(client, &endpoint, &config.anon_key)
        .json(&rows)
        .send()
        .map_err(|error| format!("No se pudo subir la tabla {table}: {error}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!(
            "Supabase rechazó la subida de {table}. Código HTTP: {status}. {body}"
        ));
    }

    let synced_at = current_sqlite_timestamp(conn).map_err(to_command_error)?;
    mark_table_synced(conn, table, &synced_at).map_err(to_command_error)?;
    Ok(rows.len())
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

    let synced_at = current_sqlite_timestamp(conn).map_err(to_command_error)?;
    mark_table_ids_synced(conn, table, &ids, &synced_at).map_err(to_command_error)?;
    Ok(rows.len())
}

fn run_auto_sync_once(pool: &DbPool) -> AppResult<usize> {
    let conn = pool.get().map_err(AppError::from).map_err(to_command_error)?;
    ensure_sync_uuids(&conn).map_err(to_command_error)?;
    evaluar_notificaciones_negocio(&conn).map_err(to_command_error)?;
    let pendientes_antes = count_sync_pending(&conn).map_err(to_command_error)?;
    let ventas_pendientes_antes = count_table_sync_pending(&conn, "ventas").map_err(to_command_error)?;
    let config = get_supabase_config_from_conn(&conn)
        .map_err(to_command_error)?
        .ok_or_else(|| "Supabase no está configurado.".to_string())?;

    if !config.is_connected || config.url.trim().is_empty() || config.anon_key.trim().is_empty() {
        return Ok(0);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("No se pudo preparar la conexión HTTP: {error}"))?;
    let limit = AUTO_SYNC_BATCH_SIZE;
    let plans = [
        (
            "sucursales",
            "id",
            format!(
                "
                SELECT id AS __local_id, id, nombre, direccion, telefono, codigo_postal,
                       eliminado, 1 AS sincronizado, updated_at
                FROM sucursales
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                       sat_clave_unidad, eliminado, 1 AS sincronizado, updated_at
                FROM productos
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE dc.sincronizado = 0
                ORDER BY dc.updated_at
                LIMIT {limit}
                "
            ),
        ),
        (
            "ventas",
            "uuid",
            format!(
                "
                SELECT id AS __local_id, sync_uuid AS uuid, id, usuario_id, sucursal_id, fecha, total,
                       metodo_pago, cliente_id, usuario_autorizo_cancelacion_id, motivo_cancelacion,
                       fecha_cancelacion, estado, 1 AS sincronizado, updated_at
                FROM ventas
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                       dv.costo_unitario_pactado, 1 AS sincronizado, dv.updated_at
                FROM detalle_ventas dv
                INNER JOIN ventas v ON v.id = dv.venta_id
                WHERE dv.sincronizado = 0
                ORDER BY dv.updated_at
                LIMIT {limit}
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
                WHERE ca.sincronizado = 0
                ORDER BY ca.updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE cm.sincronizado = 0
                ORDER BY cm.updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE dt.sincronizado = 0
                ORDER BY dt.updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
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
                WHERE fe.sincronizado = 0
                ORDER BY fe.updated_at
                LIMIT {limit}
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
                WHERE sincronizado = 0
                ORDER BY updated_at
                LIMIT {limit}
                "
            ),
        ),
    ];

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

fn start_sync_worker(pool: DbPool) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let pool = pool.clone();
            let result = tauri::async_runtime::spawn_blocking(move || {
                let uploaded = run_auto_sync_once(&pool)?;
                let downloaded = run_auto_pull_once(&pool)?;
                Ok::<(usize, usize), String>((uploaded, downloaded))
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
        .header("Prefer", "resolution=merge-duplicates,return=minimal")
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

#[tauri::command]
fn get_supabase_config(state_db: tauri::State<DbState>) -> AppResult<SupabaseConfig> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
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
    url: String,
    anon_key: String,
) -> AppResult<SupabaseConfig> {
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

    let conn = get_conn(&state_db).map_err(to_command_error)?;
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
fn disconnect_supabase(state_db: tauri::State<DbState>) -> AppResult<()> {
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
fn crear_respaldo_local(state_db: tauri::State<DbState>) -> AppResult<String> {
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
fn aplicar_respaldo_local(state_db: tauri::State<DbState>, backup_json: String) -> AppResult<()> {
    if backup_json.trim().is_empty() {
        return Err("El archivo de respaldo está vacío.".to_string());
    }

    let backup: BackupLocal = serde_json::from_str(&backup_json)
        .map_err(|error| format!("El archivo de respaldo no es válido: {error}"))?;
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    apply_backup_to_conn(&mut conn, backup).map_err(to_command_error)
}

#[tauri::command]
fn sincronizar_hacia_nube(state_db: tauri::State<DbState>) -> AppResult<SyncUploadResult> {
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

    let upload_plan: [(&str, &str, &str); 22] = [
        (
            "sucursales",
            "id",
            "
            SELECT id, nombre, direccion, telefono, codigo_postal, eliminado, 1 AS sincronizado, updated_at
            FROM sucursales
            WHERE sincronizado = 0
            ",
        ),
        (
            "empresa_config_fiscal",
            "id",
            "
            SELECT id, rfc, razon_social, regimen_fiscal, registro_patronal, actualizado_at,
                   1 AS sincronizado, updated_at
            FROM empresa_config_fiscal
            WHERE sincronizado = 0
            ",
        ),
        (
            "proveedores",
            "id",
            "
            SELECT id, nombre, contacto_nombre, telefono, email, direccion, eliminado, 1 AS sincronizado, updated_at
            FROM proveedores
            WHERE sincronizado = 0
            ",
        ),
        (
            "marcas",
            "id",
            "
            SELECT id, nombre, eliminado, 1 AS sincronizado, updated_at
            FROM marcas
            WHERE sincronizado = 0
            ",
        ),
        (
            "unidades",
            "id",
            "
            SELECT id, nombre, clave_sat, eliminado, 1 AS sincronizado, updated_at
            FROM unidades
            WHERE sincronizado = 0
            ",
        ),
        (
            "usuarios",
            "id",
            "
            SELECT id, email, nombre, role, sucursal_id, password_hash, eliminado, 1 AS sincronizado, updated_at
            FROM usuarios
            WHERE sincronizado = 0
            ",
        ),
        (
            "productos",
            "id",
            "
            SELECT id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
                   marca, categoria, unidad, precio_costo, costo_promedio, precio_venta, sat_clave_prod_serv,
                   sat_clave_unidad, eliminado, 1 AS sincronizado, updated_at
            FROM productos
            WHERE sincronizado = 0
            ",
        ),
        (
            "inventario_sucursal",
            "producto_id,sucursal_id",
            "
            SELECT producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta, 1 AS sincronizado, updated_at
            FROM inventario_sucursal
            WHERE sincronizado = 0
            ",
        ),
        (
            "clientes",
            "id",
            "
            SELECT id, nombre, telefono, direccion, limite_credito, saldo_deudor, eliminado, 1 AS sincronizado, updated_at
            FROM clientes
            WHERE sincronizado = 0
            ",
        ),
        (
            "clientes_datos_fiscales",
            "cliente_id",
            "
            SELECT cliente_id, rfc, razon_social, regimen_fiscal, codigo_postal, 1 AS sincronizado, updated_at
            FROM clientes_datos_fiscales
            WHERE sincronizado = 0
            ",
        ),
        (
            "compras",
            "uuid",
            "
            SELECT sync_uuid AS uuid, id, proveedor_id, sucursal_id, fecha, total, 1 AS sincronizado, updated_at
            FROM compras
            WHERE sincronizado = 0
            ",
        ),
        (
            "detalle_compras",
            "uuid",
            "
            SELECT dc.sync_uuid AS uuid, dc.id, dc.compra_id, c.sucursal_id, dc.producto_id,
                   dc.cantidad, dc.precio_costo_pactado, dc.costo_promedio_resultante,
                   1 AS sincronizado, dc.updated_at
            FROM detalle_compras dc
            INNER JOIN compras c ON c.id = dc.compra_id
            WHERE dc.sincronizado = 0
            ",
        ),
        (
            "ventas",
            "uuid",
            "
            SELECT sync_uuid AS uuid, id, usuario_id, sucursal_id, fecha, total, metodo_pago, cliente_id,
                   usuario_autorizo_cancelacion_id, motivo_cancelacion, fecha_cancelacion, estado,
                   1 AS sincronizado, updated_at
            FROM ventas
            WHERE sincronizado = 0
            ",
        ),
        (
            "detalle_ventas",
            "uuid",
            "
            SELECT dv.sync_uuid AS uuid, dv.id, dv.venta_id, v.sucursal_id, dv.producto_id,
                   dv.cantidad, dv.precio_venta_pactado, dv.costo_unitario_pactado,
                   1 AS sincronizado, dv.updated_at
            FROM detalle_ventas dv
            INNER JOIN ventas v ON v.id = dv.venta_id
            WHERE dv.sincronizado = 0
            ",
        ),
        (
            "creditos_abonos",
            "uuid",
            "
            SELECT ca.sync_uuid AS uuid, ca.id, ca.cliente_id, ca.monto, ca.fecha, ca.usuario_id,
                   u.sucursal_id, 1 AS sincronizado, ca.updated_at
            FROM creditos_abonos ca
            INNER JOIN usuarios u ON u.id = ca.usuario_id
            WHERE ca.sincronizado = 0
            ",
        ),
        (
            "cajas_sesiones",
            "uuid",
            "
            SELECT sync_uuid AS uuid, id, usuario_id, sucursal_id, fecha_apertura, monto_inicial,
                   fecha_cierre, monto_final_real, monto_esperado, estado, 1 AS sincronizado, updated_at
            FROM cajas_sesiones
            WHERE sincronizado = 0
            ",
        ),
        (
            "caja_movimientos",
            "uuid",
            "
            SELECT cm.sync_uuid AS uuid, cm.id, cm.sesion_id, cs.sucursal_id, cm.tipo, cm.monto,
                   cm.motivo, 1 AS sincronizado, cm.updated_at
            FROM caja_movimientos cm
            INNER JOIN cajas_sesiones cs ON cs.id = cm.sesion_id
            WHERE cm.sincronizado = 0
            ",
        ),
        (
            "traspasos",
            "uuid",
            "
            SELECT sync_uuid AS uuid, id, sucursal_origen_id, sucursal_destino_id, usuario_id,
                   fecha, estado, usuario_recibio_id, fecha_recepcion, observaciones_recepcion,
                   1 AS sincronizado, updated_at
            FROM traspasos
            WHERE sincronizado = 0
            ",
        ),
        (
            "detalle_traspasos",
            "uuid",
            "
            SELECT dt.sync_uuid AS uuid, dt.id, dt.traspaso_id, t.sucursal_origen_id, dt.producto_id,
                   dt.cantidad, 1 AS sincronizado, dt.updated_at
            FROM detalle_traspasos dt
            INNER JOIN traspasos t ON t.id = dt.traspaso_id
            WHERE dt.sincronizado = 0
            ",
        ),
        (
            "mermas_ajustes",
            "uuid",
            "
            SELECT sync_uuid AS uuid, id, producto_id, sucursal_id, usuario_id, cantidad,
                   tipo_movimiento, motivo, fecha, 1 AS sincronizado, updated_at
            FROM mermas_ajustes
            WHERE sincronizado = 0
            ",
        ),
        (
            "facturas_emitidas",
            "uuid",
            "
            SELECT fe.sync_uuid AS uuid, fe.id, fe.venta_id, v.sucursal_id, fe.uuid AS uuid_sat,
                   fe.rfc_receptor, fe.monto_total, fe.estado, fe.fecha_emision, fe.pdf_path,
                   fe.xml_path, 1 AS sincronizado, fe.updated_at
            FROM facturas_emitidas fe
            INNER JOIN ventas v ON v.id = fe.venta_id
            WHERE fe.sincronizado = 0
            ",
        ),
        (
            "movimientos_inventario",
            "uuid",
            "
            SELECT uuid, producto_id, sucursal_id, tipo, referencia_tipo,
                   referencia_id, cantidad, costo_unitario, usuario_id, fecha,
                   1 AS sincronizado, updated_at
            FROM movimientos_inventario
            WHERE sincronizado = 0
            ",
        ),
    ];

    let mut por_tabla = HashMap::new();
    let mut total_registros = 0;

    for (table, conflict_target, sql) in upload_plan {
        let uploaded = upload_pending_table(&conn, &client, &config, table, conflict_target, sql)?;
        if uploaded > 0 {
            por_tabla.insert(table.to_string(), uploaded);
            total_registros += uploaded;
        }
    }

    Ok(SyncUploadResult {
        total_registros,
        por_tabla,
    })
}

#[tauri::command]
fn sincronizar_desde_nube(state_db: tauri::State<DbState>) -> AppResult<()> {
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
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
        let endpoint = format!("{}/rest/v1/{}?select=*", config.url.trim_end_matches('/'), table);
        let response = supabase_request_builder(&client, &endpoint, &config.anon_key)
            .send()
            .map_err(|error| format!("No se pudo descargar la tabla {table}: {error}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "Supabase rechazó la descarga de {table}. Código HTTP: {}.",
                response.status()
            ));
        }

        let rows: Vec<JsonValue> = response
            .json()
            .map_err(|error| format!("La respuesta de Supabase para {table} no es JSON válido: {error}"))?;
        tablas.insert((*table).to_string(), rows);
    }

    let backup = BackupLocal {
        version: "supabase-rest-v1".to_string(),
        generado_at: current_timestamp_string(),
        tablas,
    };
    apply_backup_to_conn(&mut conn, backup).map_err(to_command_error)
}

#[tauri::command]
fn cancelar_venta(
    state_db: tauri::State<DbState>,
    venta_id: String,
    usuario_autorizo_id: String,
    motivo_cancelacion: String,
    fecha_cancelacion: String,
) -> AppResult<()> {
    if venta_id.trim().is_empty() {
        return Err("Falta el identificador de la venta.".to_string());
    }
    if usuario_autorizo_id.trim().is_empty()
        || motivo_cancelacion.trim().is_empty()
        || fecha_cancelacion.trim().is_empty()
    {
        return Err("La cancelación requiere usuario autorizador, motivo y fecha.".to_string());
    }

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

    let usuario_autorizo_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM usuarios WHERE id = ?1 AND eliminado = 0",
            [&usuario_autorizo_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if usuario_autorizo_exists == 0 {
        return Err("El usuario que autoriza la cancelación no existe.".to_string());
    }

    let caja_id: String = tx
        .query_row(
            "
            SELECT id
            FROM cajas_sesiones
            WHERE sucursal_id = ?1 AND estado = 'ABIERTA'
            ORDER BY fecha_apertura DESC
            LIMIT 1
            ",
            [&sucursal_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(AppError::from)
        .map_err(to_command_error)?
        .ok_or_else(|| "No se puede cancelar una venta sin caja ABIERTA en la sucursal.".to_string())?;

    tx.execute(
        "
        UPDATE ventas
        SET estado = 'CANCELADA',
            usuario_autorizo_cancelacion_id = ?2,
            motivo_cancelacion = ?3,
            fecha_cancelacion = ?4
        WHERE id = ?1
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
            INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo, costo_promedio)
            VALUES (?1, ?2, ?3, 0, ?4)
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
              stock = inventario_sucursal.stock + excluded.stock
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
                END
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
        INSERT INTO caja_movimientos (id, sesion_id, tipo, monto, motivo)
        VALUES (?1, ?2, 'EGRESO', ?3, ?4)
        ",
        params![
            movimiento_id,
            caja_id,
            total,
            format!("DEVOLUCIÓN EN VENTA #{} - {}", venta_id, motivo_cancelacion)
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
    traspaso: RegistrarTraspasoInput,
) -> AppResult<()> {
    let traspaso = RegistrarTraspasoInput {
        detalles: consolidar_detalles_traspaso(&traspaso.detalles),
        ..traspaso
    };
    validate_registrar_traspaso_input(&traspaso).map_err(to_command_error)?;

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
        "INSERT INTO traspasos (id, sucursal_origen_id, sucursal_destino_id, usuario_id, fecha, estado) VALUES (?1, ?2, ?3, ?4, ?5, 'EN_TRANSITO')",
        params![
            traspaso.id,
            traspaso.sucursal_origen_id,
            traspaso.sucursal_destino_id,
            traspaso.usuario_id,
            traspaso.fecha
        ],
    )
    .map_err(|error| map_write_error(error, "traspaso"))
    .map_err(to_command_error)?;

    for detalle in &traspaso.detalles {
        let (_stock_actual, costo_unitario) =
            inventario_costo_promedio(&tx, &detalle.producto_id, &traspaso.sucursal_origen_id)
                .map_err(to_command_error)?;

        tx.execute(
            "INSERT INTO detalle_traspasos (id, traspaso_id, producto_id, cantidad) VALUES (?1, ?2, ?3, ?4)",
            params![detalle.id, traspaso.id, detalle.producto_id, detalle.cantidad],
        )
        .map_err(|error| map_write_error(error, "detalle de traspaso"))
        .map_err(to_command_error)?;

        let affected = tx
            .execute(
            "
            UPDATE inventario_sucursal
            SET stock = stock - ?1
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
    input: RecibirTraspasoInput,
) -> AppResult<()> {
    if input.traspaso_id.trim().is_empty()
        || input.usuario_recibio_id.trim().is_empty()
        || input.fecha_recepcion.trim().is_empty()
    {
        return Err("Datos incompletos para recibir el traspaso.".to_string());
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
            INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo, costo_promedio)
            VALUES (?1, ?2, ?3, 0, ?4)
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
              stock = inventario_sucursal.stock + excluded.stock,
              costo_promedio = CASE
                  WHEN inventario_sucursal.stock + excluded.stock > 0 THEN
                      ((inventario_sucursal.stock * inventario_sucursal.costo_promedio) + (excluded.stock * excluded.costo_promedio))
                      / (inventario_sucursal.stock + excluded.stock)
                  ELSE excluded.costo_promedio
              END
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

    tx.execute(
        "
        UPDATE traspasos
        SET estado = 'RECIBIDO',
            usuario_recibio_id = ?2,
            fecha_recepcion = ?3,
            observaciones_recepcion = ?4
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
fn registrar_merma_ajuste(
    state_db: tauri::State<DbState>,
    movimiento: RegistrarMermaAjusteInput,
) -> AppResult<()> {
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

    let tipo_kardex = match movimiento.tipo_movimiento.as_str() {
        "AJUSTE_ENTRADA" => "AJUSTE_ENTRADA",
        "AJUSTE_SALIDA" | "AJUSTE" => "AJUSTE_SALIDA",
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
            INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo, costo_promedio)
            VALUES (?1, ?2, ?3, 0, ?4)
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
              stock = inventario_sucursal.stock + excluded.stock
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
                SET stock = stock - ?1
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
        INSERT INTO mermas_ajustes (id, producto_id, sucursal_id, usuario_id, cantidad, tipo_movimiento, motivo, fecha)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ",
        params![
            movimiento.id,
            movimiento.producto_id,
            movimiento.sucursal_id,
            movimiento.usuario_id,
            movimiento.cantidad,
            movimiento.tipo_movimiento,
            movimiento.motivo,
            movimiento.fecha
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
fn registrar_venta(
    state_db: tauri::State<DbState>,
    venta: RegistrarVentaInput,
) -> AppResult<()> {
    let venta = RegistrarVentaInput {
        detalles: consolidar_detalles_venta(&venta.detalles).map_err(to_command_error)?,
        ..venta
    };
    validate_registrar_venta_input(&venta).map_err(to_command_error)?;

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let usuario_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM usuarios WHERE id = ?1 AND eliminado = 0",
            [&venta.usuario_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if usuario_exists == 0 {
        return Err("El usuario de la venta ya no existe.".to_string());
    }

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

    let mut total = 0.0_f64;
    for detalle in &venta.detalles {
        total += detalle.cantidad * detalle.precio_venta_pactado;
    }

    if venta.metodo_pago == "CREDITO" {
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

        if saldo_deudor + total > limite_credito {
            return Err("La venta supera el límite de crédito del cliente.".to_string());
        }

        tx.execute(
            "UPDATE clientes SET saldo_deudor = ?1 WHERE id = ?2",
            params![saldo_deudor + total, cliente_id],
        )
        .map_err(|error| map_write_error(error, "cliente"))
        .map_err(to_command_error)?;
    }

    tx.execute(
        "INSERT INTO ventas (id, usuario_id, sucursal_id, fecha, total, metodo_pago, cliente_id, estado) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'COMPLETADA')",
        params![
            venta.id,
            venta.usuario_id,
            venta.sucursal_id,
            venta.fecha,
            total,
            venta.metodo_pago,
            venta.cliente_id
        ],
    )
    .map_err(|error| map_write_error(error, "venta"))
    .map_err(to_command_error)?;

    for detalle in &venta.detalles {
        let (_stock_actual, costo_unitario) =
            inventario_costo_promedio(&tx, &detalle.producto_id, &venta.sucursal_id)
                .map_err(to_command_error)?;

        tx.execute(
            "INSERT INTO detalle_ventas (id, venta_id, producto_id, cantidad, precio_venta_pactado, costo_unitario_pactado) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                detalle.id,
                venta.id,
                detalle.producto_id,
                detalle.cantidad,
                detalle.precio_venta_pactado,
                costo_unitario
            ],
        )
        .map_err(|error| map_write_error(error, "detalle de venta"))
        .map_err(to_command_error)?;

        let affected = tx
            .execute(
            "
            UPDATE inventario_sucursal
            SET stock = stock - ?1
            WHERE producto_id = ?2 AND sucursal_id = ?3 AND stock >= ?1
            ",
            params![detalle.cantidad, detalle.producto_id, venta.sucursal_id],
            )
            .map_err(|error| map_write_error(error, "inventario"))
            .map_err(to_command_error)?;

        if affected != 1 {
            return Err(format!(
                "Stock insuficiente para producto {}. Operación cancelada.",
                detalle.producto_id
            ));
        }

        insertar_movimiento_inventario(
            &tx,
            &detalle.producto_id,
            &venta.sucursal_id,
            "VENTA",
            "VENTA",
            &venta.id,
            -detalle.cantidad,
            Some(costo_unitario),
            Some(&venta.usuario_id),
            &venta.fecha,
        )
        .map_err(to_command_error)?;
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn create_sucursal(state_db: tauri::State<DbState>, sucursal: Sucursal) -> AppResult<()> {
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
fn update_sucursal(state_db: tauri::State<DbState>, id: String, sucursal: Sucursal) -> AppResult<()> {
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
fn delete_sucursal(state_db: tauri::State<DbState>, id: String) -> AppResult<()> {
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
            buscar_productos_por_sucursal,
            get_productos_catalogo,
            create_producto_catalogo,
            update_producto_catalogo,
            guardar_inventario_sucursal,
            create_producto,
            update_producto,
            delete_producto,
            registrar_compra,
            get_caja_actual,
            abrir_caja,
            registrar_movimiento_caja,
            cerrar_caja,
            get_dashboard_stats,
            get_productos_bajo_stock,
            get_productos_mas_vendidos,
            get_historial_ventas,
            get_detalle_venta,
            get_empresa_config,
            guardar_empresa_config,
            get_supabase_config,
            test_and_save_supabase_connect,
            disconnect_supabase,
            crear_respaldo_local,
            aplicar_respaldo_local,
            sincronizar_hacia_nube,
            sincronizar_desde_nube,
            get_sync_status,
            get_notificaciones,
            marcar_notificacion_leida,
            marcar_todas_notificaciones_leidas,
            get_sync_migration_status,
            get_payload_factura,
            actualizar_estado_factura,
            get_facturas_emitidas,
            cancelar_venta,
            registrar_traspaso,
            recibir_traspaso,
            get_historial_traspasos,
            registrar_merma_ajuste,
            get_historial_mermas,
            registrar_venta
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

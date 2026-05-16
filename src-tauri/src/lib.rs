use bcrypt::{hash, verify, DEFAULT_COST};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, Connection, Error as SqliteError};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Mutex;

type DbPool = Pool<SqliteConnectionManager>;
type AppResult<T> = Result<T, String>;

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
    precio_venta: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InventarioSucursalInput {
    sucursal_id: String,
    stock: f64,
    stock_minimo: f64,
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
    precio_venta: f64,
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

fn validate_producto(producto: &Producto) -> Result<(), AppError> {
    if producto.id.trim().is_empty() {
        return Err(AppError::Validation("El producto necesita identificador interno.".to_string()));
    }

    if producto.descripcion.trim().is_empty() {
        return Err(AppError::Validation("El producto necesita descripción.".to_string()));
    }

    if producto.precio_costo < 0.0 || producto.precio_venta < 0.0 {
        return Err(AppError::Validation("Los precios no pueden ser negativos.".to_string()));
    }

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
    if input.tipo_movimiento != "MERMA" && input.tipo_movimiento != "AJUSTE" {
        return Err(AppError::Validation("Tipo de movimiento inválido. Usa MERMA o AJUSTE.".to_string()));
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
        PRAGMA foreign_keys = ON;
        PRAGMA busy_timeout = 5000;

        CREATE TABLE IF NOT EXISTS sucursales (
            id TEXT PRIMARY KEY,
            nombre TEXT NOT NULL,
            direccion TEXT NOT NULL,
            telefono TEXT NOT NULL DEFAULT ''
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

        CREATE TABLE IF NOT EXISTS productos (
            id TEXT PRIMARY KEY,
            codigo_barras TEXT UNIQUE,
            codigo_proveedor TEXT NOT NULL DEFAULT '',
            proveedor_id TEXT NOT NULL DEFAULT '',
            clave_producto TEXT NOT NULL DEFAULT '',
            descripcion TEXT NOT NULL,
            marca TEXT NOT NULL DEFAULT '',
            categoria TEXT NOT NULL DEFAULT '',
            unidad TEXT NOT NULL DEFAULT '',
            precio_costo REAL NOT NULL DEFAULT 0,
            precio_venta REAL NOT NULL DEFAULT 0,
            FOREIGN KEY (proveedor_id) REFERENCES proveedores(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS inventario_sucursal (
            producto_id TEXT NOT NULL,
            sucursal_id TEXT NOT NULL,
            stock REAL NOT NULL DEFAULT 0,
            stock_minimo REAL NOT NULL DEFAULT 0,
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
            estado TEXT NOT NULL DEFAULT 'COMPLETADA' CHECK(estado IN ('COMPLETADA', 'CANCELADA')),
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (cliente_id) REFERENCES clientes(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE TABLE IF NOT EXISTS detalle_ventas (
            id TEXT PRIMARY KEY,
            venta_id TEXT NOT NULL,
            producto_id TEXT NOT NULL,
            cantidad REAL NOT NULL DEFAULT 0,
            precio_venta_pactado REAL NOT NULL DEFAULT 0,
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
            FOREIGN KEY (sucursal_origen_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (sucursal_destino_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT
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
            tipo_movimiento TEXT NOT NULL CHECK(tipo_movimiento IN ('MERMA', 'AJUSTE')),
            motivo TEXT NOT NULL,
            fecha TEXT NOT NULL,
            FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (sucursal_id) REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
            FOREIGN KEY (usuario_id) REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT
        );

        CREATE INDEX IF NOT EXISTS idx_productos_descripcion ON productos(descripcion);
        CREATE INDEX IF NOT EXISTS idx_productos_codigo_barras ON productos(codigo_barras);
        CREATE INDEX IF NOT EXISTS idx_productos_clave_producto ON productos(clave_producto);
        CREATE INDEX IF NOT EXISTS idx_productos_codigo_proveedor ON productos(codigo_proveedor);
        CREATE INDEX IF NOT EXISTS idx_inventario_sucursal_id ON inventario_sucursal(sucursal_id);
        CREATE INDEX IF NOT EXISTS idx_compras_sucursal_fecha ON compras(sucursal_id, fecha);
        CREATE INDEX IF NOT EXISTS idx_detalle_compras_compra ON detalle_compras(compra_id);
        CREATE INDEX IF NOT EXISTS idx_ventas_sucursal_fecha ON ventas(sucursal_id, fecha);
        CREATE INDEX IF NOT EXISTS idx_detalle_ventas_venta ON detalle_ventas(venta_id);
        CREATE INDEX IF NOT EXISTS idx_clientes_nombre ON clientes(nombre);
        CREATE INDEX IF NOT EXISTS idx_abonos_cliente_fecha ON creditos_abonos(cliente_id, fecha);
        CREATE INDEX IF NOT EXISTS idx_cajas_sesiones_usuario_estado ON cajas_sesiones(usuario_id, sucursal_id, estado);
        CREATE INDEX IF NOT EXISTS idx_caja_movimientos_sesion ON caja_movimientos(sesion_id);
        CREATE INDEX IF NOT EXISTS idx_traspasos_fecha ON traspasos(fecha);
        CREATE INDEX IF NOT EXISTS idx_detalle_traspasos_traspaso ON detalle_traspasos(traspaso_id);
        CREATE INDEX IF NOT EXISTS idx_mermas_fecha ON mermas_ajustes(fecha);
        CREATE INDEX IF NOT EXISTS idx_mermas_sucursal ON mermas_ajustes(sucursal_id);
        ",
    )?;

    migrate_user_role_schema(conn)?;
    migrate_productos_add_proveedor(conn)?;
    migrate_ventas_add_estado_cliente(conn)?;

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

    conn.execute("ALTER TABLE productos ADD COLUMN proveedor_id TEXT NOT NULL DEFAULT ''", [])?;
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
            "SELECT id, email, nombre, role, sucursal_id, password_hash FROM usuarios WHERE email = ?1",
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
            "UPDATE usuarios SET nombre = ?1, email = ?2, password_hash = ?3 WHERE id = ?4",
            params![nombre, email, password_hash, usuario_actual.id],
        )
        .map_err(|error| map_write_error(error, "usuario"))
        .map_err(to_command_error)?;
    } else {
        conn.execute(
            "UPDATE usuarios SET nombre = ?1, email = ?2 WHERE id = ?3",
            params![nombre, email, usuario_actual.id],
        )
        .map_err(|error| map_write_error(error, "usuario"))
        .map_err(to_command_error)?;
    }

    let usuario_actualizado = conn
        .query_row(
            "SELECT id, email, nombre, role, sucursal_id FROM usuarios WHERE id = ?1",
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
        .query_row("SELECT COUNT(*) FROM usuarios", [], |row| row.get(0))
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
        .query_row("SELECT COUNT(*) FROM usuarios", [], |row| row.get(0))
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if user_count > 0 {
        return Err("El sistema ya tiene usuarios registrados.".to_string());
    }

    let password_hash = hash(password, DEFAULT_COST).map_err(AppError::from).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    tx.execute(
        "INSERT INTO sucursales (id, nombre, direccion, telefono) VALUES (?1, ?2, ?3, ?4)",
        params![sucursal.id, sucursal.nombre, sucursal.direccion, sucursal.telefono],
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
fn get_usuarios(state_db: tauri::State<DbState>) -> AppResult<Vec<Usuario>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, email, nombre, role, sucursal_id FROM usuarios ORDER BY nombre")
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let usuarios_iter = stmt
        .query_map([], |row| {
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
fn create_usuario(state_db: tauri::State<DbState>, usuario: Usuario, password: String) -> AppResult<()> {
    validate_usuario(&usuario, false).map_err(to_command_error)?;

    if password.trim().is_empty() {
        return Err("El usuario necesita contraseña.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;
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
fn update_usuario(state_db: tauri::State<DbState>, id: String, usuario: Usuario) -> AppResult<()> {
    validate_usuario(&usuario, false).map_err(to_command_error)?;

    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let affected = conn
        .execute(
            "UPDATE usuarios SET email = ?1, nombre = ?2, role = ?3, sucursal_id = ?4 WHERE id = ?5",
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
fn delete_usuario(state_db: tauri::State<DbState>, id: String) -> AppResult<()> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let affected = conn
        .execute("DELETE FROM usuarios WHERE id = ?1", [&id])
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
        .prepare("SELECT id, nombre, direccion, telefono FROM sucursales ORDER BY nombre")
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let sucursales_iter = stmt
        .query_map([], |row| {
            Ok(Sucursal {
                id: row.get(0)?,
                nombre: row.get(1)?,
                direccion: row.get(2)?,
                telefono: row.get(3)?,
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
        .prepare("SELECT id, nombre, contacto_nombre, telefono, email, direccion FROM proveedores ORDER BY nombre")
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
            "UPDATE proveedores SET nombre = ?1, contacto_nombre = ?2, telefono = ?3, email = ?4, direccion = ?5 WHERE id = ?6",
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
            "SELECT COUNT(*) FROM productos WHERE proveedor_id = ?1",
            [&id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if active_products > 0 {
        return Err("No se puede eliminar el proveedor porque tiene productos asociados.".to_string());
    }

    let affected = conn
        .execute("DELETE FROM proveedores WHERE id = ?1", [&id])
        .map_err(|error| map_write_error(error, "proveedor"))
        .map_err(to_command_error)?;
    if affected == 0 {
        return Err("No se encontró el proveedor que intentas eliminar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn get_clientes(state_db: tauri::State<DbState>) -> AppResult<Vec<Cliente>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare("SELECT id, nombre, telefono, direccion, limite_credito, saldo_deudor FROM clientes ORDER BY nombre")
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
            "UPDATE clientes SET nombre = ?1, telefono = ?2, direccion = ?3, limite_credito = ?4 WHERE id = ?5",
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
            "SELECT COALESCE(saldo_deudor, 0) FROM clientes WHERE id = ?1",
            [&id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if saldo > 0.0 {
        return Err("No se puede eliminar cliente con saldo deudor pendiente.".to_string());
    }

    let affected = conn
        .execute("DELETE FROM clientes WHERE id = ?1", [&id])
        .map_err(|error| map_write_error(error, "cliente"))
        .map_err(to_command_error)?;
    if affected == 0 {
        return Err("No se encontró el cliente que intentas eliminar.".to_string());
    }
    Ok(())
}

#[tauri::command]
fn registrar_abono(state_db: tauri::State<DbState>, abono: AbonoCreditoInput) -> AppResult<()> {
    validate_abono_credito(&abono).map_err(to_command_error)?;
    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let saldo_actual: f64 = tx
        .query_row(
            "SELECT saldo_deudor FROM clientes WHERE id = ?1",
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
                p.precio_costo,
                p.precio_venta,
                i.sucursal_id,
                i.stock,
                i.stock_minimo
            FROM productos p
            INNER JOIN inventario_sucursal i ON i.producto_id = p.id
            WHERE i.sucursal_id = ?1
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
                precio_venta: row.get(10)?,
                sucursal_id: row.get(11)?,
                stock: row.get(12)?,
                stock_minimo: row.get(13)?,
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
    let pattern = format!("%{}%", query.trim().to_lowercase());

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
                p.precio_costo,
                p.precio_venta,
                i.sucursal_id,
                i.stock,
                i.stock_minimo
            FROM productos p
            INNER JOIN inventario_sucursal i ON i.producto_id = p.id
            WHERE i.sucursal_id = ?1
              AND (
                LOWER(p.descripcion) LIKE ?2
                OR LOWER(p.codigo_barras) LIKE ?2
                OR LOWER(p.codigo_proveedor) LIKE ?2
                OR LOWER(p.clave_producto) LIKE ?2
              )
            ORDER BY p.descripcion
            ",
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map(params![sucursal_id, pattern], |row| {
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
                precio_venta: row.get(10)?,
                sucursal_id: row.get(11)?,
                stock: row.get(12)?,
                stock_minimo: row.get(13)?,
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
fn create_producto(
    state_db: tauri::State<DbState>,
    producto: Producto,
    inventario: InventarioSucursalInput,
) -> AppResult<()> {
    validate_producto(&producto).map_err(to_command_error)?;
    validate_inventario_input(&inventario).map_err(to_command_error)?;

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    tx.execute(
        "
        INSERT INTO productos (
            id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
            marca, categoria, unidad, precio_costo, precio_venta
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ",
        params![
            producto.id,
            if producto.codigo_barras.trim().is_empty() {
                None::<String>
            } else {
                Some(producto.codigo_barras)
            },
            producto.codigo_proveedor,
            producto.proveedor_id,
            producto.clave_producto,
            producto.descripcion,
            producto.marca,
            producto.categoria,
            producto.unidad,
            producto.precio_costo,
            producto.precio_venta
        ],
    )
    .map_err(|error| map_write_error(error, "producto"))
    .map_err(to_command_error)?;

    tx.execute(
        "
        INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo)
        VALUES (?1, ?2, ?3, ?4)
        ",
        params![producto.id, inventario.sucursal_id, inventario.stock, inventario.stock_minimo],
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
    validate_producto(&producto).map_err(to_command_error)?;
    validate_inventario_input(&inventario).map_err(to_command_error)?;

    if producto_id.trim().is_empty() {
        return Err("Falta el identificador del producto a actualizar.".to_string());
    }

    let conn = get_conn(&state_db).map_err(to_command_error)?;
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
            precio_venta = ?10
        WHERE id = ?11
        ",
        params![
            if producto.codigo_barras.trim().is_empty() {
                None::<String>
            } else {
                Some(producto.codigo_barras)
            },
            producto.codigo_proveedor,
            producto.proveedor_id,
            producto.clave_producto,
            producto.descripcion,
            producto.marca,
            producto.categoria,
            producto.unidad,
            producto.precio_costo,
            producto.precio_venta,
            producto_id
        ],
    )
    .map_err(|error| map_write_error(error, "producto"))
    .map_err(to_command_error)?;

    conn.execute(
        "
        INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo)
        VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
          stock = excluded.stock,
          stock_minimo = excluded.stock_minimo
        ",
        params![producto_id, inventario.sucursal_id, inventario.stock, inventario.stock_minimo],
    )
    .map_err(|error| map_write_error(error, "inventario"))
    .map_err(to_command_error)?;

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
            "SELECT COUNT(*) FROM proveedores WHERE id = ?1",
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
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1",
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
        tx.execute(
            "INSERT INTO detalle_compras (id, compra_id, producto_id, cantidad, precio_costo_pactado) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                detalle.id,
                compra.id,
                detalle.producto_id,
                detalle.cantidad,
                detalle.precio_costo_pactado
            ],
        )
        .map_err(|error| map_write_error(error, "detalle de compra"))
        .map_err(to_command_error)?;

        tx.execute(
            "
            INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo)
            VALUES (?1, ?2, ?3, 0)
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
              stock = inventario_sucursal.stock + excluded.stock
            ",
            params![detalle.producto_id, compra.sucursal_id, detalle.cantidad],
        )
        .map_err(|error| map_write_error(error, "inventario"))
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
            "SELECT COALESCE(SUM(monto), 0) FROM caja_movimientos WHERE sesion_id = ?1 AND tipo = 'EGRESO'",
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
    filtro: Option<DashboardFiltroInput>,
) -> AppResult<DashboardStats> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let filtro_ref = filtro.as_ref();
    let sucursal_id = filtro_ref.and_then(|f| normalize_filter(&f.sucursal_id));
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
                SELECT COALESCE(SUM((dv.precio_venta_pactado - p.precio_costo) * dv.cantidad), 0)
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
                SELECT COALESCE(SUM((dv.precio_venta_pactado - p.precio_costo) * dv.cantidad), 0)
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
                    SELECT COALESCE(SUM((dv.precio_venta_pactado - p.precio_costo) * dv.cantidad), 0)
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
                    SELECT COALESCE(SUM((dv.precio_venta_pactado - p.precio_costo) * dv.cantidad), 0)
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
    sucursal_id: Option<String>,
) -> AppResult<Vec<ProductoBajoStock>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let sid = normalize_filter(&sucursal_id);

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
    filtro: Option<DashboardFiltroInput>,
) -> AppResult<Vec<ProductoMasVendido>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let filtro_ref = filtro.as_ref();
    let sucursal_id = filtro_ref.and_then(|f| normalize_filter(&f.sucursal_id));
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
    filtro: Option<HistorialVentasFiltro>,
) -> AppResult<Vec<HistorialVenta>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let filtro_ref = filtro.as_ref();
    let fi = filtro_ref
        .and_then(|f| normalize_filter(&f.fecha_inicio))
        .unwrap_or_else(|| "0001-01-01T00:00:00.000Z".to_string());
    let ff = filtro_ref
        .and_then(|f| normalize_filter(&f.fecha_fin))
        .unwrap_or_else(|| "9999-12-31T23:59:59.999Z".to_string());
    let sid = filtro_ref.and_then(|f| normalize_filter(&f.sucursal_id));
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
    venta_id: String,
) -> AppResult<Vec<HistorialVentaDetalle>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare(
            "
            SELECT dv.id, dv.venta_id, dv.producto_id, p.descripcion, p.marca, dv.cantidad, dv.precio_venta_pactado
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

#[tauri::command]
fn cancelar_venta(state_db: tauri::State<DbState>, venta_id: String) -> AppResult<()> {
    if venta_id.trim().is_empty() {
        return Err("Falta el identificador de la venta.".to_string());
    }

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let (usuario_id, sucursal_id, metodo_pago, total, estado, cliente_id): (
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

    tx.execute("UPDATE ventas SET estado = 'CANCELADA' WHERE id = ?1", [&venta_id])
        .map_err(|error| map_write_error(error, "venta"))
        .map_err(to_command_error)?;

    let mut stmt = tx
        .prepare("SELECT producto_id, cantidad FROM detalle_ventas WHERE venta_id = ?1")
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let iter = stmt
        .query_map([&venta_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)))
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let mut items: Vec<(String, f64)> = Vec::new();
    for item in iter {
        items.push(item.map_err(AppError::from).map_err(to_command_error)?);
    }
    drop(stmt);

    for (producto_id, cantidad) in items {
        tx.execute(
            "
            INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo)
            VALUES (?1, ?2, ?3, 0)
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
              stock = inventario_sucursal.stock + excluded.stock
            ",
            params![producto_id, sucursal_id, cantidad],
        )
        .map_err(|error| map_write_error(error, "inventario"))
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

    if metodo_pago == "EFECTIVO" {
        let caja_abierta: Option<String> = tx
            .query_row(
                "
                SELECT id
                FROM cajas_sesiones
                WHERE usuario_id = ?1 AND sucursal_id = ?2 AND estado = 'ABIERTA'
                ORDER BY fecha_apertura DESC
                LIMIT 1
                ",
                params![usuario_id, sucursal_id],
                |row| row.get(0),
            )
            .ok();

        if let Some(caja_id) = caja_abierta {
            tx.execute(
                "
                UPDATE cajas_sesiones
                SET monto_esperado = CASE
                    WHEN monto_esperado - ?1 < 0 THEN 0
                    ELSE monto_esperado - ?1
                END
                WHERE id = ?2
                ",
                params![total, caja_id],
            )
            .map_err(|error| map_write_error(error, "caja"))
            .map_err(to_command_error)?;
        }
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn registrar_traspaso(
    state_db: tauri::State<DbState>,
    traspaso: RegistrarTraspasoInput,
) -> AppResult<()> {
    validate_registrar_traspaso_input(&traspaso).map_err(to_command_error)?;

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let sucursal_origen_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1",
            [&traspaso.sucursal_origen_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    let sucursal_destino_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1",
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
            "SELECT COUNT(*) FROM usuarios WHERE id = ?1",
            [&traspaso.usuario_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if usuario_exists == 0 {
        return Err("El usuario que registra el traspaso no existe.".to_string());
    }

    for detalle in &traspaso.detalles {
        let stock_origen: Option<f64> = tx
            .query_row(
                "SELECT stock FROM inventario_sucursal WHERE producto_id = ?1 AND sucursal_id = ?2",
                params![detalle.producto_id, traspaso.sucursal_origen_id],
                |row| row.get(0),
            )
            .ok();
        let stock = stock_origen.unwrap_or(0.0);
        if stock < detalle.cantidad {
            return Err(format!(
                "Stock insuficiente para producto {} en sucursal origen. Disponible: {}, solicitado: {}.",
                detalle.producto_id, stock, detalle.cantidad
            ));
        }
    }

    tx.execute(
        "INSERT INTO traspasos (id, sucursal_origen_id, sucursal_destino_id, usuario_id, fecha) VALUES (?1, ?2, ?3, ?4, ?5)",
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
        tx.execute(
            "INSERT INTO detalle_traspasos (id, traspaso_id, producto_id, cantidad) VALUES (?1, ?2, ?3, ?4)",
            params![detalle.id, traspaso.id, detalle.producto_id, detalle.cantidad],
        )
        .map_err(|error| map_write_error(error, "detalle de traspaso"))
        .map_err(to_command_error)?;

        tx.execute(
            "
            UPDATE inventario_sucursal
            SET stock = stock - ?1
            WHERE producto_id = ?2 AND sucursal_id = ?3
            ",
            params![detalle.cantidad, detalle.producto_id, traspaso.sucursal_origen_id],
        )
        .map_err(|error| map_write_error(error, "inventario origen"))
        .map_err(to_command_error)?;

        tx.execute(
            "
            INSERT INTO inventario_sucursal (producto_id, sucursal_id, stock, stock_minimo)
            VALUES (?1, ?2, ?3, 0)
            ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
              stock = inventario_sucursal.stock + excluded.stock
            ",
            params![detalle.producto_id, traspaso.sucursal_destino_id, detalle.cantidad],
        )
        .map_err(|error| map_write_error(error, "inventario destino"))
        .map_err(to_command_error)?;
    }

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn get_historial_traspasos(state_db: tauri::State<DbState>) -> AppResult<Vec<HistorialTraspaso>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare(
            "
            SELECT
              t.id,
              t.sucursal_origen_id,
              so.nombre,
              t.sucursal_destino_id,
              sd.nombre,
              t.usuario_id,
              u.nombre,
              t.fecha
            FROM traspasos t
            INNER JOIN sucursales so ON so.id = t.sucursal_origen_id
            INNER JOIN sucursales sd ON sd.id = t.sucursal_destino_id
            INNER JOIN usuarios u ON u.id = t.usuario_id
            ORDER BY t.fecha DESC
            ",
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map([], |row| {
            Ok(HistorialTraspaso {
                id: row.get(0)?,
                sucursal_origen_id: row.get(1)?,
                sucursal_origen_nombre: row.get(2)?,
                sucursal_destino_id: row.get(3)?,
                sucursal_destino_nombre: row.get(4)?,
                usuario_id: row.get(5)?,
                usuario_nombre: row.get(6)?,
                fecha: row.get(7)?,
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
            "SELECT COUNT(*) FROM productos WHERE id = ?1",
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
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1",
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
            "SELECT COUNT(*) FROM usuarios WHERE id = ?1",
            [&movimiento.usuario_id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;
    if usuario_exists == 0 {
        return Err("El usuario que registra el ajuste no existe.".to_string());
    }

    let stock_actual: Option<f64> = tx
        .query_row(
            "SELECT stock FROM inventario_sucursal WHERE producto_id = ?1 AND sucursal_id = ?2",
            params![movimiento.producto_id, movimiento.sucursal_id],
            |row| row.get(0),
        )
        .ok();
    let stock = stock_actual.unwrap_or(0.0);
    if stock < movimiento.cantidad {
        return Err(format!(
            "Stock insuficiente. Disponible: {}, solicitado: {}.",
            stock, movimiento.cantidad
        ));
    }

    tx.execute(
        "
        UPDATE inventario_sucursal
        SET stock = stock - ?1
        WHERE producto_id = ?2 AND sucursal_id = ?3
        ",
        params![movimiento.cantidad, movimiento.producto_id, movimiento.sucursal_id],
    )
    .map_err(|error| map_write_error(error, "inventario"))
    .map_err(to_command_error)?;

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

    tx.commit().map_err(AppError::from).map_err(to_command_error)?;
    Ok(())
}

#[tauri::command]
fn get_historial_mermas(state_db: tauri::State<DbState>) -> AppResult<Vec<HistorialMerma>> {
    let conn = get_conn(&state_db).map_err(to_command_error)?;
    let mut stmt = conn
        .prepare(
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
              p.precio_costo,
              (m.cantidad * p.precio_costo) AS costo_total
            FROM mermas_ajustes m
            INNER JOIN productos p ON p.id = m.producto_id
            INNER JOIN sucursales s ON s.id = m.sucursal_id
            INNER JOIN usuarios u ON u.id = m.usuario_id
            ORDER BY m.fecha DESC
            ",
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    let iter = stmt
        .query_map([], |row| {
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
    validate_registrar_venta_input(&venta).map_err(to_command_error)?;

    let mut conn = get_conn(&state_db).map_err(to_command_error)?;
    let tx = conn.transaction().map_err(AppError::from).map_err(to_command_error)?;

    let usuario_exists: i64 = tx
        .query_row(
            "SELECT COUNT(*) FROM usuarios WHERE id = ?1",
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
            "SELECT COUNT(*) FROM sucursales WHERE id = ?1",
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

    for detalle in &venta.detalles {
        let stock_actual: Option<f64> = tx
            .query_row(
                "SELECT stock FROM inventario_sucursal WHERE producto_id = ?1 AND sucursal_id = ?2",
                params![detalle.producto_id, venta.sucursal_id],
                |row| row.get(0),
            )
            .ok();

        let stock = stock_actual.unwrap_or(0.0);
        if stock < detalle.cantidad {
            return Err(format!(
                "Stock insuficiente para producto {}. Disponible: {}, solicitado: {}.",
                detalle.producto_id, stock, detalle.cantidad
            ));
        }
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
                "SELECT limite_credito, saldo_deudor FROM clientes WHERE id = ?1",
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
        tx.execute(
            "INSERT INTO detalle_ventas (id, venta_id, producto_id, cantidad, precio_venta_pactado) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                detalle.id,
                venta.id,
                detalle.producto_id,
                detalle.cantidad,
                detalle.precio_venta_pactado
            ],
        )
        .map_err(|error| map_write_error(error, "detalle de venta"))
        .map_err(to_command_error)?;

        tx.execute(
            "
            UPDATE inventario_sucursal
            SET stock = stock - ?1
            WHERE producto_id = ?2 AND sucursal_id = ?3
            ",
            params![detalle.cantidad, detalle.producto_id, venta.sucursal_id],
        )
        .map_err(|error| map_write_error(error, "inventario"))
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
        "INSERT INTO sucursales (id, nombre, direccion, telefono) VALUES (?1, ?2, ?3, ?4)",
        params![sucursal.id, sucursal.nombre, sucursal.direccion, sucursal.telefono],
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
            "UPDATE sucursales SET nombre = ?1, direccion = ?2, telefono = ?3 WHERE id = ?4",
            params![sucursal.nombre, sucursal.direccion, sucursal.telefono, id],
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
            "SELECT COUNT(*) FROM usuarios WHERE sucursal_id = ?1",
            [&id],
            |row| row.get(0),
        )
        .map_err(AppError::from)
        .map_err(to_command_error)?;

    if active_users > 0 {
        return Err("No se puede eliminar la sucursal porque tiene usuarios activos.".to_string());
    }

    let affected = conn
        .execute("DELETE FROM sucursales WHERE id = ?1", [&id])
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
        conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;")
    });

    let pool = Pool::builder()
        .max_size(8)
        .build(manager)
        .expect("No se pudo crear el pool de conexiones SQLite");

    {
        let conn = pool.get().expect("No se pudo abrir DB");
        init_db(&conn).expect("No se pudo inicializar el esquema de DB");
    }

    tauri::Builder::default()
        .manage(DbState(pool))
        .manage(SesionActual(Mutex::new(None)))
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
            get_clientes,
            create_cliente,
            update_cliente,
            delete_cliente,
            registrar_abono,
            create_sucursal,
            update_sucursal,
            delete_sucursal,
            get_productos_por_sucursal,
            buscar_productos_por_sucursal,
            create_producto,
            update_producto,
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
            cancelar_venta,
            registrar_traspaso,
            get_historial_traspasos,
            registrar_merma_ajuste,
            get_historial_mermas,
            registrar_venta
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

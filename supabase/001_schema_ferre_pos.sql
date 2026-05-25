-- Ferre-POS - Esquema maestro Supabase/PostgreSQL
-- Ejecutar en el SQL Editor de Supabase.
-- Estrategia:
-- 1. Catalogos y entidades maestras conservan id TEXT compatible con SQLite.
-- 2. Tablas transaccionales usan uuid UUID como identificador global para evitar colisiones offline.
-- 3. Todas las tablas sincronizables incluyen sincronizado BOOLEAN y updated_at TIMESTAMPTZ.

CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = now();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION touch_updated_at(table_name TEXT)
RETURNS VOID AS $$
BEGIN
  EXECUTE format('DROP TRIGGER IF EXISTS trg_%I_updated_at ON %I', table_name, table_name);
  EXECUTE format(
    'CREATE TRIGGER trg_%I_updated_at BEFORE UPDATE ON %I
     FOR EACH ROW EXECUTE FUNCTION set_updated_at()',
    table_name,
    table_name
  );
END;
$$ LANGUAGE plpgsql;

CREATE TABLE IF NOT EXISTS sucursales (
  id TEXT PRIMARY KEY,
  nombre TEXT NOT NULL,
  direccion TEXT NOT NULL,
  telefono TEXT NOT NULL DEFAULT '',
  codigo_postal VARCHAR(5) NOT NULL DEFAULT '',
  eliminado BOOLEAN NOT NULL DEFAULT FALSE,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS empresa_config_fiscal (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  rfc VARCHAR(13) NOT NULL DEFAULT '',
  razon_social TEXT NOT NULL DEFAULT '',
  regimen_fiscal VARCHAR(3) NOT NULL DEFAULT '',
  registro_patronal TEXT NULL,
  actualizado_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS proveedores (
  id TEXT PRIMARY KEY,
  nombre TEXT NOT NULL,
  contacto_nombre TEXT NOT NULL DEFAULT '',
  telefono TEXT NOT NULL DEFAULT '',
  email TEXT NOT NULL DEFAULT '',
  direccion TEXT NOT NULL DEFAULT '',
  eliminado BOOLEAN NOT NULL DEFAULT FALSE,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS usuarios (
  id TEXT PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  nombre TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('SUPERADMIN', 'ADMIN', 'USUARIO')),
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  password_hash TEXT NOT NULL,
  eliminado BOOLEAN NOT NULL DEFAULT FALSE,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS marcas (
  id TEXT PRIMARY KEY,
  nombre TEXT NOT NULL UNIQUE,
  eliminado BOOLEAN NOT NULL DEFAULT FALSE,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS unidades (
  id TEXT PRIMARY KEY,
  nombre TEXT NOT NULL UNIQUE,
  clave_sat VARCHAR(3) NOT NULL DEFAULT '',
  eliminado BOOLEAN NOT NULL DEFAULT FALSE,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS productos (
  id TEXT PRIMARY KEY,
  codigo_barras TEXT UNIQUE,
  codigo_proveedor TEXT NOT NULL DEFAULT '',
  proveedor_id TEXT NOT NULL REFERENCES proveedores(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  clave_producto TEXT NOT NULL DEFAULT '',
  descripcion TEXT NOT NULL,
  marca TEXT NOT NULL DEFAULT '',
  categoria TEXT NOT NULL DEFAULT '',
  unidad TEXT NOT NULL DEFAULT '',
  precio_costo NUMERIC(12, 2) NOT NULL DEFAULT 0,
  costo_promedio NUMERIC(12, 4) NOT NULL DEFAULT 0,
  precio_venta NUMERIC(12, 2) NOT NULL DEFAULT 0,
  sat_clave_prod_serv VARCHAR(8) NOT NULL DEFAULT '',
  sat_clave_unidad VARCHAR(3) NOT NULL DEFAULT '',
  eliminado BOOLEAN NOT NULL DEFAULT FALSE,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS inventario_sucursal (
  producto_id TEXT NOT NULL REFERENCES productos(id) ON UPDATE CASCADE ON DELETE CASCADE,
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  stock NUMERIC(12, 3) NOT NULL DEFAULT 0,
  stock_minimo NUMERIC(12, 3) NOT NULL DEFAULT 0,
  costo_promedio NUMERIC(12, 4) NOT NULL DEFAULT 0,
  precio_venta NUMERIC(12, 2) NOT NULL DEFAULT 0,
  eliminado BOOLEAN NOT NULL DEFAULT FALSE,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  PRIMARY KEY (producto_id, sucursal_id)
);

CREATE TABLE IF NOT EXISTS clientes (
  id TEXT PRIMARY KEY,
  nombre TEXT NOT NULL,
  telefono TEXT NOT NULL DEFAULT '',
  direccion TEXT NOT NULL DEFAULT '',
  limite_credito NUMERIC(12, 2) NOT NULL DEFAULT 0,
  saldo_deudor NUMERIC(12, 2) NOT NULL DEFAULT 0,
  eliminado BOOLEAN NOT NULL DEFAULT FALSE,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS clientes_datos_fiscales (
  cliente_id TEXT PRIMARY KEY REFERENCES clientes(id) ON UPDATE CASCADE ON DELETE CASCADE,
  rfc VARCHAR(13) NOT NULL UNIQUE,
  razon_social TEXT NOT NULL,
  regimen_fiscal VARCHAR(3) NOT NULL,
  codigo_postal VARCHAR(5) NOT NULL,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS compras (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  id TEXT NOT NULL,
  proveedor_id TEXT NOT NULL REFERENCES proveedores(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  fecha TIMESTAMPTZ NOT NULL,
  total NUMERIC(12, 2) NOT NULL DEFAULT 0,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (id, sucursal_id)
);

CREATE TABLE IF NOT EXISTS detalle_compras (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  id TEXT NOT NULL,
  compra_uuid UUID NULL REFERENCES compras(uuid) ON UPDATE CASCADE ON DELETE CASCADE,
  compra_id TEXT NOT NULL,
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  producto_id TEXT NOT NULL REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  cantidad NUMERIC(12, 3) NOT NULL DEFAULT 0,
  precio_costo_pactado NUMERIC(12, 2) NOT NULL DEFAULT 0,
  costo_promedio_resultante NUMERIC(12, 4) NULL,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (id, sucursal_id)
);

CREATE TABLE IF NOT EXISTS ventas (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  id TEXT NOT NULL,
  usuario_id TEXT NOT NULL REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  fecha TIMESTAMPTZ NOT NULL,
  total NUMERIC(12, 2) NOT NULL DEFAULT 0,
  metodo_pago TEXT NOT NULL CHECK (metodo_pago IN ('EFECTIVO', 'TARJETA', 'TRANSFERENCIA', 'CREDITO')),
  efectivo_recibido NUMERIC(12, 2) NULL,
  cambio_entregado NUMERIC(12, 2) NULL,
  cliente_id TEXT NULL REFERENCES clientes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  usuario_autorizo_cancelacion_id TEXT NULL REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  motivo_cancelacion TEXT NULL,
  fecha_cancelacion TIMESTAMPTZ NULL,
  estado TEXT NOT NULL DEFAULT 'COMPLETADA' CHECK (estado IN ('COMPLETADA', 'CANCELADA')),
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (id, sucursal_id)
);

CREATE TABLE IF NOT EXISTS detalle_ventas (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  id TEXT NOT NULL,
  venta_uuid UUID NULL REFERENCES ventas(uuid) ON UPDATE CASCADE ON DELETE CASCADE,
  venta_id TEXT NOT NULL,
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  producto_id TEXT NOT NULL REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  cantidad NUMERIC(12, 3) NOT NULL DEFAULT 0,
  precio_venta_pactado NUMERIC(12, 2) NOT NULL DEFAULT 0,
  costo_unitario_pactado NUMERIC(12, 4) NOT NULL DEFAULT 0,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (id, sucursal_id)
);

CREATE TABLE IF NOT EXISTS creditos_abonos (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  id TEXT NOT NULL,
  cliente_id TEXT NOT NULL REFERENCES clientes(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  monto NUMERIC(12, 2) NOT NULL DEFAULT 0,
  fecha TIMESTAMPTZ NOT NULL,
  usuario_id TEXT NOT NULL REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (id, sucursal_id)
);

CREATE TABLE IF NOT EXISTS cajas_sesiones (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  id TEXT NOT NULL,
  usuario_id TEXT NOT NULL REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  fecha_apertura TIMESTAMPTZ NOT NULL,
  monto_inicial NUMERIC(12, 2) NOT NULL DEFAULT 0,
  fecha_cierre TIMESTAMPTZ NULL,
  monto_final_real NUMERIC(12, 2) NULL,
  monto_esperado NUMERIC(12, 2) NOT NULL DEFAULT 0,
  estado TEXT NOT NULL CHECK (estado IN ('ABIERTA', 'CERRADA')),
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (id, sucursal_id)
);

CREATE TABLE IF NOT EXISTS caja_movimientos (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  id TEXT NOT NULL,
  sesion_uuid UUID NULL REFERENCES cajas_sesiones(uuid) ON UPDATE CASCADE ON DELETE CASCADE,
  sesion_id TEXT NOT NULL,
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  tipo TEXT NOT NULL CHECK (tipo IN ('INGRESO', 'EGRESO')),
  monto NUMERIC(12, 2) NOT NULL DEFAULT 0,
  motivo TEXT NOT NULL DEFAULT '',
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (id, sucursal_id)
);

CREATE TABLE IF NOT EXISTS traspasos (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  id TEXT NOT NULL,
  sucursal_origen_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  sucursal_destino_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  usuario_id TEXT NOT NULL REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  fecha TIMESTAMPTZ NOT NULL,
  estado TEXT NOT NULL DEFAULT 'EN_TRANSITO' CHECK (estado IN ('EN_TRANSITO', 'RECIBIDO', 'RECHAZADO', 'CANCELADO')),
  usuario_recibio_id TEXT NULL REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  fecha_recepcion TIMESTAMPTZ NULL,
  observaciones_recepcion TEXT NULL,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (id, sucursal_origen_id)
);

CREATE TABLE IF NOT EXISTS detalle_traspasos (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  id TEXT NOT NULL,
  traspaso_uuid UUID NULL REFERENCES traspasos(uuid) ON UPDATE CASCADE ON DELETE CASCADE,
  traspaso_id TEXT NOT NULL,
  sucursal_origen_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  producto_id TEXT NOT NULL REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  cantidad NUMERIC(12, 3) NOT NULL DEFAULT 0,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (id, sucursal_origen_id)
);

CREATE TABLE IF NOT EXISTS mermas_ajustes (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  id TEXT NOT NULL,
  producto_id TEXT NOT NULL REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  usuario_id TEXT NOT NULL REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  cantidad NUMERIC(12, 3) NOT NULL DEFAULT 0,
  tipo_movimiento TEXT NOT NULL CHECK (tipo_movimiento IN ('MERMA', 'AJUSTE', 'AJUSTE_ENTRADA', 'AJUSTE_SALIDA')),
  motivo TEXT NOT NULL,
  fecha TIMESTAMPTZ NOT NULL,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (id, sucursal_id)
);

CREATE TABLE IF NOT EXISTS facturas_emitidas (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  id TEXT NOT NULL,
  venta_uuid UUID NULL REFERENCES ventas(uuid) ON UPDATE CASCADE ON DELETE RESTRICT,
  venta_id TEXT NOT NULL,
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  uuid_sat TEXT NULL,
  rfc_receptor VARCHAR(13) NOT NULL,
  monto_total NUMERIC(12, 2) NOT NULL DEFAULT 0,
  estado TEXT NOT NULL DEFAULT 'PENDIENTE' CHECK (estado IN ('PENDIENTE', 'TIMBRADA', 'CANCELADA')),
  fecha_emision TIMESTAMPTZ NOT NULL,
  pdf_path TEXT NULL,
  xml_path TEXT NULL,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (id, sucursal_id),
  UNIQUE (venta_id, sucursal_id)
);

CREATE TABLE IF NOT EXISTS movimientos_inventario (
  uuid UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  producto_id TEXT NOT NULL REFERENCES productos(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  sucursal_id TEXT NOT NULL REFERENCES sucursales(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  tipo TEXT NOT NULL CHECK (tipo IN ('COMPRA', 'VENTA', 'CANCELACION_VENTA', 'TRASPASO_SALIDA', 'TRASPASO_ENTRADA', 'TRASPASO_RECHAZO', 'MERMA', 'AJUSTE_ENTRADA', 'AJUSTE_SALIDA')),
  referencia_tipo TEXT NOT NULL,
  referencia_id TEXT NOT NULL,
  cantidad NUMERIC(12, 3) NOT NULL,
  costo_unitario NUMERIC(12, 4) NULL,
  usuario_id TEXT NULL REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT,
  fecha TIMESTAMPTZ NOT NULL,
  sincronizado BOOLEAN NOT NULL DEFAULT TRUE,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE IF EXISTS sucursales ADD COLUMN IF NOT EXISTS eliminado BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE IF EXISTS usuarios ADD COLUMN IF NOT EXISTS eliminado BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE IF EXISTS proveedores ADD COLUMN IF NOT EXISTS eliminado BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE IF EXISTS productos ADD COLUMN IF NOT EXISTS eliminado BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE IF EXISTS clientes ADD COLUMN IF NOT EXISTS eliminado BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE IF EXISTS marcas ADD COLUMN IF NOT EXISTS eliminado BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE IF EXISTS unidades ADD COLUMN IF NOT EXISTS eliminado BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE IF EXISTS productos ADD COLUMN IF NOT EXISTS costo_promedio NUMERIC(12, 4) NOT NULL DEFAULT 0;
ALTER TABLE IF EXISTS inventario_sucursal ADD COLUMN IF NOT EXISTS costo_promedio NUMERIC(12, 4) NOT NULL DEFAULT 0;
ALTER TABLE IF EXISTS inventario_sucursal ADD COLUMN IF NOT EXISTS precio_venta NUMERIC(12, 2) NOT NULL DEFAULT 0;
ALTER TABLE IF EXISTS inventario_sucursal ADD COLUMN IF NOT EXISTS eliminado BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE IF EXISTS detalle_compras ADD COLUMN IF NOT EXISTS costo_promedio_resultante NUMERIC(12, 4) NULL;
ALTER TABLE IF EXISTS detalle_ventas ADD COLUMN IF NOT EXISTS costo_unitario_pactado NUMERIC(12, 4) NOT NULL DEFAULT 0;
ALTER TABLE IF EXISTS traspasos ADD COLUMN IF NOT EXISTS estado TEXT NOT NULL DEFAULT 'EN_TRANSITO';
ALTER TABLE IF EXISTS traspasos ADD COLUMN IF NOT EXISTS usuario_recibio_id TEXT NULL REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE IF EXISTS traspasos ADD COLUMN IF NOT EXISTS fecha_recepcion TIMESTAMPTZ NULL;
ALTER TABLE IF EXISTS traspasos ADD COLUMN IF NOT EXISTS observaciones_recepcion TEXT NULL;

ALTER TABLE IF EXISTS productos ALTER COLUMN proveedor_id DROP DEFAULT;

CREATE INDEX IF NOT EXISTS idx_productos_descripcion ON productos (descripcion);
CREATE INDEX IF NOT EXISTS idx_marcas_nombre ON marcas (nombre);
CREATE INDEX IF NOT EXISTS idx_unidades_nombre ON unidades (nombre);
CREATE INDEX IF NOT EXISTS idx_sucursales_eliminado ON sucursales (eliminado);
CREATE INDEX IF NOT EXISTS idx_usuarios_eliminado ON usuarios (eliminado);
CREATE INDEX IF NOT EXISTS idx_proveedores_eliminado ON proveedores (eliminado);
CREATE INDEX IF NOT EXISTS idx_productos_eliminado ON productos (eliminado);
CREATE INDEX IF NOT EXISTS idx_clientes_eliminado ON clientes (eliminado);
CREATE INDEX IF NOT EXISTS idx_productos_codigo_barras ON productos (codigo_barras);
CREATE INDEX IF NOT EXISTS idx_productos_codigo_proveedor ON productos (codigo_proveedor);
CREATE INDEX IF NOT EXISTS idx_inventario_sucursal_id ON inventario_sucursal (sucursal_id);
CREATE INDEX IF NOT EXISTS idx_inventario_sucursal_eliminado ON inventario_sucursal (eliminado);
CREATE INDEX IF NOT EXISTS idx_ventas_sucursal_fecha ON ventas (sucursal_id, fecha);
CREATE INDEX IF NOT EXISTS idx_ventas_updated_at ON ventas (updated_at);
CREATE INDEX IF NOT EXISTS idx_detalle_ventas_venta_uuid ON detalle_ventas (venta_uuid);
CREATE INDEX IF NOT EXISTS idx_clientes_nombre ON clientes (nombre);
CREATE INDEX IF NOT EXISTS idx_clientes_updated_at ON clientes (updated_at);
CREATE INDEX IF NOT EXISTS idx_abonos_cliente_fecha ON creditos_abonos (cliente_id, fecha);
CREATE INDEX IF NOT EXISTS idx_cajas_sesiones_usuario_estado ON cajas_sesiones (usuario_id, sucursal_id, estado);
CREATE INDEX IF NOT EXISTS idx_caja_movimientos_sesion_uuid ON caja_movimientos (sesion_uuid);
CREATE INDEX IF NOT EXISTS idx_traspasos_fecha ON traspasos (fecha);
CREATE INDEX IF NOT EXISTS idx_mermas_sucursal ON mermas_ajustes (sucursal_id);
CREATE INDEX IF NOT EXISTS idx_movimientos_inventario_producto_sucursal ON movimientos_inventario (producto_id, sucursal_id, fecha);
CREATE INDEX IF NOT EXISTS idx_movimientos_inventario_referencia ON movimientos_inventario (referencia_tipo, referencia_id);
CREATE INDEX IF NOT EXISTS idx_facturas_emitidas_estado_fecha ON facturas_emitidas (estado, fecha_emision);

SELECT touch_updated_at('sucursales');
SELECT touch_updated_at('empresa_config_fiscal');
SELECT touch_updated_at('proveedores');
SELECT touch_updated_at('marcas');
SELECT touch_updated_at('unidades');
SELECT touch_updated_at('usuarios');
SELECT touch_updated_at('productos');
SELECT touch_updated_at('inventario_sucursal');
SELECT touch_updated_at('clientes');
SELECT touch_updated_at('clientes_datos_fiscales');
SELECT touch_updated_at('compras');
SELECT touch_updated_at('detalle_compras');
SELECT touch_updated_at('ventas');
SELECT touch_updated_at('detalle_ventas');
SELECT touch_updated_at('creditos_abonos');
SELECT touch_updated_at('cajas_sesiones');
SELECT touch_updated_at('caja_movimientos');
SELECT touch_updated_at('traspasos');
SELECT touch_updated_at('detalle_traspasos');
SELECT touch_updated_at('mermas_ajustes');
SELECT touch_updated_at('movimientos_inventario');
SELECT touch_updated_at('facturas_emitidas');

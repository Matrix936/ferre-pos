-- Importador LOCAL (SQLite) de sistema legacy -> Ferre-POS
-- Fecha: 2026-05-31
--
-- Flujo:
-- 1) Cargar datos legacy en tablas staging: legacy_articulos y legacy_ventas.
-- 2) Ajustar variables "target_sucursal_id" y "target_usuario_id" en el bloque CONFIG.
-- 3) Ejecutar este script en la SQLite local de Ferre-POS.

PRAGMA foreign_keys = ON;
BEGIN TRANSACTION;

-- =========================
-- STAGING TABLES (si no existen)
-- =========================
CREATE TABLE IF NOT EXISTS legacy_articulos (
  id INTEGER,
  clave TEXT,
  descripcion_articulo TEXT NOT NULL,
  unidad TEXT NOT NULL,
  codigo_barra TEXT,
  existencia_stock REAL,
  caducidad TEXT,
  provedor INTEGER,
  categoria TEXT,
  marca INTEGER,
  fotos TEXT,
  descripcion_catalogo TEXT,
  precio_compra REAL,
  precio_venta REAL,
  precio_1 REAL,
  precio_2 REAL,
  precio_3 REAL,
  precio_4 REAL,
  mayoreo_apartir REAL,
  cant_min_stock REAL,
  a_granel TEXT,
  no_en_catalogo TEXT,
  ventas_negativas TEXT,
  created_at TEXT,
  updated_at TEXT
);

CREATE TABLE IF NOT EXISTS legacy_ventas (
  id INTEGER,
  contador INTEGER,
  usuario TEXT NOT NULL,
  id_turno TEXT NOT NULL,
  caja TEXT NOT NULL,
  efectivo TEXT,
  tipo_pago TEXT NOT NULL,
  comision REAL,
  total_venta REAL NOT NULL,
  cantidad_pro TEXT NOT NULL,
  id_productos TEXT NOT NULL,
  precio_vendido TEXT NOT NULL,
  tipo_precio_vendido TEXT NOT NULL,
  nombre TEXT,
  telefono TEXT,
  domicilio TEXT,
  facturar TEXT,
  created_at TEXT,
  updated_at TEXT
);

-- =========================
-- CONFIG (AJUSTAR)
-- =========================
-- Cambia estos valores antes de correr:
-- SUC-001: id real de sucursal destino
-- ADMIN-001: id real de usuario que "firmara" ventas legacy
CREATE TEMP TABLE IF NOT EXISTS _legacy_cfg (
  target_sucursal_id TEXT NOT NULL,
  target_usuario_id TEXT NOT NULL
);
DELETE FROM _legacy_cfg;
INSERT INTO _legacy_cfg(target_sucursal_id, target_usuario_id)
VALUES ('SUC-001', 'ADMIN-001');

-- =========================
-- VALIDACIONES
-- =========================
SELECT
  CASE
    WHEN NOT EXISTS (SELECT 1 FROM sucursales s JOIN _legacy_cfg c ON c.target_sucursal_id = s.id)
    THEN RAISE(ABORT, 'No existe target_sucursal_id en sucursales. Ajusta _legacy_cfg.')
    ELSE 1
  END;

SELECT
  CASE
    WHEN NOT EXISTS (SELECT 1 FROM usuarios u JOIN _legacy_cfg c ON c.target_usuario_id = u.id)
    THEN RAISE(ABORT, 'No existe target_usuario_id en usuarios. Ajusta _legacy_cfg.')
    ELSE 1
  END;

-- =========================
-- PROVEEDORES / MARCAS / CATEGORIAS / UNIDADES
-- =========================
INSERT INTO proveedores (id, nombre, contacto_nombre, telefono, email, direccion)
VALUES ('PROV-SIN-ASIGNAR', 'PROVEEDOR SIN ASIGNAR', '', '', '', '')
ON CONFLICT(id) DO UPDATE SET nombre = excluded.nombre;

INSERT INTO proveedores (id, nombre, contacto_nombre, telefono, email, direccion)
SELECT DISTINCT
  'LEGACY-PROV-' || provedor,
  'PROVEEDOR LEGACY #' || provedor,
  '', '', '', ''
FROM legacy_articulos
WHERE provedor IS NOT NULL
ON CONFLICT(id) DO UPDATE SET nombre = excluded.nombre;

INSERT INTO marcas (id, nombre)
SELECT DISTINCT
  'LEGACY-MARCA-' || marca,
  'MARCA LEGACY #' || marca
FROM legacy_articulos
WHERE marca IS NOT NULL
ON CONFLICT(id) DO UPDATE SET nombre = excluded.nombre;

INSERT INTO categorias (id, nombre)
SELECT DISTINCT
  'LEGACY-CAT-' || hex(upper(trim(coalesce(categoria, 'GENERAL')))),
  upper(trim(coalesce(categoria, 'GENERAL')))
FROM legacy_articulos
WHERE trim(coalesce(categoria, '')) <> ''
ON CONFLICT(id) DO UPDATE SET nombre = excluded.nombre;

INSERT INTO unidades (id, nombre, clave_sat)
SELECT DISTINCT
  'LEGACY-UNI-' || hex(upper(trim(coalesce(unidad, 'PIEZA')))),
  upper(trim(coalesce(unidad, 'PIEZA'))),
  'H87'
FROM legacy_articulos
WHERE trim(coalesce(unidad, '')) <> ''
ON CONFLICT(id) DO UPDATE SET
  nombre = excluded.nombre,
  clave_sat = CASE WHEN trim(excluded.clave_sat) <> '' THEN excluded.clave_sat ELSE unidades.clave_sat END;

-- =========================
-- PRODUCTOS
-- =========================
INSERT INTO productos (
  id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
  marca, categoria, unidad, precio_costo, costo_promedio, precio_venta,
  sat_clave_prod_serv, sat_clave_unidad
)
SELECT
  'LEGACY-ART-' || la.id,
  NULLIF(trim(coalesce(la.codigo_barra, '')), ''),
  coalesce(NULLIF(trim(coalesce(la.clave, '')), ''), 'LEGACY-COD-' || la.id),
  CASE WHEN la.provedor IS NOT NULL THEN 'LEGACY-PROV-' || la.provedor ELSE 'PROV-SIN-ASIGNAR' END,
  coalesce(NULLIF(trim(coalesce(la.clave, '')), ''), 'LEGACY-CLAVE-' || la.id),
  trim(la.descripcion_articulo),
  CASE WHEN la.marca IS NOT NULL THEN 'MARCA LEGACY #' || la.marca ELSE 'SIN MARCA' END,
  coalesce(NULLIF(upper(trim(coalesce(la.categoria, ''))), ''), 'GENERAL'),
  coalesce(NULLIF(upper(trim(coalesce(la.unidad, ''))), ''), 'PIEZA'),
  round(coalesce(la.precio_compra, 0), 2),
  round(coalesce(la.precio_compra, 0), 4),
  round(coalesce(NULLIF(la.precio_venta, 0), la.precio_1, la.precio_2, la.precio_3, la.precio_4, 0), 2),
  '01010101',
  'H87'
FROM legacy_articulos la
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
  precio_venta = excluded.precio_venta;

-- =========================
-- INVENTARIO SUCURSAL (destino fijo)
-- =========================
INSERT INTO inventario_sucursal (
  producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta
)
SELECT
  'LEGACY-ART-' || la.id,
  c.target_sucursal_id,
  round(coalesce(la.existencia_stock, 0), 3),
  round(coalesce(la.cant_min_stock, 0), 3),
  round(coalesce(la.precio_compra, 0), 4),
  round(coalesce(NULLIF(la.precio_venta, 0), la.precio_1, la.precio_2, la.precio_3, la.precio_4, 0), 2)
FROM legacy_articulos la
CROSS JOIN _legacy_cfg c
ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
  stock = excluded.stock,
  stock_minimo = excluded.stock_minimo,
  costo_promedio = excluded.costo_promedio,
  precio_venta = excluded.precio_venta;

-- =========================
-- CLIENTES LEGACY MINIMOS
-- =========================
INSERT INTO clientes (id, nombre, telefono, direccion, limite_credito, saldo_deudor)
SELECT DISTINCT
  'LEGACY-CLI-VTA-' || lv.id,
  substr(trim(coalesce(lv.nombre, 'CLIENTE LEGACY')), 1, 255),
  substr(trim(coalesce(lv.telefono, '')), 1, 40),
  substr(trim(coalesce(lv.domicilio, '')), 1, 500),
  0,
  0
FROM legacy_ventas lv
WHERE trim(coalesce(lv.nombre, '')) <> ''
ON CONFLICT(id) DO UPDATE SET
  nombre = excluded.nombre,
  telefono = excluded.telefono,
  direccion = excluded.direccion;

-- =========================
-- VENTAS (ENCABEZADO)
-- =========================
INSERT INTO ventas (
  id, usuario_id, sucursal_id, fecha, total, metodo_pago,
  efectivo_recibido, cambio_entregado, cliente_id, estado
)
SELECT
  'LEGACY-VTA-' || lv.id,
  c.target_usuario_id,
  c.target_sucursal_id,
  coalesce(lv.created_at, lv.updated_at, datetime('now')),
  round(coalesce(lv.total_venta, 0), 2),
  CASE
    WHEN upper(coalesce(lv.tipo_pago, '')) LIKE '%EFEC%' THEN 'EFECTIVO'
    WHEN upper(coalesce(lv.tipo_pago, '')) LIKE '%TARJ%' THEN 'TARJETA'
    WHEN upper(coalesce(lv.tipo_pago, '')) LIKE '%TRANS%' THEN 'TRANSFERENCIA'
    WHEN upper(coalesce(lv.tipo_pago, '')) LIKE '%CRED%' THEN 'CREDITO'
    ELSE 'EFECTIVO'
  END,
  CASE
    WHEN upper(coalesce(lv.tipo_pago, '')) LIKE '%EFEC%'
      THEN round(coalesce(NULLIF(replace(replace(replace(trim(coalesce(lv.efectivo, '')), '$', ''), ',', ''), ' ', ''), ''), 0), 2)
    ELSE NULL
  END,
  CASE
    WHEN upper(coalesce(lv.tipo_pago, '')) LIKE '%EFEC%' THEN
      max(
        round(coalesce(NULLIF(replace(replace(replace(trim(coalesce(lv.efectivo, '')), '$', ''), ',', ''), ' ', ''), ''), 0), 2)
        - round(coalesce(lv.total_venta, 0), 2),
        0
      )
    ELSE NULL
  END,
  CASE
    WHEN trim(coalesce(lv.nombre, '')) <> '' THEN 'LEGACY-CLI-VTA-' || lv.id
    ELSE NULL
  END,
  'COMPLETADA'
FROM legacy_ventas lv
CROSS JOIN _legacy_cfg c
ON CONFLICT(id) DO UPDATE SET
  usuario_id = excluded.usuario_id,
  sucursal_id = excluded.sucursal_id,
  fecha = excluded.fecha,
  total = excluded.total,
  metodo_pago = excluded.metodo_pago,
  efectivo_recibido = excluded.efectivo_recibido,
  cambio_entregado = excluded.cambio_entregado,
  cliente_id = excluded.cliente_id;

-- =========================
-- DETALLE VENTAS
-- Split de listas (id_productos, cantidad_pro, precio_vendido) por indice
-- =========================
WITH RECURSIVE
ventas_src AS (
  SELECT
    lv.id AS legacy_id,
    'LEGACY-VTA-' || lv.id AS venta_id,
    replace(replace(replace(coalesce(lv.id_productos, ''), ';', ','), '|', ','), ' ', '') AS ids_csv,
    replace(replace(replace(coalesce(lv.cantidad_pro, ''), ';', ','), '|', ','), ' ', '') AS qty_csv,
    replace(replace(replace(coalesce(lv.precio_vendido, ''), ';', ','), '|', ','), ' ', '') AS price_csv
  FROM legacy_ventas lv
),
prod_split(legacy_id, venta_id, idx, value, rest) AS (
  SELECT
    legacy_id,
    venta_id,
    1,
    trim(substr(ids_csv || ',', 1, instr(ids_csv || ',', ',') - 1)),
    substr(ids_csv || ',', instr(ids_csv || ',', ',') + 1)
  FROM ventas_src
  UNION ALL
  SELECT
    legacy_id,
    venta_id,
    idx + 1,
    trim(substr(rest, 1, instr(rest, ',') - 1)),
    substr(rest, instr(rest, ',') + 1)
  FROM prod_split
  WHERE rest <> ''
),
qty_split(legacy_id, venta_id, idx, value, rest) AS (
  SELECT
    legacy_id,
    venta_id,
    1,
    trim(substr(qty_csv || ',', 1, instr(qty_csv || ',', ',') - 1)),
    substr(qty_csv || ',', instr(qty_csv || ',', ',') + 1)
  FROM ventas_src
  UNION ALL
  SELECT
    legacy_id,
    venta_id,
    idx + 1,
    trim(substr(rest, 1, instr(rest, ',') - 1)),
    substr(rest, instr(rest, ',') + 1)
  FROM qty_split
  WHERE rest <> ''
),
price_split(legacy_id, venta_id, idx, value, rest) AS (
  SELECT
    legacy_id,
    venta_id,
    1,
    trim(substr(price_csv || ',', 1, instr(price_csv || ',', ',') - 1)),
    substr(price_csv || ',', instr(price_csv || ',', ',') + 1)
  FROM ventas_src
  UNION ALL
  SELECT
    legacy_id,
    venta_id,
    idx + 1,
    trim(substr(rest, 1, instr(rest, ',') - 1)),
    substr(rest, instr(rest, ',') + 1)
  FROM price_split
  WHERE rest <> ''
),
merged AS (
  SELECT
    p.legacy_id,
    p.venta_id,
    p.idx,
    p.value AS legacy_producto_id,
    coalesce(q.value, '1') AS qty_txt,
    coalesce(pr.value, '0') AS price_txt
  FROM prod_split p
  LEFT JOIN qty_split q
    ON q.legacy_id = p.legacy_id AND q.idx = p.idx
  LEFT JOIN price_split pr
    ON pr.legacy_id = p.legacy_id AND pr.idx = p.idx
  WHERE trim(coalesce(p.value, '')) <> ''
),
items_ok AS (
  SELECT
    m.legacy_id,
    m.venta_id,
    m.idx,
    'LEGACY-ART-' || cast(m.legacy_producto_id as integer) AS producto_id,
    CASE
      WHEN cast(replace(m.qty_txt, ',', '.') as real) > 0
      THEN round(cast(replace(m.qty_txt, ',', '.') as real), 3)
      ELSE 1.0
    END AS cantidad,
    round(coalesce(cast(replace(m.price_txt, ',', '.') as real), 0), 2) AS precio_venta
  FROM merged m
),
items_valid AS (
  SELECT
    i.legacy_id,
    i.venta_id,
    i.idx,
    i.producto_id,
    i.cantidad,
    i.precio_venta,
    coalesce(NULLIF(p.costo_promedio, 0), p.precio_costo, 0) AS costo_unitario
  FROM items_ok i
  INNER JOIN productos p ON p.id = i.producto_id
)
INSERT INTO detalle_ventas (
  id, venta_id, producto_id, cantidad, precio_venta_pactado, costo_unitario_pactado
)
SELECT
  'LEGACY-DV-' || iv.legacy_id || '-' || iv.idx,
  iv.venta_id,
  iv.producto_id,
  iv.cantidad,
  iv.precio_venta,
  iv.costo_unitario
FROM items_valid iv
ON CONFLICT(id) DO UPDATE SET
  venta_id = excluded.venta_id,
  producto_id = excluded.producto_id,
  cantidad = excluded.cantidad,
  precio_venta_pactado = excluded.precio_venta_pactado,
  costo_unitario_pactado = excluded.costo_unitario_pactado;

COMMIT;

-- =========================
-- VERIFICACION RAPIDA
-- =========================
-- SELECT COUNT(*) AS legacy_articulos FROM legacy_articulos;
-- SELECT COUNT(*) AS legacy_ventas FROM legacy_ventas;
-- SELECT COUNT(*) AS productos_importados FROM productos WHERE id LIKE 'LEGACY-ART-%';
-- SELECT COUNT(*) AS inventario_importado FROM inventario_sucursal WHERE producto_id LIKE 'LEGACY-ART-%';
-- SELECT COUNT(*) AS ventas_importadas FROM ventas WHERE id LIKE 'LEGACY-VTA-%';
-- SELECT COUNT(*) AS detalles_importados FROM detalle_ventas WHERE id LIKE 'LEGACY-DV-%';

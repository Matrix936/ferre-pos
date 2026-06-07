-- Importador ARTICULOS -> modelo actual Ferre-POS (SOLO catalogo + inventario)
-- Fecha: 2026-05-31
--
-- Objetivo:
-- 1) Pasar datos generales a productos (catalogo maestro).
-- 2) Pasar stock/costo/precio/minimo a inventario_sucursal.
-- 3) Conservar campos legacy no modelados en una tabla complemento.
--
-- Importante:
-- - NO toca ventas legacy.
-- - NO cambia estructura principal de productos/inventario.
-- - Solo crea tabla complemento "productos_legacy_meta" para no perder datos.
--
-- Uso:
-- A) Carga tu CSV en una tabla temporal de trabajo llamada import_articulos
--    con columnas compatibles con la tabla legacy "articulos".
-- B) Ajusta target_sucursal_id y target_proveedor_default en CONFIG.
-- C) Ejecuta este script completo.

PRAGMA foreign_keys = ON;
BEGIN TRANSACTION;

-- =========================
-- CONFIG
-- =========================
CREATE TEMP TABLE IF NOT EXISTS _cfg_import_articulos (
  target_sucursal_id TEXT NOT NULL,
  target_proveedor_default TEXT NOT NULL
);
DELETE FROM _cfg_import_articulos;
INSERT INTO _cfg_import_articulos(target_sucursal_id, target_proveedor_default)
VALUES ('SUC-001', 'PROV-SIN-ASIGNAR');

-- =========================
-- VALIDACIONES
-- =========================
SELECT
  CASE
    WHEN NOT EXISTS (
      SELECT 1
      FROM sucursales s
      JOIN _cfg_import_articulos c ON c.target_sucursal_id = s.id
    )
    THEN RAISE(ABORT, 'target_sucursal_id no existe. Ajusta _cfg_import_articulos.')
    ELSE 1
  END;

-- Tabla staging obligatoria: import_articulos
-- Debe existir antes de correr este script.
SELECT
  CASE
    WHEN NOT EXISTS (
      SELECT 1
      FROM sqlite_master
      WHERE type = 'table' AND name = 'import_articulos'
    )
    THEN RAISE(ABORT, 'No existe import_articulos. Primero importa tu CSV a esa tabla.')
    ELSE 1
  END;

-- =========================
-- TABLA COMPLEMENTO LEGACY
-- =========================
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
  FOREIGN KEY (producto_id) REFERENCES productos(id) ON UPDATE CASCADE ON DELETE CASCADE
);

-- =========================
-- CATALOGOS BASE
-- =========================
INSERT INTO proveedores (id, nombre, contacto_nombre, telefono, email, direccion)
SELECT c.target_proveedor_default, 'PROVEEDOR SIN ASIGNAR', '', '', '', ''
FROM _cfg_import_articulos c
ON CONFLICT(id) DO UPDATE SET nombre = excluded.nombre;

INSERT INTO proveedores (id, nombre, contacto_nombre, telefono, email, direccion)
SELECT DISTINCT
  'LEGACY-PROV-' || ia.provedor,
  'PROVEEDOR LEGACY #' || ia.provedor,
  '', '', '', ''
FROM import_articulos ia
WHERE ia.provedor IS NOT NULL
ON CONFLICT(id) DO UPDATE SET nombre = excluded.nombre;

INSERT INTO marcas (id, nombre)
SELECT DISTINCT
  'LEGACY-MARCA-' || ia.marca,
  'MARCA LEGACY #' || ia.marca
FROM import_articulos ia
WHERE ia.marca IS NOT NULL
ON CONFLICT(id) DO UPDATE SET nombre = excluded.nombre;

INSERT INTO categorias (id, nombre)
SELECT DISTINCT
  'LEGACY-CAT-' || hex(upper(trim(coalesce(ia.categoria, 'GENERAL')))),
  upper(trim(coalesce(ia.categoria, 'GENERAL')))
FROM import_articulos ia
WHERE trim(coalesce(ia.categoria, '')) <> ''
ON CONFLICT(id) DO UPDATE SET nombre = excluded.nombre;

INSERT INTO unidades (id, nombre, clave_sat)
SELECT DISTINCT
  'LEGACY-UNI-' || hex(upper(trim(coalesce(ia.unidad, 'PIEZA')))),
  upper(trim(coalesce(ia.unidad, 'PIEZA'))),
  'H87'
FROM import_articulos ia
WHERE trim(coalesce(ia.unidad, '')) <> ''
ON CONFLICT(id) DO UPDATE SET
  nombre = excluded.nombre,
  clave_sat = CASE
    WHEN trim(coalesce(excluded.clave_sat, '')) <> '' THEN excluded.clave_sat
    ELSE unidades.clave_sat
  END;

-- =========================
-- PRODUCTOS (catalogo maestro)
-- =========================
INSERT INTO productos (
  id, codigo_barras, codigo_proveedor, proveedor_id, clave_producto, descripcion,
  marca, categoria, unidad, precio_costo, costo_promedio, precio_venta,
  sat_clave_prod_serv, sat_clave_unidad
)
SELECT
  'LEGACY-ART-' || ia.id,
  NULLIF(trim(coalesce(ia.codigo_barra, '')), ''),
  coalesce(NULLIF(trim(coalesce(ia.clave, '')), ''), 'LEGACY-COD-' || ia.id),
  CASE
    WHEN ia.provedor IS NOT NULL THEN 'LEGACY-PROV-' || ia.provedor
    ELSE c.target_proveedor_default
  END,
  coalesce(NULLIF(trim(coalesce(ia.clave, '')), ''), 'LEGACY-CLAVE-' || ia.id),
  trim(ia.descripcion_articulo),
  CASE
    WHEN ia.marca IS NOT NULL THEN 'MARCA LEGACY #' || ia.marca
    ELSE 'SIN MARCA'
  END,
  coalesce(NULLIF(upper(trim(coalesce(ia.categoria, ''))), ''), 'GENERAL'),
  coalesce(NULLIF(upper(trim(coalesce(ia.unidad, ''))), ''), 'PIEZA'),
  round(coalesce(ia.precio_compra, 0), 2),
  round(coalesce(ia.precio_compra, 0), 4),
  round(coalesce(NULLIF(ia.precio_venta, 0), ia.precio_1, ia.precio_2, ia.precio_3, ia.precio_4, 0), 2),
  '01010101',
  'H87'
FROM import_articulos ia
CROSS JOIN _cfg_import_articulos c
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
-- INVENTARIO POR SUCURSAL
-- =========================
INSERT INTO inventario_sucursal (
  producto_id, sucursal_id, stock, stock_minimo, costo_promedio, precio_venta
)
SELECT
  'LEGACY-ART-' || ia.id,
  c.target_sucursal_id,
  round(coalesce(ia.existencia_stock, 0), 3),
  round(coalesce(ia.cant_min_stock, 0), 3),
  round(coalesce(ia.precio_compra, 0), 4),
  round(coalesce(NULLIF(ia.precio_venta, 0), ia.precio_1, ia.precio_2, ia.precio_3, ia.precio_4, 0), 2)
FROM import_articulos ia
CROSS JOIN _cfg_import_articulos c
ON CONFLICT(producto_id, sucursal_id) DO UPDATE SET
  stock = excluded.stock,
  stock_minimo = excluded.stock_minimo,
  costo_promedio = excluded.costo_promedio,
  precio_venta = excluded.precio_venta;

-- =========================
-- CAMPOS LEGACY EXTRA
-- =========================
INSERT INTO productos_legacy_meta (
  producto_id, legacy_id, caducidad, fotos, descripcion_catalogo, mayoreo_apartir,
  a_granel, no_en_catalogo, ventas_negativas, created_at_legacy, updated_at_legacy
)
SELECT
  'LEGACY-ART-' || ia.id,
  ia.id,
  ia.caducidad,
  ia.fotos,
  ia.descripcion_catalogo,
  ia.mayoreo_apartir,
  ia.a_granel,
  ia.no_en_catalogo,
  ia.ventas_negativas,
  ia.created_at,
  ia.updated_at
FROM import_articulos ia
ON CONFLICT(producto_id) DO UPDATE SET
  caducidad = excluded.caducidad,
  fotos = excluded.fotos,
  descripcion_catalogo = excluded.descripcion_catalogo,
  mayoreo_apartir = excluded.mayoreo_apartir,
  a_granel = excluded.a_granel,
  no_en_catalogo = excluded.no_en_catalogo,
  ventas_negativas = excluded.ventas_negativas,
  created_at_legacy = excluded.created_at_legacy,
  updated_at_legacy = excluded.updated_at_legacy;

COMMIT;

-- =========================
-- VERIFICACION RAPIDA
-- =========================
-- SELECT COUNT(*) AS import_rows FROM import_articulos;
-- SELECT COUNT(*) AS productos_legacy FROM productos WHERE id LIKE 'LEGACY-ART-%';
-- SELECT COUNT(*) AS inventario_legacy FROM inventario_sucursal WHERE producto_id LIKE 'LEGACY-ART-%';
-- SELECT COUNT(*) AS meta_legacy FROM productos_legacy_meta WHERE producto_id LIKE 'LEGACY-ART-%';

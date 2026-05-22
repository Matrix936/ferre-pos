-- Ferre-POS - Separación catálogo producto / datos por sucursal.

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

ALTER TABLE IF EXISTS inventario_sucursal
  ADD COLUMN IF NOT EXISTS precio_venta NUMERIC(12, 2) NOT NULL DEFAULT 0;

UPDATE inventario_sucursal i
SET precio_venta = COALESCE(p.precio_venta, 0)
FROM productos p
WHERE p.id = i.producto_id
  AND i.precio_venta = 0;

INSERT INTO marcas (id, nombre)
SELECT 'MARCA-' || regexp_replace(upper(trim(marca)), '[^A-Z0-9]+', '-', 'g'), trim(marca)
FROM productos
WHERE trim(marca) <> ''
ON CONFLICT (id) DO NOTHING;

INSERT INTO unidades (id, nombre, clave_sat)
SELECT 'UNIDAD-' || regexp_replace(upper(trim(unidad)), '[^A-Z0-9]+', '-', 'g'), trim(unidad), max(trim(sat_clave_unidad))
FROM productos
WHERE trim(unidad) <> ''
GROUP BY trim(unidad)
ON CONFLICT (id) DO NOTHING;

CREATE INDEX IF NOT EXISTS idx_marcas_nombre ON marcas (nombre);
CREATE INDEX IF NOT EXISTS idx_unidades_nombre ON unidades (nombre);

SELECT touch_updated_at('marcas');
SELECT touch_updated_at('unidades');

-- Ferre-POS - Inventario empresarial: CPP, kardex y traspasos por recepción.
-- Ejecutar en Supabase SQL Editor sobre una base existente.

ALTER TABLE IF EXISTS productos
  ADD COLUMN IF NOT EXISTS costo_promedio NUMERIC(12, 4) NOT NULL DEFAULT 0;

UPDATE productos
SET costo_promedio = precio_costo
WHERE costo_promedio = 0 AND precio_costo > 0;

ALTER TABLE IF EXISTS inventario_sucursal
  ADD COLUMN IF NOT EXISTS costo_promedio NUMERIC(12, 4) NOT NULL DEFAULT 0;

UPDATE inventario_sucursal i
SET costo_promedio = COALESCE(NULLIF(p.costo_promedio, 0), p.precio_costo, 0)
FROM productos p
WHERE p.id = i.producto_id
  AND i.costo_promedio = 0;

ALTER TABLE IF EXISTS detalle_compras
  ADD COLUMN IF NOT EXISTS costo_promedio_resultante NUMERIC(12, 4) NULL;

ALTER TABLE IF EXISTS detalle_ventas
  ADD COLUMN IF NOT EXISTS costo_unitario_pactado NUMERIC(12, 4) NOT NULL DEFAULT 0;

UPDATE detalle_ventas dv
SET costo_unitario_pactado = COALESCE(NULLIF(p.costo_promedio, 0), p.precio_costo, 0)
FROM productos p
WHERE p.id = dv.producto_id
  AND dv.costo_unitario_pactado = 0;

ALTER TABLE IF EXISTS traspasos
  ADD COLUMN IF NOT EXISTS estado TEXT NOT NULL DEFAULT 'EN_TRANSITO';
ALTER TABLE IF EXISTS traspasos
  ADD COLUMN IF NOT EXISTS usuario_recibio_id TEXT NULL REFERENCES usuarios(id) ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE IF EXISTS traspasos
  ADD COLUMN IF NOT EXISTS fecha_recepcion TIMESTAMPTZ NULL;
ALTER TABLE IF EXISTS traspasos
  ADD COLUMN IF NOT EXISTS observaciones_recepcion TEXT NULL;

UPDATE traspasos
SET estado = 'RECIBIDO',
    fecha_recepcion = COALESCE(fecha_recepcion, fecha)
WHERE estado IS NULL OR estado = '';

DO $$
DECLARE
  constraint_name text;
BEGIN
  FOR constraint_name IN
    SELECT con.conname
    FROM pg_constraint con
    JOIN pg_class rel ON rel.oid = con.conrelid
    JOIN pg_namespace nsp ON nsp.oid = rel.relnamespace
    WHERE nsp.nspname = 'public'
      AND rel.relname = 'traspasos'
      AND con.contype = 'c'
      AND pg_get_constraintdef(con.oid) LIKE '%estado%'
  LOOP
    EXECUTE format('ALTER TABLE public.traspasos DROP CONSTRAINT %I', constraint_name);
  END LOOP;
END $$;

ALTER TABLE traspasos
  ADD CONSTRAINT traspasos_estado_check
  CHECK (estado IN ('EN_TRANSITO', 'RECIBIDO', 'RECHAZADO', 'CANCELADO'));

DO $$
DECLARE
  constraint_name text;
BEGIN
  FOR constraint_name IN
    SELECT con.conname
    FROM pg_constraint con
    JOIN pg_class rel ON rel.oid = con.conrelid
    JOIN pg_namespace nsp ON nsp.oid = rel.relnamespace
    WHERE nsp.nspname = 'public'
      AND rel.relname = 'mermas_ajustes'
      AND con.contype = 'c'
      AND pg_get_constraintdef(con.oid) LIKE '%tipo_movimiento%'
  LOOP
    EXECUTE format('ALTER TABLE public.mermas_ajustes DROP CONSTRAINT %I', constraint_name);
  END LOOP;
END $$;

ALTER TABLE mermas_ajustes
  ADD CONSTRAINT mermas_ajustes_tipo_movimiento_check
  CHECK (tipo_movimiento IN ('MERMA', 'AJUSTE', 'AJUSTE_ENTRADA', 'AJUSTE_SALIDA'));

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

CREATE INDEX IF NOT EXISTS idx_movimientos_inventario_producto_sucursal
  ON movimientos_inventario (producto_id, sucursal_id, fecha);

CREATE INDEX IF NOT EXISTS idx_movimientos_inventario_referencia
  ON movimientos_inventario (referencia_tipo, referencia_id);

SELECT touch_updated_at('movimientos_inventario');

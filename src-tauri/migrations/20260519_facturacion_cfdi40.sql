PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS empresa_config_fiscal (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    rfc TEXT NOT NULL DEFAULT '',
    razon_social TEXT NOT NULL DEFAULT '',
    regimen_fiscal TEXT NOT NULL DEFAULT '',
    registro_patronal TEXT NULL,
    actualizado_at TEXT NOT NULL
);

ALTER TABLE sucursales ADD COLUMN codigo_postal TEXT NOT NULL DEFAULT '';

CREATE TABLE IF NOT EXISTS clientes_datos_fiscales (
    cliente_id TEXT PRIMARY KEY,
    rfc TEXT NOT NULL UNIQUE,
    razon_social TEXT NOT NULL,
    regimen_fiscal TEXT NOT NULL,
    codigo_postal TEXT NOT NULL,
    FOREIGN KEY (cliente_id) REFERENCES clientes(id) ON UPDATE CASCADE ON DELETE CASCADE
);

ALTER TABLE productos ADD COLUMN sat_clave_prod_serv TEXT NOT NULL DEFAULT '';
ALTER TABLE productos ADD COLUMN sat_clave_unidad TEXT NOT NULL DEFAULT '';

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

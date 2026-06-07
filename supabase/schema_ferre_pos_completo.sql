-- Ferre-POS - Esquema oficial completo Supabase/PostgreSQL
-- Uso recomendado: proyectos nuevos o bases reseteadas antes de conectar sucursales.
-- No ejecuta DROP/TRUNCATE. Si ya existen tablas con tipos distintos, revisa antes de aplicar.

create extension if not exists pgcrypto;

create or replace function public.set_updated_at()
returns trigger as $$
begin
  new.updated_at = now();
  return new;
end;
$$ language plpgsql;

create or replace function public.touch_updated_at(table_name text)
returns void as $$
begin
  execute format('drop trigger if exists trg_%I_updated_at on public.%I', table_name, table_name);
  execute format(
    'create trigger trg_%I_updated_at before update on public.%I
     for each row execute function public.set_updated_at()',
    table_name,
    table_name
  );
end;
$$ language plpgsql;

create table if not exists public.sucursales (
  id text primary key,
  nombre text not null,
  direccion text not null,
  telefono text not null default '',
  codigo_postal varchar(5) not null default '',
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now()
);

create table if not exists public.empresa_config_fiscal (
  id integer primary key check (id = 1),
  rfc varchar(13) not null default '',
  razon_social text not null default '',
  regimen_fiscal varchar(3) not null default '',
  registro_patronal text null,
  actualizado_at timestamptz not null default now(),
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now()
);

create table if not exists public.proveedores (
  id text primary key,
  nombre text not null,
  contacto_nombre text not null default '',
  telefono text not null default '',
  email text not null default '',
  direccion text not null default '',
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now()
);

create table if not exists public.usuarios (
  id text primary key,
  email text not null unique,
  nombre text not null,
  role text not null check (role in ('SUPERADMIN', 'ADMIN', 'USUARIO')),
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  password_hash text not null,
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now()
);

create table if not exists public.marcas (
  id text primary key,
  nombre text not null unique,
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now()
);

create table if not exists public.categorias (
  id text primary key,
  nombre text not null unique,
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now()
);

create table if not exists public.unidades (
  id text primary key,
  nombre text not null unique,
  clave_sat varchar(3) not null default '',
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now()
);

create table if not exists public.productos (
  id text primary key,
  codigo_barras text unique,
  codigo_proveedor text not null default '',
  proveedor_id text not null references public.proveedores(id) on update cascade on delete restrict,
  clave_producto text not null default '',
  descripcion text not null,
  marca text not null default '',
  categoria text not null default '',
  unidad text not null default '',
  precio_costo numeric(12, 2) not null default 0,
  costo_promedio numeric(12, 4) not null default 0,
  precio_venta numeric(12, 2) not null default 0,
  sat_clave_prod_serv varchar(8) not null default '',
  sat_clave_unidad varchar(3) not null default '',
  precio_1 numeric(12, 2) not null default 0,
  precio_2 numeric(12, 2) not null default 0,
  precio_3 numeric(12, 2) not null default 0,
  precio_4 numeric(12, 2) not null default 0,
  mayoreo_apartir numeric(12, 3) not null default 0,
  a_granel boolean not null default false,
  no_en_catalogo boolean not null default false,
  ventas_negativas boolean not null default false,
  caducidad date null,
  fotos text not null default '',
  descripcion_catalogo text not null default '',
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now()
);

create table if not exists public.inventario_sucursal (
  producto_id text not null references public.productos(id) on update cascade on delete cascade,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  stock numeric(12, 3) not null default 0,
  stock_minimo numeric(12, 3) not null default 0,
  costo_promedio numeric(12, 4) not null default 0,
  precio_venta numeric(12, 2) not null default 0,
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  primary key (producto_id, sucursal_id)
);

create table if not exists public.promociones (
  id text primary key,
  nombre text not null,
  tipo_descuento text not null check (tipo_descuento in ('PORCENTAJE', 'MONTO_FIJO')),
  valor numeric(12, 2) not null check (valor > 0),
  fecha_inicio timestamptz not null,
  fecha_fin timestamptz not null,
  activo boolean not null default true,
  producto_id text null references public.productos(id) on update cascade on delete restrict,
  categoria_id text null,
  marca text null,
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  constraint chk_promociones_objetivo check (
    (producto_id is not null and categoria_id is null and marca is null)
    or (producto_id is null and categoria_id is not null and marca is null)
    or (producto_id is null and categoria_id is null and marca is not null)
  ),
  constraint chk_promociones_fechas check (fecha_fin >= fecha_inicio)
);

create table if not exists public.promocion_sucursales (
  promocion_id text not null references public.promociones(id) on update cascade on delete cascade,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete cascade,
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  primary key (promocion_id, sucursal_id)
);

create table if not exists public.clientes (
  id text primary key,
  nombre text not null,
  telefono text not null default '',
  direccion text not null default '',
  limite_credito numeric(12, 2) not null default 0,
  saldo_deudor numeric(12, 2) not null default 0,
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now()
);

create table if not exists public.clientes_datos_fiscales (
  cliente_id text primary key references public.clientes(id) on update cascade on delete cascade,
  rfc varchar(13) not null unique,
  razon_social text not null,
  regimen_fiscal varchar(3) not null,
  codigo_postal varchar(5) not null,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now()
);

create table if not exists public.compras (
  uuid uuid primary key default gen_random_uuid(),
  id text not null,
  proveedor_id text not null references public.proveedores(id) on update cascade on delete restrict,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  fecha timestamptz not null,
  total numeric(12, 2) not null default 0,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  unique (id, sucursal_id)
);

create table if not exists public.detalle_compras (
  uuid uuid primary key default gen_random_uuid(),
  id text not null,
  compra_uuid uuid null references public.compras(uuid) on update cascade on delete cascade,
  compra_id text not null,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  producto_id text not null references public.productos(id) on update cascade on delete restrict,
  cantidad numeric(12, 3) not null default 0,
  precio_costo_pactado numeric(12, 2) not null default 0,
  costo_promedio_resultante numeric(12, 4) null,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  unique (id, sucursal_id)
);

create table if not exists public.ventas (
  uuid uuid primary key default gen_random_uuid(),
  id text not null,
  usuario_id text not null references public.usuarios(id) on update cascade on delete restrict,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  fecha timestamptz not null,
  total numeric(12, 2) not null default 0,
  metodo_pago text not null check (metodo_pago in ('EFECTIVO', 'TARJETA', 'TRANSFERENCIA', 'CREDITO')),
  efectivo_recibido numeric(12, 2) null,
  cambio_entregado numeric(12, 2) null,
  cliente_id text null references public.clientes(id) on update cascade on delete restrict,
  cliente_rapido_nombre text null,
  cliente_rapido_telefono text null,
  cliente_rapido_domicilio text null,
  requiere_factura boolean not null default false,
  usuario_autorizo_cancelacion_id text null references public.usuarios(id) on update cascade on delete restrict,
  motivo_cancelacion text null,
  fecha_cancelacion timestamptz null,
  estado text not null default 'COMPLETADA' check (estado in ('COMPLETADA', 'CANCELADA')),
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  unique (id, sucursal_id)
);

create table if not exists public.detalle_ventas (
  uuid uuid primary key default gen_random_uuid(),
  id text not null,
  venta_uuid uuid null references public.ventas(uuid) on update cascade on delete cascade,
  venta_id text not null,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  producto_id text not null references public.productos(id) on update cascade on delete restrict,
  cantidad numeric(12, 3) not null default 0,
  precio_venta_pactado numeric(12, 2) not null default 0,
  costo_unitario_pactado numeric(12, 4) not null default 0,
  tipo_precio_vendido text not null default 'MOSTRADOR',
  precio_original numeric(12, 2) not null default 0,
  descuento_aplicado numeric(12, 2) not null default 0,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  unique (id, sucursal_id)
);

create table if not exists public.creditos_abonos (
  uuid uuid primary key default gen_random_uuid(),
  id text not null,
  cliente_id text not null references public.clientes(id) on update cascade on delete restrict,
  monto numeric(12, 2) not null default 0,
  fecha timestamptz not null,
  usuario_id text not null references public.usuarios(id) on update cascade on delete restrict,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  unique (id, sucursal_id)
);

create table if not exists public.cajas_sesiones (
  uuid uuid primary key default gen_random_uuid(),
  id text not null,
  usuario_id text not null references public.usuarios(id) on update cascade on delete restrict,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  fecha_apertura timestamptz not null,
  monto_inicial numeric(12, 2) not null default 0,
  fecha_cierre timestamptz null,
  monto_final_real numeric(12, 2) null,
  monto_esperado numeric(12, 2) not null default 0,
  estado text not null check (estado in ('ABIERTA', 'CERRADA')),
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  unique (id, sucursal_id)
);

create table if not exists public.caja_movimientos (
  uuid uuid primary key default gen_random_uuid(),
  id text not null,
  sesion_uuid uuid null references public.cajas_sesiones(uuid) on update cascade on delete cascade,
  sesion_id text not null,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  tipo text not null check (tipo in ('INGRESO', 'EGRESO')),
  monto numeric(12, 2) not null default 0,
  motivo text not null default '',
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  unique (id, sucursal_id)
);

create table if not exists public.traspasos (
  uuid uuid primary key default gen_random_uuid(),
  id text not null,
  sucursal_origen_id text not null references public.sucursales(id) on update cascade on delete restrict,
  sucursal_destino_id text not null references public.sucursales(id) on update cascade on delete restrict,
  usuario_id text not null references public.usuarios(id) on update cascade on delete restrict,
  fecha timestamptz not null,
  estado text not null default 'EN_TRANSITO' check (estado in ('EN_TRANSITO', 'RECIBIDO', 'RECHAZADO', 'CANCELADO')),
  usuario_recibio_id text null references public.usuarios(id) on update cascade on delete restrict,
  fecha_recepcion timestamptz null,
  observaciones_recepcion text null,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  unique (id, sucursal_origen_id)
);

create table if not exists public.detalle_traspasos (
  uuid uuid primary key default gen_random_uuid(),
  id text not null,
  traspaso_uuid uuid null references public.traspasos(uuid) on update cascade on delete cascade,
  traspaso_id text not null,
  sucursal_origen_id text not null references public.sucursales(id) on update cascade on delete restrict,
  producto_id text not null references public.productos(id) on update cascade on delete restrict,
  cantidad numeric(12, 3) not null default 0,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  unique (id, sucursal_origen_id)
);

create table if not exists public.mermas_ajustes (
  uuid uuid primary key default gen_random_uuid(),
  id text not null,
  producto_id text not null references public.productos(id) on update cascade on delete restrict,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  usuario_id text not null references public.usuarios(id) on update cascade on delete restrict,
  cantidad numeric(12, 3) not null default 0,
  tipo_movimiento text not null check (tipo_movimiento in ('MERMA', 'AJUSTE', 'AJUSTE_ENTRADA', 'AJUSTE_SALIDA')),
  motivo text not null,
  fecha timestamptz not null,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  unique (id, sucursal_id)
);

create table if not exists public.facturas_emitidas (
  uuid uuid primary key default gen_random_uuid(),
  id text not null,
  venta_uuid uuid null references public.ventas(uuid) on update cascade on delete restrict,
  venta_id text not null,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  uuid_sat text null,
  rfc_receptor varchar(13) not null,
  monto_total numeric(12, 2) not null default 0,
  estado text not null default 'PENDIENTE' check (estado in ('PENDIENTE', 'TIMBRADA', 'CANCELADA')),
  fecha_emision timestamptz not null,
  pdf_path text null,
  xml_path text null,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  unique (id, sucursal_id),
  unique (venta_id, sucursal_id)
);

create table if not exists public.movimientos_inventario (
  uuid uuid primary key default gen_random_uuid(),
  producto_id text not null references public.productos(id) on update cascade on delete restrict,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete restrict,
  tipo text not null check (tipo in ('COMPRA', 'VENTA', 'CANCELACION_VENTA', 'TRASPASO_SALIDA', 'TRASPASO_ENTRADA', 'TRASPASO_RECHAZO', 'MERMA', 'AJUSTE_ENTRADA', 'AJUSTE_SALIDA')),
  referencia_tipo text not null,
  referencia_id text not null,
  cantidad numeric(12, 3) not null,
  costo_unitario numeric(12, 4) null,
  usuario_id text null references public.usuarios(id) on update cascade on delete restrict,
  fecha timestamptz not null,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now()
);

create index if not exists idx_sucursales_eliminado on public.sucursales(eliminado);
create index if not exists idx_usuarios_eliminado on public.usuarios(eliminado);
create index if not exists idx_proveedores_eliminado on public.proveedores(eliminado);
create index if not exists idx_marcas_nombre on public.marcas(nombre);
create index if not exists idx_categorias_nombre on public.categorias(nombre);
create index if not exists idx_unidades_nombre on public.unidades(nombre);
create index if not exists idx_productos_eliminado on public.productos(eliminado);
create index if not exists idx_productos_descripcion on public.productos(descripcion);
create index if not exists idx_productos_codigo_barras on public.productos(codigo_barras);
create index if not exists idx_productos_codigo_proveedor on public.productos(codigo_proveedor);
create index if not exists idx_productos_mayoreo on public.productos(mayoreo_apartir) where mayoreo_apartir > 0;
create index if not exists idx_productos_caducidad on public.productos(caducidad) where caducidad is not null;
create index if not exists idx_inventario_sucursal_id on public.inventario_sucursal(sucursal_id);
create index if not exists idx_inventario_sucursal_eliminado on public.inventario_sucursal(eliminado);
create index if not exists idx_promociones_producto on public.promociones(producto_id);
create index if not exists idx_promociones_categoria on public.promociones(categoria_id);
create index if not exists idx_promociones_vigencia on public.promociones(activo, eliminado, fecha_inicio, fecha_fin);
create index if not exists idx_promocion_sucursales_sucursal on public.promocion_sucursales(sucursal_id, eliminado);
create index if not exists idx_clientes_eliminado on public.clientes(eliminado);
create index if not exists idx_clientes_nombre on public.clientes(nombre);
create index if not exists idx_clientes_updated_at on public.clientes(updated_at);
create index if not exists idx_ventas_sucursal_fecha on public.ventas(sucursal_id, fecha);
create index if not exists idx_ventas_updated_at on public.ventas(updated_at);
create index if not exists idx_detalle_ventas_venta_uuid on public.detalle_ventas(venta_uuid);
create index if not exists idx_abonos_cliente_fecha on public.creditos_abonos(cliente_id, fecha);
create index if not exists idx_cajas_sesiones_usuario_estado on public.cajas_sesiones(usuario_id, sucursal_id, estado);
create index if not exists idx_caja_movimientos_sesion_uuid on public.caja_movimientos(sesion_uuid);
create index if not exists idx_traspasos_fecha on public.traspasos(fecha);
create index if not exists idx_mermas_sucursal on public.mermas_ajustes(sucursal_id);
create index if not exists idx_movimientos_inventario_producto_sucursal on public.movimientos_inventario(producto_id, sucursal_id, fecha);
create index if not exists idx_movimientos_inventario_referencia on public.movimientos_inventario(referencia_tipo, referencia_id);
create index if not exists idx_facturas_emitidas_estado_fecha on public.facturas_emitidas(estado, fecha_emision);

select public.touch_updated_at('sucursales');
select public.touch_updated_at('empresa_config_fiscal');
select public.touch_updated_at('proveedores');
select public.touch_updated_at('usuarios');
select public.touch_updated_at('marcas');
select public.touch_updated_at('categorias');
select public.touch_updated_at('unidades');
select public.touch_updated_at('productos');
select public.touch_updated_at('inventario_sucursal');
select public.touch_updated_at('promociones');
select public.touch_updated_at('promocion_sucursales');
select public.touch_updated_at('clientes');
select public.touch_updated_at('clientes_datos_fiscales');
select public.touch_updated_at('compras');
select public.touch_updated_at('detalle_compras');
select public.touch_updated_at('ventas');
select public.touch_updated_at('detalle_ventas');
select public.touch_updated_at('creditos_abonos');
select public.touch_updated_at('cajas_sesiones');
select public.touch_updated_at('caja_movimientos');
select public.touch_updated_at('traspasos');
select public.touch_updated_at('detalle_traspasos');
select public.touch_updated_at('mermas_ajustes');
select public.touch_updated_at('facturas_emitidas');
select public.touch_updated_at('movimientos_inventario');

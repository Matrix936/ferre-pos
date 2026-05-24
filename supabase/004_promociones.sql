-- Ferre-POS: Promociones y descuentos
-- Ejecutar en Supabase SQL Editor después del esquema base.

create table if not exists public.promociones (
  id uuid primary key default gen_random_uuid(),
  nombre text not null,
  tipo_descuento varchar(20) not null check (tipo_descuento in ('PORCENTAJE', 'MONTO_FIJO')),
  valor numeric(12,2) not null check (valor > 0),
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
  promocion_id uuid not null references public.promociones(id) on update cascade on delete cascade,
  sucursal_id text not null references public.sucursales(id) on update cascade on delete cascade,
  eliminado boolean not null default false,
  sincronizado boolean not null default true,
  updated_at timestamptz not null default now(),
  primary key (promocion_id, sucursal_id)
);

create index if not exists idx_promociones_producto on public.promociones(producto_id);
create index if not exists idx_promociones_categoria on public.promociones(categoria_id);
create index if not exists idx_promociones_vigencia
  on public.promociones(activo, eliminado, fecha_inicio, fecha_fin);
create index if not exists idx_promocion_sucursales_sucursal
  on public.promocion_sucursales(sucursal_id, eliminado);

create or replace function public.set_updated_at()
returns trigger as $$
begin
  new.updated_at = now();
  return new;
end;
$$ language plpgsql;

drop trigger if exists trg_promociones_updated_at on public.promociones;
create trigger trg_promociones_updated_at
before update on public.promociones
for each row execute function public.set_updated_at();

drop trigger if exists trg_promocion_sucursales_updated_at on public.promocion_sucursales;
create trigger trg_promocion_sucursales_updated_at
before update on public.promocion_sucursales
for each row execute function public.set_updated_at();

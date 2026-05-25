-- ADVERTENCIA:
-- Esto borra TODAS las tablas, vistas, funciones, triggers y datos dentro del schema public.
-- No borra usuarios de auth.users ni archivos de storage, pero sí todo Ferre-POS en public.

drop schema if exists public cascade;

create schema public;

grant usage on schema public to postgres, anon, authenticated, service_role;
grant all on schema public to postgres, service_role;

alter default privileges in schema public
grant all on tables to postgres, service_role;

alter default privileges in schema public
grant select, insert, update, delete on tables to anon, authenticated;

alter default privileges in schema public
grant all on sequences to postgres, service_role;

alter default privileges in schema public
grant usage, select on sequences to anon, authenticated;

alter default privileges in schema public
grant all on functions to postgres, service_role;

alter default privileges in schema public
grant execute on functions to anon, authenticated;
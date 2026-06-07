import { Box, CircularProgress, MenuItem, Paper, TextField, Typography } from '@mui/material';
import { ReactNode } from 'react';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';

export interface IndicadorFiltro {
  fechaInicio: string;
  fechaFin: string;
  sucursalId: string;
  marca: string;
  categoria: string;
  proveedorId: string;
  metodoPago: string;
  usuarioId: string;
}

interface IndicadorFiltrosProps {
  filtro: IndicadorFiltro;
  onChange: (filtro: IndicadorFiltro) => void;
  showDates?: boolean;
  showCatalogFilters?: boolean;
  showPaymentFilter?: boolean;
  showUserFilter?: boolean;
}

export const money = (value: number) => `$${Number(value || 0).toFixed(2)}`;
export const numberText = (value: number) => Number(value || 0).toLocaleString('es-MX');
export const percent = (value: number) => `${Number(value || 0).toFixed(2)}%`;

export function toBackendFiltro(filtro: IndicadorFiltro) {
  return {
    fechaInicio: filtro.fechaInicio ? `${filtro.fechaInicio}T00:00:00.000Z` : undefined,
    fechaFin: filtro.fechaFin ? `${filtro.fechaFin}T23:59:59.999Z` : undefined,
    sucursalId: filtro.sucursalId || undefined,
    marca: filtro.marca || undefined,
    categoria: filtro.categoria || undefined,
    proveedorId: filtro.proveedorId || undefined,
    metodoPago: filtro.metodoPago || undefined,
    usuarioId: filtro.usuarioId || undefined,
  };
}

export const emptyIndicadorFiltro = (): IndicadorFiltro => ({
  fechaInicio: '',
  fechaFin: '',
  sucursalId: '',
  marca: '',
  categoria: '',
  proveedorId: '',
  metodoPago: '',
  usuarioId: '',
});

export function previousPeriodFiltro(filtro: IndicadorFiltro): IndicadorFiltro | null {
  if (!filtro.fechaInicio || !filtro.fechaFin) return null;
  const start = new Date(`${filtro.fechaInicio}T00:00:00`);
  const end = new Date(`${filtro.fechaFin}T00:00:00`);
  if (!Number.isFinite(start.getTime()) || !Number.isFinite(end.getTime()) || end < start) return null;
  const days = Math.max(1, Math.round((end.getTime() - start.getTime()) / 86_400_000) + 1);
  const previousEnd = addDays(start, -1);
  const previousStart = addDays(previousEnd, -(days - 1));
  return {
    ...filtro,
    fechaInicio: toDateInput(previousStart),
    fechaFin: toDateInput(previousEnd),
  };
}

export function comparisonText(current: number, previous?: number | null) {
  if (previous === undefined || previous === null) return 'Selecciona un periodo para comparar';
  if (previous === 0 && current === 0) return 'Sin variación vs periodo anterior';
  if (previous === 0) return 'Nuevo movimiento vs periodo anterior';
  const delta = ((current - previous) / Math.abs(previous)) * 100;
  const sign = delta >= 0 ? '+' : '';
  return `${sign}${delta.toFixed(1)}% vs periodo anterior`;
}

const todayString = () => new Date().toISOString().slice(0, 10);

const addDays = (date: Date, days: number) => {
  const copy = new Date(date);
  copy.setDate(copy.getDate() + days);
  return copy;
};

const toDateInput = (date: Date) => date.toISOString().slice(0, 10);

const periodosRapidos = [
  { label: 'Hoy', getRange: () => ({ start: todayString(), end: todayString() }) },
  { label: '7 días', getRange: () => ({ start: toDateInput(addDays(new Date(), -6)), end: todayString() }) },
  { label: '30 días', getRange: () => ({ start: toDateInput(addDays(new Date(), -29)), end: todayString() }) },
  { label: 'Mes', getRange: () => {
    const now = new Date();
    return { start: toDateInput(new Date(now.getFullYear(), now.getMonth(), 1)), end: todayString() };
  } },
  { label: 'Año', getRange: () => {
    const now = new Date();
    return { start: toDateInput(new Date(now.getFullYear(), 0, 1)), end: todayString() };
  } },
];

export function IndicadorPage({ title, subtitle, children }: { title: string; subtitle: string; children: ReactNode }) {
  return (
    <Box sx={{ width: '100%', mt: 2 }}>
      <Box sx={{ mb: 3 }}>
        <Typography variant="h5" sx={{ fontWeight: 800 }}>
          {title}
        </Typography>
        <Typography variant="body2" color="text.secondary">
          {subtitle}
        </Typography>
      </Box>
      {children}
    </Box>
  );
}

export function IndicadorLoadingBanner({ visible }: { visible: boolean }) {
  if (!visible) return null;
  return (
    <Paper
      elevation={0}
      sx={{
        p: 1.5,
        borderRadius: 2,
        border: '1px solid',
        borderColor: 'divider',
        mb: 2,
        display: 'flex',
        alignItems: 'center',
        gap: 1.5,
      }}
    >
      <CircularProgress size={18} />
      <Typography variant="body2" color="text.secondary">
        Recalculando indicadores...
      </Typography>
    </Paper>
  );
}

export function IndicadorFiltros({
  filtro,
  onChange,
  showDates = true,
  showCatalogFilters = false,
  showPaymentFilter = false,
  showUserFilter = false,
}: IndicadorFiltrosProps) {
  const { user } = useAuth();
  const { sucursales, marcas, categorias, proveedores, usuarios } = useCatalogos();
  const isSuperAdmin = user?.role === 'SUPERADMIN';

  return (
    <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
      {showDates && (
        <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap', mb: 2 }}>
          {periodosRapidos.map((periodo) => (
            <Paper
              key={periodo.label}
              component="button"
              type="button"
              elevation={0}
              onClick={() => {
                const range = periodo.getRange();
                onChange({ ...filtro, fechaInicio: range.start, fechaFin: range.end });
              }}
              sx={{
                px: 1.25,
                py: 0.75,
                borderRadius: '8px',
                border: '1px solid',
                borderColor: 'divider',
                bgcolor: 'background.paper',
                cursor: 'pointer',
                fontWeight: 700,
                color: 'text.secondary',
                '&:hover': { bgcolor: 'action.hover' },
              }}
            >
              {periodo.label}
            </Paper>
          ))}
        </Box>
      )}
      <Box sx={{ display: 'grid', gap: 2, gridTemplateColumns: { xs: '1fr', md: 'repeat(3, minmax(0, 1fr))' } }}>
        {showDates && (
          <>
            <TextField
              label="Fecha inicio"
              type="date"
              value={filtro.fechaInicio}
              onChange={(event) => onChange({ ...filtro, fechaInicio: event.target.value })}
              slotProps={{ inputLabel: { shrink: true } }}
            />
            <TextField
              label="Fecha fin"
              type="date"
              value={filtro.fechaFin}
              onChange={(event) => onChange({ ...filtro, fechaFin: event.target.value })}
              slotProps={{ inputLabel: { shrink: true } }}
            />
          </>
        )}
        {isSuperAdmin && (
          <TextField
            select
            label="Sucursal"
            value={filtro.sucursalId}
            onChange={(event) => onChange({ ...filtro, sucursalId: event.target.value })}
          >
            <MenuItem value="">Todas</MenuItem>
            {sucursales.map((sucursal) => (
              <MenuItem key={sucursal.id} value={sucursal.id}>
                {sucursal.nombre}
              </MenuItem>
            ))}
          </TextField>
        )}
        {showCatalogFilters && (
          <>
            <TextField select label="Marca" value={filtro.marca} onChange={(event) => onChange({ ...filtro, marca: event.target.value })}>
              <MenuItem value="">Todas</MenuItem>
              {marcas.map((marca) => <MenuItem key={marca.id} value={marca.nombre}>{marca.nombre}</MenuItem>)}
            </TextField>
            <TextField select label="Categoría" value={filtro.categoria} onChange={(event) => onChange({ ...filtro, categoria: event.target.value })}>
              <MenuItem value="">Todas</MenuItem>
              {categorias.map((categoria) => <MenuItem key={categoria.id} value={categoria.nombre}>{categoria.nombre}</MenuItem>)}
            </TextField>
            <TextField select label="Proveedor" value={filtro.proveedorId} onChange={(event) => onChange({ ...filtro, proveedorId: event.target.value })}>
              <MenuItem value="">Todos</MenuItem>
              {proveedores.map((proveedor) => <MenuItem key={proveedor.id} value={proveedor.id}>{proveedor.nombre}</MenuItem>)}
            </TextField>
          </>
        )}
        {showPaymentFilter && (
          <TextField select label="Método de pago" value={filtro.metodoPago} onChange={(event) => onChange({ ...filtro, metodoPago: event.target.value })}>
            <MenuItem value="">Todos</MenuItem>
            <MenuItem value="EFECTIVO">Efectivo</MenuItem>
            <MenuItem value="TARJETA">Tarjeta</MenuItem>
            <MenuItem value="TRANSFERENCIA">Transferencia</MenuItem>
            <MenuItem value="CREDITO">Crédito</MenuItem>
          </TextField>
        )}
        {showUserFilter && (
          <TextField select label="Cajero/usuario" value={filtro.usuarioId} onChange={(event) => onChange({ ...filtro, usuarioId: event.target.value })}>
            <MenuItem value="">Todos</MenuItem>
            {usuarios.map((usuario) => <MenuItem key={usuario.id} value={usuario.id}>{usuario.nombre}</MenuItem>)}
          </TextField>
        )}
      </Box>
    </Paper>
  );
}

export function MetricCard({ label, value, helper }: { label: string; value: string; helper?: string }) {
  return (
    <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
      <Typography variant="caption" color="text.secondary" sx={{ fontWeight: 800, textTransform: 'uppercase' }}>
        {label}
      </Typography>
      <Typography variant="h5" sx={{ fontWeight: 900, mt: 0.5 }}>
        {value}
      </Typography>
      {helper && (
        <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
          {helper}
        </Typography>
      )}
    </Paper>
  );
}

export function MetricGrid({ children }: { children: ReactNode }) {
  return <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: 'repeat(4, minmax(0, 1fr))' }, gap: 1.5, mb: 2 }}>{children}</Box>;
}

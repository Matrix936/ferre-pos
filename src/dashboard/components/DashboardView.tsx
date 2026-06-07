import { useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { invoke } from '@tauri-apps/api/core';
import {
  Box,
  Button,
  Card,
  CardContent,
  CircularProgress,
  Alert,
  MenuItem,
  Paper,
  Chip,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from '@mui/material';
import { ShoppingCart as ReabastecerIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { TablePager } from '../../shared/components/TablePager';

interface DashboardStats {
  totalVendido: number;
  utilidadNeta: number;
  transacciones: number;
  ticketPromedio: number;
  margenPorcentaje: number;
}

interface ProductoBajoStock {
  productoId: string;
  descripcion: string;
  marca: string;
  sucursalId: string;
  sucursalNombre: string;
  stock: number;
  stockMinimo: number;
}

interface ProductosBajoStockPage {
  rows: ProductoBajoStock[];
  total: number;
}

interface ProductoMasVendido {
  productoId: string;
  descripcion: string;
  marca: string;
  unidadesVendidas: number;
}

interface IndicadorInventarioResumen {
  productosEnInventario: number;
  valorInventario: number;
  stockTotal: number;
  stockBajo: number;
  sinStock: number;
  sobreStock: number;
}

interface IndicadorFinancieroResumen {
  ingresosCaja: number;
  egresosCaja: number;
  ventasEfectivo: number;
  ventasTarjeta: number;
  ventasTransferencia: number;
  ventasCredito: number;
  compras: number;
  cuentasPorCobrar: number;
  flujoNetoEstimado: number;
}

const money = (value: number) => `$${Number(value || 0).toFixed(2)}`;
const numberText = (value: number) => Number(value || 0).toLocaleString('es-MX');

export function DashboardView() {
  const { user } = useAuth();
  const { sucursales } = useCatalogos();
  const navigate = useNavigate();
  const [selectedSucursalId, setSelectedSucursalId] = useState('');
  const [fechaInicio, setFechaInicio] = useState('');
  const [fechaFin, setFechaFin] = useState('');
  const [stats, setStats] = useState<DashboardStats>({
    totalVendido: 0,
    utilidadNeta: 0,
    transacciones: 0,
    ticketPromedio: 0,
    margenPorcentaje: 0,
  });
  const [bajoStock, setBajoStock] = useState<ProductoBajoStock[]>([]);
  const [topVendidos, setTopVendidos] = useState<ProductoMasVendido[]>([]);
  const [inventario, setInventario] = useState<IndicadorInventarioResumen | null>(null);
  const [financiero, setFinanciero] = useState<IndicadorFinancieroResumen | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [bajoStockPage, setBajoStockPage] = useState(0);
  const [bajoStockPageSize, setBajoStockPageSizeState] = useState(10);
  const [bajoStockTotalRows, setBajoStockTotalRows] = useState(0);

  const isSuperAdmin = user?.role === 'SUPERADMIN';
  const canViewDashboard = user?.role === 'SUPERADMIN' || user?.role === 'ADMIN';

  const filtro = useMemo(
    () => ({
      sucursalId: selectedSucursalId || undefined,
      fechaInicio: fechaInicio ? `${fechaInicio}T00:00:00.000Z` : undefined,
      fechaFin: fechaFin ? `${fechaFin}T23:59:59.999Z` : undefined,
    }),
    [selectedSucursalId, fechaInicio, fechaFin],
  );

  const fetchData = async () => {
    setLoading(true);
    setError('');
    try {
      const [statsData, lowStockData, topData, inventarioData, financieroData] = await Promise.all([
        invoke<DashboardStats>('get_dashboard_stats', { filtro }),
        invoke<ProductosBajoStockPage>('get_productos_bajo_stock_page', {
          sucursalId: filtro.sucursalId,
          page: bajoStockPage,
          pageSize: bajoStockPageSize,
        }),
        invoke<ProductoMasVendido[]>('get_productos_mas_vendidos', { filtro }),
        invoke<IndicadorInventarioResumen>('get_indicador_inventario', { filtro }),
        invoke<IndicadorFinancieroResumen>('get_indicador_financiero', { filtro }),
      ]);
      setStats(statsData);
      setBajoStock(lowStockData.rows);
      setBajoStockTotalRows(lowStockData.total);
      setTopVendidos(topData);
      setInventario(inventarioData);
      setFinanciero(financieroData);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!canViewDashboard) return;
    fetchData();
  }, [canViewDashboard, filtro, bajoStockPage, bajoStockPageSize]);

  const bajoStockTotalPages = Math.max(1, Math.ceil(bajoStockTotalRows / bajoStockPageSize));
  const bajoStockFromRow = bajoStockTotalRows === 0 ? 0 : bajoStockPage * bajoStockPageSize + 1;
  const bajoStockToRow = Math.min((bajoStockPage + 1) * bajoStockPageSize, bajoStockTotalRows);
  const setBajoStockPageSize = (value: number) => {
    setBajoStockPageSizeState(value);
    setBajoStockPage(0);
  };

  if (!canViewDashboard) {
    return (
      <Box sx={{ width: '100%', mt: 2 }}>
        <Typography variant="h5" sx={{ fontWeight: 700, mb: 1 }}>
          Panel principal
        </Typography>
        <Typography color="text.secondary">
          Bienvenido. Este panel analítico está disponible para administradores.
        </Typography>
      </Box>
    );
  }

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3 }}>
        Dashboard Analítico
      </Typography>

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
          {isSuperAdmin ? (
            <TextField
              select
              label="Sucursal"
              value={selectedSucursalId}
              onChange={(event) => setSelectedSucursalId(event.target.value)}
              sx={{ minWidth: 220 }}
            >
              <MenuItem value="">Todas</MenuItem>
              {sucursales.map((sucursal) => (
                <MenuItem key={sucursal.id} value={sucursal.id}>
                  {sucursal.nombre}
                </MenuItem>
              ))}
            </TextField>
          ) : (
            <TextField
              label="Sucursal"
              value={sucursales.find((sucursal) => sucursal.id === user?.sucursalId)?.nombre ?? 'Mi sucursal'}
              sx={{ minWidth: 220 }}
              disabled
            />
          )}
          <TextField
            label="Fecha inicio"
            type="date"
            value={fechaInicio}
            onChange={(event) => setFechaInicio(event.target.value)}
            slotProps={{ inputLabel: { shrink: true } }}
          />
          <TextField
            label="Fecha fin"
            type="date"
            value={fechaFin}
            onChange={(event) => setFechaFin(event.target.value)}
            slotProps={{ inputLabel: { shrink: true } }}
          />
        </Box>
      </Paper>

      {loading && (
        <Paper elevation={0} sx={{ p: 1.5, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2, display: 'flex', alignItems: 'center', gap: 1.5 }}>
          <CircularProgress size={18} />
          <Typography variant="body2" color="text.secondary">Actualizando métricas...</Typography>
        </Paper>
      )}

      <Box sx={{ display: 'grid', gap: 2, gridTemplateColumns: { xs: '1fr', sm: 'repeat(2, 1fr)', xl: 'repeat(4, 1fr)' }, mb: 2 }}>
        <ExecutiveCard label="Ventas" value={money(stats.totalVendido)} helper={`${numberText(stats.transacciones)} tickets`} onClick={() => navigate('/indicadores/ventas')} />
        <ExecutiveCard label="Utilidad bruta" value={money(stats.utilidadNeta)} helper={`Margen ${stats.margenPorcentaje.toFixed(2)}%`} onClick={() => navigate('/indicadores/rentabilidad')} />
        <ExecutiveCard label="Flujo neto" value={money(financiero?.flujoNetoEstimado ?? 0)} helper={`Caja: ${money((financiero?.ingresosCaja ?? 0) - (financiero?.egresosCaja ?? 0))}`} onClick={() => navigate('/indicadores/financiero')} />
        <ExecutiveCard label="Inventario valuado" value={money(inventario?.valorInventario ?? 0)} helper={`${numberText(inventario?.stockTotal ?? 0)} piezas`} onClick={() => navigate('/indicadores/inventario')} />
        <ExecutiveCard label="Ticket promedio" value={money(stats.ticketPromedio)} helper="Venta promedio por ticket" onClick={() => navigate('/indicadores/ventas')} />
        <ExecutiveCard label="Cuentas por cobrar" value={money(financiero?.cuentasPorCobrar ?? 0)} helper="Crédito pendiente" onClick={() => navigate('/clientes')} />
        <ExecutiveCard label="Stock bajo" value={numberText(inventario?.stockBajo ?? bajoStock.length)} helper={`${numberText(inventario?.sinStock ?? 0)} sin stock`} onClick={() => navigate('/indicadores/inventario')} />
        <ExecutiveCard label="Sobreinventario" value={numberText(inventario?.sobreStock ?? 0)} helper="Capital detenido" onClick={() => navigate('/indicadores/inventario')} />
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 800, mb: 1 }}>
          Alertas gerenciales
        </Typography>
        <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
          <Chip
            label={`${numberText(inventario?.stockBajo ?? bajoStock.length)} productos en bajo stock`}
            color={(inventario?.stockBajo ?? bajoStock.length) > 0 ? 'warning' : 'success'}
            variant="outlined"
          />
          <Chip
            label={`${numberText(inventario?.sinStock ?? 0)} productos sin stock`}
            color={(inventario?.sinStock ?? 0) > 0 ? 'error' : 'success'}
            variant="outlined"
          />
          <Chip
            label={`${money(financiero?.cuentasPorCobrar ?? 0)} por cobrar`}
            color={(financiero?.cuentasPorCobrar ?? 0) > 0 ? 'warning' : 'success'}
            variant="outlined"
          />
          <Chip
            label={`Margen ${stats.margenPorcentaje.toFixed(2)}%`}
            color={stats.margenPorcentaje < 15 && stats.totalVendido > 0 ? 'warning' : 'success'}
            variant="outlined"
          />
        </Box>
      </Paper>

      <Box sx={{ display: 'grid', gap: 2, gridTemplateColumns: { xs: '1fr', lg: '1.2fr 1fr' } }}>
        <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
          <Box sx={{ p: 2, display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <Typography variant="subtitle1" sx={{ fontWeight: 700 }}>Productos Bajo Stock</Typography>
            <Button size="small" variant="contained" startIcon={<ReabastecerIcon />} onClick={() => navigate('/compras')}>
              Reabastecer
            </Button>
          </Box>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>Producto</TableCell>
                <TableCell>Marca</TableCell>
                <TableCell>Sucursal</TableCell>
                <TableCell align="right">Stock</TableCell>
                <TableCell align="right">Mín.</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {bajoStock.map((item) => (
                <TableRow key={`${item.productoId}-${item.sucursalId}`} hover>
                  <TableCell>{item.descripcion}</TableCell>
                  <TableCell>{item.marca || '-'}</TableCell>
                  <TableCell>{item.sucursalNombre}</TableCell>
                  <TableCell align="right">{item.stock}</TableCell>
                  <TableCell align="right">{item.stockMinimo}</TableCell>
                </TableRow>
              ))}
              {bajoStock.length === 0 && (
                <TableRow>
                  <TableCell colSpan={5} align="center" sx={{ py: 3, color: 'text.secondary' }}>
                    Sin alertas de bajo stock.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
          <TablePager
            page={bajoStockPage}
            pageSize={bajoStockPageSize}
            totalPages={bajoStockTotalPages}
            totalRows={bajoStockTotalRows}
            fromRow={bajoStockFromRow}
            toRow={bajoStockToRow}
            canPreviousPage={bajoStockPage > 0}
            canNextPage={bajoStockPage + 1 < bajoStockTotalPages}
            onPreviousPage={() => setBajoStockPage((prev) => Math.max(0, prev - 1))}
            onNextPage={() => setBajoStockPage((prev) => Math.min(bajoStockTotalPages - 1, prev + 1))}
            onPageSizeChange={setBajoStockPageSize}
            rowLabel="alertas"
          />
        </Paper>

        <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
          <Box sx={{ p: 2 }}>
            <Typography variant="subtitle1" sx={{ fontWeight: 700, mb: 1 }}>Top 5 Más Vendidos</Typography>
            {topVendidos.map((item, index) => (
              <Box key={item.productoId} sx={{ py: 1.2, borderBottom: index < topVendidos.length - 1 ? '1px solid' : 'none', borderColor: 'divider' }}>
                <Typography variant="body2" sx={{ fontWeight: 600 }}>{item.descripcion}</Typography>
                <Typography variant="caption" color="text.secondary">
                  {item.marca || 'Sin marca'} · {item.unidadesVendidas} unidades
                </Typography>
              </Box>
            ))}
            {topVendidos.length === 0 && (
              <Typography variant="body2" color="text.secondary">Aún no hay ventas en el rango seleccionado.</Typography>
            )}
            <Typography variant="caption" sx={{ display: 'block', mt: 2 }} color="text.secondary">
              Transacciones: {stats.transacciones}
            </Typography>
          </Box>
        </Paper>
      </Box>
    </Box>
  );
}

function ExecutiveCard({
  label,
  value,
  helper,
  onClick,
}: {
  label: string;
  value: string;
  helper: string;
  onClick: () => void;
}) {
  return (
    <Card
      elevation={0}
      onClick={onClick}
      sx={{
        border: '1px solid',
        borderColor: 'divider',
        cursor: 'pointer',
        transition: 'transform 160ms ease, border-color 160ms ease, background-color 160ms ease',
        '&:hover': {
          transform: 'translateY(-1px)',
          borderColor: 'primary.main',
          bgcolor: 'action.hover',
        },
      }}
    >
      <CardContent>
        <Typography color="text.secondary" variant="subtitle2" sx={{ fontWeight: 800 }}>
          {label}
        </Typography>
        <Typography variant="h4" sx={{ fontWeight: 900, mt: 1 }}>
          {value}
        </Typography>
        <Typography variant="caption" color="text.secondary">
          {helper}
        </Typography>
      </CardContent>
    </Card>
  );
}

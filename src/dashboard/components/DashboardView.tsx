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

interface ProductoMasVendido {
  productoId: string;
  descripcion: string;
  marca: string;
  unidadesVendidas: number;
}

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
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

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
      const [statsData, lowStockData, topData] = await Promise.all([
        invoke<DashboardStats>('get_dashboard_stats', { filtro }),
        invoke<ProductoBajoStock[]>('get_productos_bajo_stock', { sucursalId: filtro.sucursalId }),
        invoke<ProductoMasVendido[]>('get_productos_mas_vendidos', { filtro }),
      ]);
      setStats(statsData);
      setBajoStock(lowStockData);
      setTopVendidos(topData);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (!canViewDashboard) return;
    fetchData();
  }, [canViewDashboard, filtro]);

  if (!canViewDashboard) {
    return (
      <Box sx={{ maxWidth: 900, mx: 'auto', mt: 2 }}>
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
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
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

      <Box sx={{ display: 'grid', gap: 2, gridTemplateColumns: { xs: '1fr', md: 'repeat(5, 1fr)' }, mb: 2 }}>
        <Card elevation={0} sx={{ border: '1px solid', borderColor: 'divider' }}>
          <CardContent>
            <Typography color="text.secondary" variant="subtitle2">Ventas</Typography>
            <Typography variant="h4" sx={{ fontWeight: 800, mt: 1 }}>${stats.totalVendido.toFixed(2)}</Typography>
          </CardContent>
        </Card>
        <Card elevation={0} sx={{ border: '1px solid', borderColor: 'divider' }}>
          <CardContent>
            <Typography color="text.secondary" variant="subtitle2">Utilidad Neta</Typography>
            <Typography variant="h4" sx={{ fontWeight: 800, mt: 1 }}>${stats.utilidadNeta.toFixed(2)}</Typography>
          </CardContent>
        </Card>
        <Card elevation={0} sx={{ border: '1px solid', borderColor: 'divider' }}>
          <CardContent>
            <Typography color="text.secondary" variant="subtitle2">Margen</Typography>
            <Typography variant="h4" sx={{ fontWeight: 800, mt: 1 }}>{stats.margenPorcentaje.toFixed(2)}%</Typography>
          </CardContent>
        </Card>
        <Card elevation={0} sx={{ border: '1px solid', borderColor: 'divider' }}>
          <CardContent>
            <Typography color="text.secondary" variant="subtitle2">Ticket Promedio</Typography>
            <Typography variant="h4" sx={{ fontWeight: 800, mt: 1 }}>${stats.ticketPromedio.toFixed(2)}</Typography>
          </CardContent>
        </Card>
        <Card elevation={0} sx={{ border: '1px solid', borderColor: 'divider' }}>
          <CardContent>
            <Typography color="text.secondary" variant="subtitle2">Alertas Bajo Stock</Typography>
            <Typography variant="h4" sx={{ fontWeight: 800, mt: 1 }}>{bajoStock.length}</Typography>
          </CardContent>
        </Card>
      </Box>

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
              {bajoStock.slice(0, 8).map((item) => (
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

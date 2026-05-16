import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  MenuItem,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from '@mui/material';
import { RoleGuard } from '../../auth/components/RoleGuard';
import { Sucursal } from '../../sucursales/types';
import { User } from '../../auth/types';

interface HistorialVenta {
  id: string;
  fecha: string;
  total: number;
  metodoPago: string;
  estado: string;
  sucursalId: string;
  sucursalNombre: string;
  usuarioId: string;
  usuarioNombre: string;
  clienteId?: string;
  clienteNombre?: string;
}

interface HistorialVentaDetalle {
  id: string;
  ventaId: string;
  productoId: string;
  descripcion: string;
  marca: string;
  cantidad: number;
  precioVentaPactado: number;
}

export function HistorialVentasView() {
  const [ventas, setVentas] = useState<HistorialVenta[]>([]);
  const [sucursales, setSucursales] = useState<Sucursal[]>([]);
  const [usuarios, setUsuarios] = useState<User[]>([]);
  const [fechaInicio, setFechaInicio] = useState('');
  const [fechaFin, setFechaFin] = useState('');
  const [sucursalId, setSucursalId] = useState('');
  const [usuarioId, setUsuarioId] = useState('');
  const [selectedVenta, setSelectedVenta] = useState<HistorialVenta | null>(null);
  const [detalle, setDetalle] = useState<HistorialVentaDetalle[]>([]);
  const [loadingCancel, setLoadingCancel] = useState(false);

  const fetchBaseData = async () => {
    const [sucursalesData, usuariosData] = await Promise.all([
      invoke<Sucursal[]>('get_sucursales'),
      invoke<User[]>('get_usuarios'),
    ]);
    setSucursales(sucursalesData);
    setUsuarios(usuariosData);
  };

  const fetchVentas = async () => {
    const filtro = {
      fechaInicio: fechaInicio ? `${fechaInicio}T00:00:00.000Z` : undefined,
      fechaFin: fechaFin ? `${fechaFin}T23:59:59.999Z` : undefined,
      sucursalId: sucursalId || undefined,
      usuarioId: usuarioId || undefined,
    };
    const data = await invoke<HistorialVenta[]>('get_historial_ventas', { filtro });
    setVentas(data);
  };

  useEffect(() => {
    fetchBaseData().catch((error) => console.error('Error base historial:', error));
  }, []);

  useEffect(() => {
    fetchVentas().catch((error) => console.error('Error historial ventas:', error));
  }, [fechaInicio, fechaFin, sucursalId, usuarioId]);

  const openDetalle = async (venta: HistorialVenta) => {
    setSelectedVenta(venta);
    const data = await invoke<HistorialVentaDetalle[]>('get_detalle_venta', { ventaId: venta.id });
    setDetalle(data);
  };

  const handleCancelarVenta = async () => {
    if (!selectedVenta) return;
    if (!confirm('¿Seguro que deseas cancelar esta venta?')) return;
    setLoadingCancel(true);
    try {
      await invoke('cancelar_venta', { ventaId: selectedVenta.id });
      const ventaActualizada = { ...selectedVenta, estado: 'CANCELADA' };
      setSelectedVenta(ventaActualizada);
      setVentas((prev) => prev.map((v) => (v.id === ventaActualizada.id ? ventaActualizada : v)));
    } catch (error) {
      alert(`Error al cancelar venta: ${error}`);
    } finally {
      setLoadingCancel(false);
    }
  };

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3 }}>
        Historial de Ventas
      </Typography>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'grid', gap: 2, gridTemplateColumns: { xs: '1fr', md: 'repeat(4, minmax(180px, 1fr))' } }}>
          <TextField label="Fecha inicio" type="date" value={fechaInicio} onChange={(e) => setFechaInicio(e.target.value)} slotProps={{ inputLabel: { shrink: true } }} />
          <TextField label="Fecha fin" type="date" value={fechaFin} onChange={(e) => setFechaFin(e.target.value)} slotProps={{ inputLabel: { shrink: true } }} />
          <TextField select label="Sucursal" value={sucursalId} onChange={(e) => setSucursalId(e.target.value)}>
            <MenuItem value="">Todas</MenuItem>
            {sucursales.map((sucursal) => (
              <MenuItem key={sucursal.id} value={sucursal.id}>{sucursal.nombre}</MenuItem>
            ))}
          </TextField>
          <TextField select label="Usuario" value={usuarioId} onChange={(e) => setUsuarioId(e.target.value)}>
            <MenuItem value="">Todos</MenuItem>
            {usuarios.map((usuario) => (
              <MenuItem key={usuario.id} value={usuario.id}>{usuario.nombre}</MenuItem>
            ))}
          </TextField>
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Folio</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Fecha</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Sucursal</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Usuario</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Método</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Estado</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Total</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600 }}>Detalle</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {ventas.map((venta) => (
                <TableRow key={venta.id} hover>
                  <TableCell>{venta.id.slice(0, 8)}</TableCell>
                  <TableCell>{new Date(venta.fecha).toLocaleString()}</TableCell>
                  <TableCell>{venta.sucursalNombre}</TableCell>
                  <TableCell>{venta.usuarioNombre}</TableCell>
                  <TableCell>{venta.metodoPago}</TableCell>
                  <TableCell>{venta.estado}</TableCell>
                  <TableCell>${venta.total.toFixed(2)}</TableCell>
                  <TableCell align="right">
                    <Button size="small" onClick={() => openDetalle(venta)}>Ver ticket</Button>
                  </TableCell>
                </TableRow>
              ))}
              {ventas.length === 0 && (
                <TableRow>
                  <TableCell colSpan={8} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay ventas para los filtros seleccionados.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Dialog open={Boolean(selectedVenta)} onClose={() => setSelectedVenta(null)} maxWidth="md" fullWidth>
        <DialogTitle>Detalle de Venta</DialogTitle>
        <DialogContent sx={{ pt: 2 }}>
          <Typography variant="body2" sx={{ mb: 2 }}>
            Folio: <strong>{selectedVenta?.id}</strong> · Estado: <strong>{selectedVenta?.estado}</strong>
          </Typography>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>Producto</TableCell>
                <TableCell>Marca</TableCell>
                <TableCell align="right">Cantidad</TableCell>
                <TableCell align="right">Precio</TableCell>
                <TableCell align="right">Importe</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {detalle.map((item) => (
                <TableRow key={item.id}>
                  <TableCell>{item.descripcion}</TableCell>
                  <TableCell>{item.marca || '-'}</TableCell>
                  <TableCell align="right">{item.cantidad}</TableCell>
                  <TableCell align="right">${item.precioVentaPactado.toFixed(2)}</TableCell>
                  <TableCell align="right">${(item.cantidad * item.precioVentaPactado).toFixed(2)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <RoleGuard allowedRoles={['SUPERADMIN', 'ADMIN']}>
            <Button
              color="error"
              variant="contained"
              onClick={handleCancelarVenta}
              disabled={!selectedVenta || selectedVenta.estado === 'CANCELADA' || loadingCancel}
            >
              Cancelar Venta
            </Button>
          </RoleGuard>
          <Button onClick={() => setSelectedVenta(null)}>Cerrar</Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

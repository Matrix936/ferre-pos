import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Box,
  Button,
  CircularProgress,
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
import { Print as PrintIcon } from '@mui/icons-material';
import { RoleGuard } from '../../auth/components/RoleGuard';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { useConfig } from '../../config/context/ConfigContext';
import { TicketPreview } from './TicketPreview';

interface HistorialVenta {
  id: string;
  fecha: string;
  total: number;
  metodoPago: string;
  efectivoRecibido?: number | null;
  cambioEntregado?: number | null;
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

interface EmpresaConfigFiscal {
  rfc: string;
  razonSocial: string;
  regimenFiscal: string;
  registroPatronal?: string | null;
  actualizadoAt: string;
}

export function HistorialVentasView() {
  const { user } = useAuth();
  const { sucursales, usuarios } = useCatalogos();
  const { logo } = useConfig();
  const isSuperAdmin = user?.role === 'SUPERADMIN';
  const [ventas, setVentas] = useState<HistorialVenta[]>([]);
  const [fechaInicio, setFechaInicio] = useState('');
  const [fechaFin, setFechaFin] = useState('');
  const [sucursalId, setSucursalId] = useState('');
  const [usuarioId, setUsuarioId] = useState('');
  const [selectedVenta, setSelectedVenta] = useState<HistorialVenta | null>(null);
  const [detalle, setDetalle] = useState<HistorialVentaDetalle[]>([]);
  const [loadingCancel, setLoadingCancel] = useState(false);
  const [openCancelDialog, setOpenCancelDialog] = useState(false);
  const [motivoCancelacion, setMotivoCancelacion] = useState('');
  const [usuarioAutorizoClave, setUsuarioAutorizoClave] = useState('');
  const [cancelError, setCancelError] = useState('');
  const [printing, setPrinting] = useState(false);
  const [detailError, setDetailError] = useState('');
  const [empresaConfig, setEmpresaConfig] = useState<EmpresaConfigFiscal | null>(null);

  const fetchVentas = async () => {
    const filtro = {
      fechaInicio: fechaInicio ? `${fechaInicio}T00:00:00.000Z` : undefined,
      fechaFin: fechaFin ? `${fechaFin}T23:59:59.999Z` : undefined,
      sucursalId: isSuperAdmin ? sucursalId || undefined : user?.sucursalId,
      usuarioId: usuarioId || undefined,
    };
    const data = await invoke<HistorialVenta[]>('get_historial_ventas', { filtro });
    setVentas(data);
  };

  useEffect(() => {
    fetchVentas().catch((error) => console.error('Error historial ventas:', error));
  }, [fechaInicio, fechaFin, sucursalId, usuarioId, user?.sucursalId, isSuperAdmin]);

  useEffect(() => {
    invoke<EmpresaConfigFiscal | null>('get_empresa_config')
      .then(setEmpresaConfig)
      .catch((error) => console.error('Error configuración fiscal:', error));
  }, []);

  const openDetalle = async (venta: HistorialVenta) => {
    setSelectedVenta(venta);
    setDetalle([]);
    setDetailError('');
    try {
      const data = await invoke<HistorialVentaDetalle[]>('get_detalle_venta', { ventaId: venta.id });
      setDetalle(data);
    } catch (error) {
      setDetailError(`Error al cargar detalle: ${error}`);
    }
  };

  const handleCloseDetalle = () => {
    if (printing || loadingCancel) return;
    setSelectedVenta(null);
    setDetalle([]);
    setDetailError('');
  };

  const handleReimprimirTicket = async () => {
    if (!selectedVenta || detalle.length === 0) return;
    const printerName = window.prompt('Nombre o ruta de la impresora térmica', '\\\\localhost\\IMPRESORA_TICKETS');
    if (!printerName?.trim()) return;

    setPrinting(true);
    setDetailError('');
    try {
      await invoke('imprimir_ticket_y_abrir_caja', {
        printerName: printerName.trim(),
        paperWidth: 42,
        abrirCajon: false,
        ticket: {
          folio: selectedVenta.id.slice(0, 8).toUpperCase(),
          fecha: new Date(selectedVenta.fecha).toLocaleString(),
          cajero: selectedVenta.usuarioNombre,
          sucursal: selectedVenta.sucursalNombre,
          metodoPago: selectedVenta.metodoPago,
          productos: detalle.map((item) => ({
            descripcion: item.descripcion,
            cantidad: item.cantidad,
            precioUnitario: item.precioVentaPactado,
            importe: item.cantidad * item.precioVentaPactado,
          })),
          subtotal: selectedVenta.total / 1.16,
          descuento: 0,
          total: selectedVenta.total,
          recibido: selectedVenta.metodoPago === 'EFECTIVO' ? selectedVenta.efectivoRecibido ?? selectedVenta.total : undefined,
          cambio: selectedVenta.metodoPago === 'EFECTIVO' ? selectedVenta.cambioEntregado ?? 0 : undefined,
          mensaje: '¡Gracias por su compra!',
        },
      });
    } catch (error) {
      setDetailError(`Error al reimprimir ticket: ${error}`);
    } finally {
      setPrinting(false);
    }
  };

  const handleCancelarVenta = async () => {
    if (!selectedVenta) return;
    if (!user) {
      setCancelError('No hay una sesión activa para autorizar la cancelación.');
      return;
    }
    if (!motivoCancelacion.trim() || !usuarioAutorizoClave.trim()) {
      setCancelError('Motivo y contraseña/PIN de autorización son obligatorios.');
      return;
    }
    setLoadingCancel(true);
    setCancelError('');
    try {
      await invoke('cancelar_venta', {
        ventaId: selectedVenta.id,
        usuarioAutorizoId: user.id,
        usuarioAutorizoClave: usuarioAutorizoClave.trim(),
        motivoCancelacion: motivoCancelacion.trim(),
        fechaCancelacion: new Date().toISOString(),
      });
      const ventaActualizada = { ...selectedVenta, estado: 'CANCELADA' };
      setSelectedVenta(ventaActualizada);
      setVentas((prev) => prev.map((v) => (v.id === ventaActualizada.id ? ventaActualizada : v)));
      setOpenCancelDialog(false);
      setMotivoCancelacion('');
      setUsuarioAutorizoClave('');
    } catch (error) {
      setCancelError(`Error al cancelar venta: ${error}`);
    } finally {
      setLoadingCancel(false);
    }
  };

  const openCancelacion = () => {
    setMotivoCancelacion('');
    setUsuarioAutorizoClave('');
    setCancelError('');
    setOpenCancelDialog(true);
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
          {isSuperAdmin && (
            <TextField select label="Sucursal" value={sucursalId} onChange={(e) => setSucursalId(e.target.value)}>
              <MenuItem value="">Todas</MenuItem>
              {sucursales.map((sucursal) => (
                <MenuItem key={sucursal.id} value={sucursal.id}>{sucursal.nombre}</MenuItem>
              ))}
            </TextField>
          )}
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

      <Dialog open={Boolean(selectedVenta)} onClose={handleCloseDetalle} maxWidth="md" fullWidth>
        <DialogTitle>Detalle de Venta</DialogTitle>
        <DialogContent sx={{ '&&': { pt: 2.5 } }}>
          {detailError && <Alert severity="error" sx={{ mb: 2 }}>{detailError}</Alert>}
          {selectedVenta && (
            <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: 'minmax(320px, 380px) 1fr' }, gap: 3, alignItems: 'start' }}>
              <TicketPreview
                venta={selectedVenta}
                detalles={detalle}
                empresaNombre={empresaConfig?.razonSocial}
                rfc={empresaConfig?.rfc}
                regimenFiscal={empresaConfig?.regimenFiscal}
                codigoPostal={sucursales.find((sucursal) => sucursal.id === selectedVenta.sucursalId)?.codigoPostal}
                logoSrc={logo || undefined}
              />
              <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
                <Typography variant="subtitle1" sx={{ fontWeight: 800, mb: 1 }}>
                  Resumen de venta
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Folio completo
                </Typography>
                <Typography variant="body2" sx={{ fontWeight: 700, mb: 2, wordBreak: 'break-all' }}>
                  {selectedVenta.id}
                </Typography>
                <Typography variant="body2">Estado: <strong>{selectedVenta.estado}</strong></Typography>
                <Typography variant="body2">Sucursal: <strong>{selectedVenta.sucursalNombre}</strong></Typography>
                <Typography variant="body2">Cajero: <strong>{selectedVenta.usuarioNombre}</strong></Typography>
                <Typography variant="body2">Productos: <strong>{detalle.length}</strong></Typography>
                <Typography variant="h5" sx={{ fontWeight: 900, mt: 2 }}>
                  ${selectedVenta.total.toFixed(2)}
                </Typography>
              </Paper>
            </Box>
          )}
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <RoleGuard allowedRoles={['SUPERADMIN', 'ADMIN']}>
            <Button
              color="error"
              variant="contained"
              onClick={openCancelacion}
              disabled={!selectedVenta || selectedVenta.estado === 'CANCELADA' || loadingCancel}
              startIcon={loadingCancel ? <CircularProgress size={18} color="inherit" /> : undefined}
            >
              {loadingCancel ? 'Cancelando...' : 'Cancelar Venta'}
            </Button>
          </RoleGuard>
          <Button
            variant="outlined"
            startIcon={printing ? <CircularProgress size={18} /> : <PrintIcon />}
            onClick={handleReimprimirTicket}
            disabled={printing || loadingCancel || detalle.length === 0}
          >
            {printing ? 'Reimprimiendo...' : 'Reimprimir Ticket'}
          </Button>
          <Button onClick={handleCloseDetalle} disabled={loadingCancel || printing}>Cerrar</Button>
        </DialogActions>
      </Dialog>

      <Dialog open={openCancelDialog} onClose={loadingCancel ? undefined : () => setOpenCancelDialog(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Autorizar cancelación</DialogTitle>
        <DialogContent sx={{ '&&': { pt: 2.5 }, display: 'flex', flexDirection: 'column', gap: 2 }}>
          <Alert severity="warning">
            La venta se cancelará, el stock regresará a la sucursal y se registrará la salida de efectivo en caja.
          </Alert>
          {cancelError && <Alert severity="error">{cancelError}</Alert>}
          <TextField
            label="Motivo de cancelación"
            value={motivoCancelacion}
            onChange={(e) => setMotivoCancelacion(e.target.value)}
            multiline
            minRows={3}
            autoFocus
          />
          <TextField
            label="Contraseña/PIN autorizador"
            type="password"
            value={usuarioAutorizoClave}
            onChange={(e) => setUsuarioAutorizoClave(e.target.value)}
          />
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <Button onClick={() => setOpenCancelDialog(false)} disabled={loadingCancel}>Cancelar</Button>
          <Button
            color="error"
            variant="contained"
            onClick={handleCancelarVenta}
            disabled={loadingCancel || !motivoCancelacion.trim() || !usuarioAutorizoClave.trim()}
            startIcon={loadingCancel ? <CircularProgress size={18} color="inherit" /> : undefined}
          >
            {loadingCancel ? 'Cancelando...' : 'Confirmar cancelación'}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

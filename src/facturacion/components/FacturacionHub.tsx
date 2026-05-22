import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Box,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Typography,
} from '@mui/material';
import { FactCheck as FacturarIcon, Verified as TimbrarIcon } from '@mui/icons-material';

interface HistorialVenta {
  id: string;
  fecha: string;
  total: number;
  metodoPago: string;
  estado: string;
  sucursalNombre: string;
  usuarioNombre: string;
  clienteNombre?: string;
}

interface FacturaEmitida {
  id: string;
  ventaId: string;
  uuid?: string | null;
  rfcReceptor: string;
  montoTotal: number;
  estado: 'PENDIENTE' | 'TIMBRADA' | 'CANCELADA';
  fechaEmision: string;
}

type FacturaPayload = Record<string, unknown>;

function buildTestUuid() {
  return `TEST-${crypto.randomUUID().toUpperCase()}`;
}

export function FacturacionHub() {
  const [ventas, setVentas] = useState<HistorialVenta[]>([]);
  const [facturas, setFacturas] = useState<FacturaEmitida[]>([]);
  const [selectedVenta, setSelectedVenta] = useState<HistorialVenta | null>(null);
  const [payload, setPayload] = useState<FacturaPayload | null>(null);
  const [error, setError] = useState('');
  const [loadingVentaId, setLoadingVentaId] = useState('');
  const [loadingTimbrado, setLoadingTimbrado] = useState(false);

  const fetchData = async () => {
    const [ventasData, facturasData] = await Promise.all([
      invoke<HistorialVenta[]>('get_historial_ventas', { filtro: {} }),
      invoke<FacturaEmitida[]>('get_facturas_emitidas'),
    ]);
    setVentas(ventasData);
    setFacturas(facturasData);
  };

  useEffect(() => {
    fetchData().catch((err) => setError(String(err)));
  }, []);

  const ventasFacturables = useMemo(
    () => ventas.filter((venta) => venta.estado === 'COMPLETADA').slice(0, 30),
    [ventas],
  );

  const handleGenerarFactura = async (venta: HistorialVenta) => {
    setError('');
    setLoadingVentaId(venta.id);
    try {
      const data = await invoke<FacturaPayload>('get_payload_factura', { ventaId: venta.id });
      setSelectedVenta(venta);
      setPayload(data);
      await fetchData();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoadingVentaId('');
    }
  };

  const handleSimularTimbrado = async () => {
    if (!selectedVenta) return;
    setLoadingTimbrado(true);
    setError('');
    try {
      await invoke('actualizar_estado_factura', {
        input: {
          facturaId: `FAC-${selectedVenta.id}`,
          uuid: buildTestUuid(),
          pdfPath: null,
          xmlPath: null,
        },
      });
      setPayload(null);
      setSelectedVenta(null);
      await fetchData();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoadingTimbrado(false);
    }
  };

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3 }}>
        Facturación electrónica
      </Typography>

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden', mb: 3 }}>
        <Box sx={{ p: 2, borderBottom: '1px solid', borderColor: 'divider' }}>
          <Typography variant="h6" sx={{ fontWeight: 700 }}>Ventas completadas</Typography>
        </Box>
        <TableContainer>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Folio venta</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Fecha</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Cliente</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Sucursal</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Total</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acción</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {ventasFacturables.map((venta) => (
                <TableRow key={venta.id} hover>
                  <TableCell>{venta.id.slice(0, 8)}</TableCell>
                  <TableCell>{new Date(venta.fecha).toLocaleString()}</TableCell>
                  <TableCell>{venta.clienteNombre || 'Sin cliente'}</TableCell>
                  <TableCell>{venta.sucursalNombre}</TableCell>
                  <TableCell>${venta.total.toFixed(2)}</TableCell>
                  <TableCell align="right">
                    <Button
                      size="small"
                      variant="outlined"
                      startIcon={<FacturarIcon />}
                      onClick={() => handleGenerarFactura(venta)}
                      disabled={loadingVentaId === venta.id}
                    >
                      Generar Factura
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
              {ventasFacturables.length === 0 && (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay ventas completadas para facturar.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <Box sx={{ p: 2, borderBottom: '1px solid', borderColor: 'divider' }}>
          <Typography variant="h6" sx={{ fontWeight: 700 }}>Facturas emitidas</Typography>
        </Box>
        <TableContainer>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Folio</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>RFC cliente</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Monto</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Estado</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>UUID</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {facturas.map((factura) => (
                <TableRow key={factura.id} hover>
                  <TableCell>{factura.id}</TableCell>
                  <TableCell>{factura.rfcReceptor}</TableCell>
                  <TableCell>${factura.montoTotal.toFixed(2)}</TableCell>
                  <TableCell>
                    <Chip
                      label={factura.estado}
                      size="small"
                      color={factura.estado === 'TIMBRADA' ? 'success' : 'warning'}
                      sx={{ borderRadius: '6px', fontWeight: 700 }}
                    />
                  </TableCell>
                  <TableCell>{factura.uuid || '-'}</TableCell>
                </TableRow>
              ))}
              {facturas.length === 0 && (
                <TableRow>
                  <TableCell colSpan={5} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    Aún no hay facturas emitidas.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Dialog open={Boolean(payload)} onClose={() => setPayload(null)} maxWidth="md" fullWidth>
        <DialogTitle>Payload CFDI 4.0</DialogTitle>
        <DialogContent>
          <Box
            component="pre"
            sx={{
              m: 0,
              p: 2,
              borderRadius: 1,
              bgcolor: 'background.default',
              overflow: 'auto',
              maxHeight: 520,
              fontSize: 13,
            }}
          >
            {JSON.stringify(payload, null, 2)}
          </Box>
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <Button onClick={() => setPayload(null)}>Cerrar</Button>
          <Button
            variant="contained"
            startIcon={<TimbrarIcon />}
            onClick={handleSimularTimbrado}
            disabled={loadingTimbrado}
            disableElevation
          >
            Simular Timbrado
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Box,
  Button,
  Chip,
  CircularProgress,
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
import { AsyncButton } from '../../shared/components/AsyncButton';
import { TablePager } from '../../shared/components/TablePager';
import { useDialogHotkeys } from '../../shared/hooks/useDialogHotkeys';
import { dialogActionsSx, dialogContentSx } from '../../shared/ui/patterns';

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

interface HistorialVentasPage {
  rows: HistorialVenta[];
  total: number;
}

interface FacturasEmitidasPage {
  rows: FacturaEmitida[];
  total: number;
}

function buildTestUuid() {
  return `TEST-${crypto.randomUUID().toUpperCase()}`;
}

export function FacturacionHub() {
  const [ventas, setVentas] = useState<HistorialVenta[]>([]);
  const [facturas, setFacturas] = useState<FacturaEmitida[]>([]);
  const [facturasVentas, setFacturasVentas] = useState<FacturaEmitida[]>([]);
  const [selectedVenta, setSelectedVenta] = useState<HistorialVenta | null>(null);
  const [payload, setPayload] = useState<FacturaPayload | null>(null);
  const [error, setError] = useState('');
  const [loadingVentaId, setLoadingVentaId] = useState('');
  const [loadingTimbrado, setLoadingTimbrado] = useState(false);
  const [ventasPage, setVentasPage] = useState(0);
  const [ventasPageSize, setVentasPageSizeState] = useState(10);
  const [ventasTotalRows, setVentasTotalRows] = useState(0);
  const [facturasPage, setFacturasPage] = useState(0);
  const [facturasPageSize, setFacturasPageSizeState] = useState(10);
  const [facturasTotalRows, setFacturasTotalRows] = useState(0);

  const fetchData = async () => {
    const [ventasData, facturasData] = await Promise.all([
      invoke<HistorialVentasPage>('get_historial_ventas_page', {
        filtro: { estado: 'COMPLETADA' },
        page: ventasPage,
        pageSize: ventasPageSize,
      }),
      invoke<FacturasEmitidasPage>('get_facturas_emitidas_page', {
        page: facturasPage,
        pageSize: facturasPageSize,
      }),
    ]);
    const facturasVentasData = ventasData.rows.length > 0
      ? await invoke<FacturaEmitida[]>('get_facturas_por_ventas', {
        ventaIds: ventasData.rows.map((venta) => venta.id),
      })
      : [];
    setVentas(ventasData.rows);
    setVentasTotalRows(ventasData.total);
    setFacturas(facturasData.rows);
    setFacturasVentas(facturasVentasData);
    setFacturasTotalRows(facturasData.total);
  };

  useEffect(() => {
    fetchData().catch((err) => setError(String(err)));
  }, [ventasPage, ventasPageSize, facturasPage, facturasPageSize]);

  const facturaPorVenta = useMemo(
    () => new Map([...facturas, ...facturasVentas].map((factura) => [factura.ventaId, factura])),
    [facturas, facturasVentas],
  );
  const ventasTotalPages = Math.max(1, Math.ceil(ventasTotalRows / ventasPageSize));
  const ventasFromRow = ventasTotalRows === 0 ? 0 : ventasPage * ventasPageSize + 1;
  const ventasToRow = Math.min((ventasPage + 1) * ventasPageSize, ventasTotalRows);
  const setVentasPageSize = (value: number) => {
    setVentasPageSizeState(value);
    setVentasPage(0);
  };
  const facturasTotalPages = Math.max(1, Math.ceil(facturasTotalRows / facturasPageSize));
  const facturasFromRow = facturasTotalRows === 0 ? 0 : facturasPage * facturasPageSize + 1;
  const facturasToRow = Math.min((facturasPage + 1) * facturasPageSize, facturasTotalRows);
  const setFacturasPageSize = (value: number) => {
    setFacturasPageSizeState(value);
    setFacturasPage(0);
  };

  const handleGenerarFactura = async (venta: HistorialVenta) => {
    const factura = facturaPorVenta.get(venta.id);
    if (factura?.estado === 'TIMBRADA') {
      setError('Esta venta ya tiene una factura timbrada.');
      return;
    }
    if (factura?.estado === 'CANCELADA') {
      setError('Esta venta tiene una factura cancelada. Revisa el historial antes de generar otra.');
      return;
    }
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

  useDialogHotkeys({
    open: Boolean(payload),
    disabled: loadingTimbrado,
    cancelDisabled: loadingTimbrado,
    onConfirm: handleSimularTimbrado,
    onCancel: () => setPayload(null),
  });

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3 }}>
        Facturación electrónica
      </Typography>

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden', mb: 3 }}>
        <Box sx={{ p: 2, borderBottom: '1px solid', borderColor: 'divider' }}>
          <Typography variant="h6" sx={{ fontWeight: 700 }}>Ventas completadas</Typography>
          <Typography variant="body2" color="text.secondary">
            Solo se puede timbrar una vez por venta. Las ventas con CFDI timbrado quedan protegidas contra duplicados.
          </Typography>
        </Box>
        <TableContainer>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Folio venta</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Fecha</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Cliente</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Sucursal</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Pago</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Factura</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Total</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Acción</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {ventas.map((venta) => {
                const factura = facturaPorVenta.get(venta.id);
                const yaTimbrada = factura?.estado === 'TIMBRADA';
                const facturaCancelada = factura?.estado === 'CANCELADA';
                return (
                  <TableRow key={venta.id} hover>
                    <TableCell>{venta.id.slice(0, 8)}</TableCell>
                    <TableCell>{new Date(venta.fecha).toLocaleString()}</TableCell>
                    <TableCell>{venta.clienteNombre || 'Sin cliente'}</TableCell>
                    <TableCell>{venta.sucursalNombre}</TableCell>
                    <TableCell>{venta.metodoPago}</TableCell>
                    <TableCell>
                      <Chip
                        label={factura?.estado ?? 'SIN CFDI'}
                        size="small"
                        color={yaTimbrada ? 'success' : facturaCancelada ? 'error' : factura ? 'warning' : 'default'}
                        sx={{ borderRadius: '6px', fontWeight: 700 }}
                      />
                    </TableCell>
                    <TableCell>${venta.total.toFixed(2)}</TableCell>
                    <TableCell>
                      <Button
                        size="small"
                        variant="outlined"
                        startIcon={loadingVentaId === venta.id ? <CircularProgress size={16} /> : <FacturarIcon />}
                        onClick={() => handleGenerarFactura(venta)}
                        disabled={Boolean(loadingVentaId) || yaTimbrada || facturaCancelada}
                      >
                        {loadingVentaId === venta.id ? 'Generando...' : factura ? 'Ver payload' : 'Generar Factura'}
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
              {ventas.length === 0 && (
                <TableRow>
                  <TableCell colSpan={8} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay ventas completadas para facturar.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
        <TablePager
          page={ventasPage}
          pageSize={ventasPageSize}
          totalPages={ventasTotalPages}
          totalRows={ventasTotalRows}
          fromRow={ventasFromRow}
          toRow={ventasToRow}
          canPreviousPage={ventasPage > 0}
          canNextPage={ventasPage + 1 < ventasTotalPages}
          onPreviousPage={() => setVentasPage((prev) => Math.max(0, prev - 1))}
          onNextPage={() => setVentasPage((prev) => Math.min(ventasTotalPages - 1, prev + 1))}
          onPageSizeChange={setVentasPageSize}
          rowLabel="ventas"
        />
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
                      color={factura.estado === 'TIMBRADA' ? 'success' : factura.estado === 'CANCELADA' ? 'error' : 'warning'}
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
        <TablePager
          page={facturasPage}
          pageSize={facturasPageSize}
          totalPages={facturasTotalPages}
          totalRows={facturasTotalRows}
          fromRow={facturasFromRow}
          toRow={facturasToRow}
          canPreviousPage={facturasPage > 0}
          canNextPage={facturasPage + 1 < facturasTotalPages}
          onPreviousPage={() => setFacturasPage((prev) => Math.max(0, prev - 1))}
          onNextPage={() => setFacturasPage((prev) => Math.min(facturasTotalPages - 1, prev + 1))}
          onPageSizeChange={setFacturasPageSize}
          rowLabel="facturas"
        />
      </Paper>

      <Dialog open={Boolean(payload)} onClose={loadingTimbrado ? undefined : () => setPayload(null)} maxWidth="md" fullWidth>
        <DialogTitle>Payload CFDI 4.0</DialogTitle>
        <DialogContent sx={dialogContentSx}>
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
        <DialogActions sx={dialogActionsSx}>
          <Button onClick={() => setPayload(null)} disabled={loadingTimbrado}>Cerrar</Button>
          <AsyncButton
            variant="contained"
            startIcon={<TimbrarIcon />}
            onClick={handleSimularTimbrado}
            disabled={loadingTimbrado}
            loading={loadingTimbrado}
            loadingText="Timbrando..."
            disableElevation
          >
            Simular Timbrado
          </AsyncButton>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

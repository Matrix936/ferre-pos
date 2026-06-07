import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Autocomplete,
  Box,
  Button,
  Chip,
  CircularProgress,
  MenuItem,
  Paper,
  Stack,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from '@mui/material';
import { Delete as DeleteIcon, LocalShipping as TraspasoIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { ProductoInventario } from '../../inventario/types';
import { AsyncButton } from '../../shared/components/AsyncButton';
import { FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { TablePager } from '../../shared/components/TablePager';
import { useDebouncedValue } from '../../shared/hooks/useDebouncedValue';
import { useFeedback } from '../../shared/hooks/useFeedback';
import { useLocalPagination } from '../../shared/hooks/useLocalPagination';

interface TraspasoRow {
  productoId: string;
  descripcion: string;
  marca: string;
  stockDisponible: number;
  cantidad: string;
}

interface HistorialTraspaso {
  id: string;
  sucursalOrigenId: string;
  sucursalOrigenNombre: string;
  sucursalDestinoId: string;
  sucursalDestinoNombre: string;
  usuarioId: string;
  usuarioNombre: string;
  fecha: string;
  estado: 'EN_TRANSITO' | 'RECIBIDO' | 'RECHAZADO' | 'CANCELADO';
  usuarioRecibioId?: string | null;
  usuarioRecibioNombre?: string | null;
  fechaRecepcion?: string | null;
  observacionesRecepcion?: string | null;
}

interface HistorialTraspasosPage {
  rows: HistorialTraspaso[];
  total: number;
}

const QUANTITY_PATTERN = /^\d+(\.\d{0,3})?$/;

export function NuevoTraspasoView() {
  const { user } = useAuth();
  const { sucursales } = useCatalogos();
  const [origenId, setOrigenId] = useState(user?.sucursalId ?? '');
  const [destinoId, setDestinoId] = useState('');
  const [search, setSearch] = useState('');
  const [productosBusqueda, setProductosBusqueda] = useState<ProductoInventario[]>([]);
  const [detalle, setDetalle] = useState<TraspasoRow[]>([]);
  const [historial, setHistorial] = useState<HistorialTraspaso[]>([]);
  const [loading, setLoading] = useState(false);
  const [receivingId, setReceivingId] = useState<string | null>(null);
  const [refreshingHistorial, setRefreshingHistorial] = useState(false);
  const [pendientesPage, setPendientesPage] = useState(0);
  const [pendientesPageSize, setPendientesPageSizeState] = useState(10);
  const [pendientesTotalRows, setPendientesTotalRows] = useState(0);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();
  const searchDebounced = useDebouncedValue(search, 300);

  const fetchHistorial = async () => {
    const data = await invoke<HistorialTraspasosPage>('get_historial_traspasos_page', {
      page: pendientesPage,
      pageSize: pendientesPageSize,
      estado: 'EN_TRANSITO',
    });
    setHistorial(data.rows);
    setPendientesTotalRows(data.total);
  };

  useEffect(() => {
    fetchHistorial().catch((error) => console.error('Error historial traspasos:', error));
  }, [pendientesPage, pendientesPageSize]);

  useEffect(() => {
    if (!destinoId && sucursales.length > 0) {
      const firstDifferent = sucursales.find((s) => s.id !== (user?.sucursalId ?? ''));
      setDestinoId(firstDifferent?.id ?? '');
    }
  }, [destinoId, sucursales, user?.sucursalId]);

  useEffect(() => {
    const query = searchDebounced.trim();
    if (!origenId || query.length <= 2) {
      setProductosBusqueda([]);
      return;
    }
    let active = true;
    invoke<ProductoInventario[]>('buscar_productos_por_sucursal', { sucursalId: origenId, query })
      .then((data) => {
        if (active) setProductosBusqueda(data);
      })
      .catch((error) => {
        console.error('Error productos origen:', error);
        if (active) setProductosBusqueda([]);
      });
    return () => {
      active = false;
    };
  }, [origenId, searchDebounced]);

  useEffect(() => {
    setDetalle([]);
  }, [origenId]);

  const invalidSucursales = useMemo(
    () => !origenId || !destinoId || origenId === destinoId,
    [origenId, destinoId],
  );
  const detalleValido = detalle.every((row) => {
    const cantidad = row.cantidad.trim();
    return (
      QUANTITY_PATTERN.test(cantidad) &&
      Number(cantidad) > 0 &&
      Number(cantidad) <= row.stockDisponible
    );
  });

  const addProducto = (producto: ProductoInventario) => {
    if (detalle.some((row) => row.productoId === producto.id)) return;
    setDetalle((prev) => [
      ...prev,
      {
        productoId: producto.id,
        descripcion: producto.descripcion,
        marca: producto.marca,
        stockDisponible: producto.stock,
        cantidad: '1',
      },
    ]);
  };

  const updateCantidad = (productoId: string, cantidad: string) => {
    setDetalle((prev) => prev.map((row) => (row.productoId === productoId ? { ...row, cantidad } : row)));
  };

  const removeRow = (productoId: string) => {
    setDetalle((prev) => prev.filter((row) => row.productoId !== productoId));
  };

  const clearForm = () => {
    setSearch('');
    setDetalle([]);
  };

  const handleConfirmar = async () => {
    if (loading) return;
    if (!user?.id) return;
    if (invalidSucursales) {
      showFeedback('Selecciona sucursal origen y destino distintas.', 'warning');
      return;
    }
    if (detalle.length === 0) {
      showFeedback('Agrega al menos un producto.', 'warning');
      return;
    }
    const invalidos = detalle.find(
      (row) =>
        !QUANTITY_PATTERN.test(row.cantidad.trim()) ||
        Number(row.cantidad) <= 0 ||
        Number(row.cantidad) > row.stockDisponible,
    );
    if (invalidos) {
      showFeedback(`Revisa la cantidad de ${invalidos.descripcion}. Debe ser mayor a cero, máximo 3 decimales y no superar stock.`, 'warning');
      return;
    }
    const excedidos = detalle.find((row) => Number(row.cantidad || 0) > row.stockDisponible);
    if (excedidos) {
      showFeedback(`Cantidad excedida para ${excedidos.descripcion}.`, 'warning');
      return;
    }

    setLoading(true);
    try {
      await invoke('registrar_traspaso', {
        traspaso: {
          id: crypto.randomUUID(),
          sucursalOrigenId: origenId,
          sucursalDestinoId: destinoId,
          usuarioId: user.id,
          fecha: new Date().toISOString(),
          detalles: detalle.map((row) => ({
            id: crypto.randomUUID(),
            productoId: row.productoId,
            cantidad: Number(row.cantidad || 0),
          })),
        },
      });
      showFeedback('Traspaso registrado en tránsito. La sucursal destino debe recibirlo para sumar inventario.');
      clearForm();
      setProductosBusqueda([]);
      await fetchHistorial();
    } catch (error) {
      showFeedback(`Error al registrar traspaso: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const confirmarDisabled = loading || invalidSucursales || detalle.length === 0 || !detalleValido;

  const handleRecibirTraspaso = async (traspaso: HistorialTraspaso) => {
    if (!user?.id) return;
    const canReceive = user.role === 'SUPERADMIN' || user.sucursalId === traspaso.sucursalDestinoId;
    if (!canReceive) {
      showFeedback('Solo SUPERADMIN o la sucursal destino pueden recibir este traspaso.', 'warning');
      return;
    }

    setReceivingId(traspaso.id);
    try {
      await invoke('recibir_traspaso', {
        input: {
          traspasoId: traspaso.id,
          usuarioRecibioId: user.id,
          fechaRecepcion: new Date().toISOString(),
          observacionesRecepcion: '',
        },
      });
      showFeedback('Traspaso recibido. El inventario de la sucursal destino fue actualizado.');
      await fetchHistorial();
    } catch (error) {
      showFeedback(`Error al recibir traspaso: ${error}`, 'error');
    } finally {
      setReceivingId(null);
    }
  };

  const detallePager = useLocalPagination(detalle);
  const pendientesTotalPages = Math.max(1, Math.ceil(pendientesTotalRows / pendientesPageSize));
  const pendientesFromRow = pendientesTotalRows === 0 ? 0 : pendientesPage * pendientesPageSize + 1;
  const pendientesToRow = Math.min((pendientesPage + 1) * pendientesPageSize, pendientesTotalRows);
  const setPendientesPageSize = (value: number) => {
    setPendientesPageSizeState(value);
    setPendientesPage(0);
  };

  const handleRefreshHistorial = async () => {
    setRefreshingHistorial(true);
    try {
      await fetchHistorial();
    } catch (error) {
      showFeedback(`Error al actualizar: ${error}`, 'error');
    } finally {
      setRefreshingHistorial(false);
    }
  };

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3 }}>
        Traspaso entre Sucursales
      </Typography>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'grid', gap: 2, gridTemplateColumns: { xs: '1fr', md: '1fr 1fr 2fr' } }}>
          <TextField select label="Sucursal origen" value={origenId} onChange={(e) => setOrigenId(e.target.value)}>
            {sucursales.map((sucursal) => (
              <MenuItem key={sucursal.id} value={sucursal.id}>{sucursal.nombre}</MenuItem>
            ))}
          </TextField>
          <TextField
            select
            label="Sucursal destino"
            value={destinoId}
            onChange={(e) => setDestinoId(e.target.value)}
            error={Boolean(destinoId) && destinoId === origenId}
            helperText={destinoId === origenId ? 'Debe ser una sucursal distinta.' : ''}
          >
            {sucursales.map((sucursal) => (
              <MenuItem key={sucursal.id} value={sucursal.id}>{sucursal.nombre}</MenuItem>
            ))}
          </TextField>
          <Autocomplete
            freeSolo
            options={productosBusqueda}
            getOptionLabel={(option) => (typeof option === 'string' ? option : option.descripcion)}
            filterOptions={(options) => options}
            noOptionsText="Escribe al menos 3 caracteres para buscar coincidencias"
            inputValue={search}
            onInputChange={(_, value, reason) => {
              if (reason === 'reset') {
                setSearch('');
                setProductosBusqueda([]);
                return;
              }
              setSearch(value);
              if (reason !== 'input' || !value.trim()) {
                setProductosBusqueda([]);
              }
            }}
            onChange={(_, value) => {
              if (!value) return;
              if (typeof value === 'string') {
                const selected = productosBusqueda.find((item) =>
                  item.descripcion.toLowerCase().includes(value.trim().toLowerCase()),
                );
                if (selected) addProducto(selected);
              } else {
                addProducto(value);
              }
              setSearch('');
              setProductosBusqueda([]);
            }}
            renderOption={(props, option) => (
              <Box component="li" {...props}>
                <Box sx={{ display: 'flex', justifyContent: 'space-between', width: '100%', gap: 2 }}>
                  <Box>
                    <Typography variant="body2" sx={{ fontWeight: 600 }}>
                      {option.descripcion}
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      {option.marca || 'Sin marca'}
                    </Typography>
                  </Box>
                  <Typography variant="body2" sx={{ fontWeight: 700 }}>
                    Stock: {option.stock}
                  </Typography>
                </Box>
              </Box>
            )}
            renderInput={(params) => (
              <TextField {...params} label="Buscar producto por descripción, código o clave" fullWidth />
            )}
          />
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Descripción</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Marca</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Stock disponible</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Cantidad a enviar</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {detallePager.paginatedRows.map((row) => {
                const cantidad = Number(row.cantidad || 0);
                const invalidFormat = Boolean(row.cantidad) && !QUANTITY_PATTERN.test(row.cantidad.trim());
                const nonPositive = Boolean(row.cantidad) && cantidad <= 0;
                const exceeds = cantidad > row.stockDisponible;
                const hasError = invalidFormat || nonPositive || exceeds;
                return (
                  <TableRow key={row.productoId} hover>
                    <TableCell>{row.descripcion}</TableCell>
                    <TableCell>{row.marca || '-'}</TableCell>
                    <TableCell>{row.stockDisponible}</TableCell>
                    <TableCell sx={{ width: 220 }}>
                      <TextField
                        type="number"
                        size="small"
                        value={row.cantidad}
                        onChange={(e) => updateCantidad(row.productoId, e.target.value)}
                        error={hasError}
                        helperText={
                          invalidFormat || nonPositive
                            ? 'Mayor a 0, máximo 3 decimales.'
                            : exceeds
                              ? 'Supera stock disponible'
                              : ' '
                        }
                        slotProps={{ htmlInput: { min: 0.001, step: '0.001', max: row.stockDisponible, inputMode: 'decimal' } }}
                      />
                    </TableCell>
                    <TableCell>
                      <Button color="error" size="small" startIcon={<DeleteIcon />} onClick={() => removeRow(row.productoId)}>
                        Quitar
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
              {detalle.length === 0 && (
                <TableRow>
                  <TableCell colSpan={5} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    Agrega productos para preparar el traspaso.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
        <TablePager
          page={detallePager.page}
          pageSize={detallePager.pageSize}
          totalPages={detallePager.totalPages}
          totalRows={detallePager.totalRows}
          fromRow={detallePager.fromRow}
          toRow={detallePager.toRow}
          canPreviousPage={detallePager.canPreviousPage}
          canNextPage={detallePager.canNextPage}
          onPreviousPage={detallePager.previousPage}
          onNextPage={detallePager.nextPage}
          onPageSizeChange={detallePager.setPageSize}
          rowLabel="productos"
        />
      </Paper>

      <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 2 }}>
        <AsyncButton
          variant="contained"
          startIcon={<TraspasoIcon />}
          onClick={handleConfirmar}
          disabled={confirmarDisabled}
          loading={loading}
          loadingText="Registrando..."
        >
          Confirmar Traspaso
        </AsyncButton>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mt: 3 }}>
        <Stack
          direction={{ xs: 'column', sm: 'row' }}
          spacing={1.5}
          sx={{ mb: 2, justifyContent: 'space-between', alignItems: { xs: 'flex-start', sm: 'center' } }}
        >
          <Box>
            <Typography variant="h6" sx={{ fontWeight: 700 }}>
              Traspasos pendientes de recepción
            </Typography>
            <Typography variant="body2" color="text.secondary">
              La mercancía en tránsito no aparece en el inventario destino hasta confirmarse aquí.
            </Typography>
          </Box>
          <Button
            variant="outlined"
            size="small"
            onClick={handleRefreshHistorial}
            disabled={refreshingHistorial}
            startIcon={refreshingHistorial ? <CircularProgress size={16} /> : undefined}
          >
            {refreshingHistorial ? 'Actualizando...' : 'Actualizar'}
          </Button>
        </Stack>

        <TableContainer>
          <Table size="small">
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Folio</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Origen</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Destino</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Fecha salida</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Registró</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Estado</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Acción</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {historial.map((traspaso) => {
                const canReceive = user?.role === 'SUPERADMIN' || user?.sucursalId === traspaso.sucursalDestinoId;
                return (
                  <TableRow key={traspaso.id} hover>
                    <TableCell>{traspaso.id.slice(0, 8)}</TableCell>
                    <TableCell>{traspaso.sucursalOrigenNombre}</TableCell>
                    <TableCell>{traspaso.sucursalDestinoNombre}</TableCell>
                    <TableCell>{new Date(traspaso.fecha).toLocaleString()}</TableCell>
                    <TableCell>{traspaso.usuarioNombre}</TableCell>
                    <TableCell>
                      <Chip label="En tránsito" color="warning" size="small" />
                    </TableCell>
                    <TableCell>
                      <Button
                        variant="contained"
                        size="small"
                        onClick={() => handleRecibirTraspaso(traspaso)}
                        disabled={!canReceive || receivingId === traspaso.id}
                        startIcon={receivingId === traspaso.id ? <CircularProgress size={16} color="inherit" /> : undefined}
                      >
                        {receivingId === traspaso.id ? 'Recibiendo...' : 'Recibir'}
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
              {historial.length === 0 && (
                <TableRow>
                  <TableCell colSpan={7} align="center" sx={{ py: 3, color: 'text.secondary' }}>
                    No hay traspasos pendientes.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
        <TablePager
          page={pendientesPage}
          pageSize={pendientesPageSize}
          totalPages={pendientesTotalPages}
          totalRows={pendientesTotalRows}
          fromRow={pendientesFromRow}
          toRow={pendientesToRow}
          canPreviousPage={pendientesPage > 0}
          canNextPage={pendientesPage + 1 < pendientesTotalPages}
          onPreviousPage={() => setPendientesPage((prev) => Math.max(0, prev - 1))}
          onNextPage={() => setPendientesPage((prev) => Math.min(pendientesTotalPages - 1, prev + 1))}
          onPageSizeChange={setPendientesPageSize}
          rowLabel="traspasos"
        />
      </Paper>

      <FeedbackSnackbar message={feedbackMessage} severity={feedbackSeverity} onClose={closeFeedback} />
    </Box>
  );
}

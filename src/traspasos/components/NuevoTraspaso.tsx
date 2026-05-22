import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Chip,
  MenuItem,
  Paper,
  Snackbar,
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
import { useDebouncedValue } from '../../shared/hooks/useDebouncedValue';

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

export function NuevoTraspasoView() {
  const { user } = useAuth();
  const { sucursales } = useCatalogos();
  const [origenId, setOrigenId] = useState(user?.sucursalId ?? '');
  const [destinoId, setDestinoId] = useState('');
  const [search, setSearch] = useState('');
  const [productosBusqueda, setProductosBusqueda] = useState<ProductoInventario[]>([]);
  const [detalle, setDetalle] = useState<TraspasoRow[]>([]);
  const [historial, setHistorial] = useState<HistorialTraspaso[]>([]);
  const [snackbar, setSnackbar] = useState('');
  const [loading, setLoading] = useState(false);
  const [receivingId, setReceivingId] = useState<string | null>(null);
  const searchDebounced = useDebouncedValue(search, 300);

  const fetchHistorial = async () => {
    const data = await invoke<HistorialTraspaso[]>('get_historial_traspasos');
    setHistorial(data);
  };

  useEffect(() => {
    fetchHistorial().catch((error) => console.error('Error historial traspasos:', error));
  }, []);

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
    if (!user?.id) return;
    if (invalidSucursales) {
      setSnackbar('Selecciona sucursal origen y destino distintas.');
      return;
    }
    if (detalle.length === 0) {
      setSnackbar('Agrega al menos un producto.');
      return;
    }
    const excedidos = detalle.find((row) => Number(row.cantidad || 0) > row.stockDisponible);
    if (excedidos) {
      setSnackbar(`Cantidad excedida para ${excedidos.descripcion}.`);
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
      setSnackbar('Traspaso registrado en tránsito. La sucursal destino debe recibirlo para sumar inventario.');
      clearForm();
      setProductosBusqueda([]);
      await fetchHistorial();
    } catch (error) {
      setSnackbar(`Error al registrar traspaso: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleRecibirTraspaso = async (traspaso: HistorialTraspaso) => {
    if (!user?.id) return;
    const canReceive =
      user.role === 'SUPERADMIN' || user.role === 'ADMIN' || user.sucursalId === traspaso.sucursalDestinoId;
    if (!canReceive) {
      setSnackbar('Solo la sucursal destino o un administrador puede recibir este traspaso.');
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
      setSnackbar('Traspaso recibido. El inventario de la sucursal destino fue actualizado.');
      await fetchHistorial();
    } catch (error) {
      setSnackbar(`Error al recibir traspaso: ${error}`);
    } finally {
      setReceivingId(null);
    }
  };

  const pendientes = historial.filter((item) => item.estado === 'EN_TRANSITO');

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
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
            inputValue={search}
            onInputChange={(_, value, reason) => {
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
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {detalle.map((row) => {
                const cantidad = Number(row.cantidad || 0);
                const exceeds = cantidad > row.stockDisponible;
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
                        error={exceeds}
                        helperText={exceeds ? 'Supera stock disponible' : ' '}
                        slotProps={{ htmlInput: { min: 0.01, step: '0.01', max: row.stockDisponible } }}
                      />
                    </TableCell>
                    <TableCell align="right">
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
      </Paper>

      <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 2 }}>
        <Button
          variant="contained"
          startIcon={<TraspasoIcon />}
          onClick={handleConfirmar}
          disabled={loading || invalidSucursales || detalle.length === 0}
        >
          Confirmar Traspaso
        </Button>
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
          <Button variant="outlined" size="small" onClick={() => fetchHistorial().catch((error) => setSnackbar(`Error al actualizar: ${error}`))}>
            Actualizar
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
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acción</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {pendientes.map((traspaso) => {
                const canReceive =
                  user?.role === 'SUPERADMIN' || user?.role === 'ADMIN' || user?.sucursalId === traspaso.sucursalDestinoId;
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
                    <TableCell align="right">
                      <Button
                        variant="contained"
                        size="small"
                        onClick={() => handleRecibirTraspaso(traspaso)}
                        disabled={!canReceive || receivingId === traspaso.id}
                      >
                        Recibir
                      </Button>
                    </TableCell>
                  </TableRow>
                );
              })}
              {pendientes.length === 0 && (
                <TableRow>
                  <TableCell colSpan={7} align="center" sx={{ py: 3, color: 'text.secondary' }}>
                    No hay traspasos pendientes.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Snackbar open={Boolean(snackbar)} autoHideDuration={3200} onClose={() => setSnackbar('')}>
        <Alert onClose={() => setSnackbar('')} severity={snackbar.startsWith('Error') ? 'error' : 'success'} variant="filled">
          {snackbar}
        </Alert>
      </Snackbar>
    </Box>
  );
}

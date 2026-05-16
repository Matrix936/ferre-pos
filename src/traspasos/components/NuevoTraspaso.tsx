import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Box,
  Button,
  MenuItem,
  Paper,
  Snackbar,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from '@mui/material';
import { Add as AddIcon, Delete as DeleteIcon, LocalShipping as TraspasoIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { ProductoInventario } from '../../inventario/types';
import { Sucursal } from '../../sucursales/types';

interface TraspasoRow {
  productoId: string;
  descripcion: string;
  marca: string;
  stockDisponible: number;
  cantidad: string;
}

export function NuevoTraspasoView() {
  const { user } = useAuth();
  const [sucursales, setSucursales] = useState<Sucursal[]>([]);
  const [origenId, setOrigenId] = useState(user?.sucursalId ?? '');
  const [destinoId, setDestinoId] = useState('');
  const [search, setSearch] = useState('');
  const [productosBusqueda, setProductosBusqueda] = useState<ProductoInventario[]>([]);
  const [detalle, setDetalle] = useState<TraspasoRow[]>([]);
  const [snackbar, setSnackbar] = useState('');
  const [loading, setLoading] = useState(false);

  const fetchSucursales = async () => {
    const data = await invoke<Sucursal[]>('get_sucursales');
    setSucursales(data);
    if (!destinoId && data.length > 0) {
      const firstDifferent = data.find((s) => s.id !== (user?.sucursalId ?? ''));
      setDestinoId(firstDifferent?.id ?? '');
    }
  };

  const fetchProductos = async () => {
    if (!origenId) return;
    const query = search.trim();
    const data = query
      ? await invoke<ProductoInventario[]>('buscar_productos_por_sucursal', { sucursalId: origenId, query })
      : await invoke<ProductoInventario[]>('get_productos_por_sucursal', { sucursalId: origenId });
    setProductosBusqueda(data);
  };

  useEffect(() => {
    fetchSucursales().catch((error) => console.error('Error sucursales:', error));
  }, []);

  useEffect(() => {
    fetchProductos().catch((error) => console.error('Error productos origen:', error));
  }, [origenId, search]);

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
      setSnackbar('Traspaso registrado correctamente.');
      clearForm();
      fetchProductos().catch((error) => console.error('Error recargando productos:', error));
    } catch (error) {
      setSnackbar(`Error al registrar traspaso: ${error}`);
    } finally {
      setLoading(false);
    }
  };

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
          <TextField
            label="Buscar producto por descripción, código o clave"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            fullWidth
          />
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Typography variant="subtitle1" sx={{ fontWeight: 600, mb: 1.5 }}>
          Agregar productos desde sucursal origen
        </Typography>
        <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
          {productosBusqueda.slice(0, 20).map((producto) => (
            <Button key={producto.id} size="small" variant="outlined" startIcon={<AddIcon />} onClick={() => addProducto(producto)}>
              {producto.descripcion}
            </Button>
          ))}
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

      <Snackbar open={Boolean(snackbar)} autoHideDuration={3200} onClose={() => setSnackbar('')}>
        <Alert onClose={() => setSnackbar('')} severity={snackbar.startsWith('Error') ? 'error' : 'success'} variant="filled">
          {snackbar}
        </Alert>
      </Snackbar>
    </Box>
  );
}

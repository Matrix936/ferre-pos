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
import { Add as AddIcon, Delete as DeleteIcon, Save as SaveIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { Proveedor, ProductoInventario, RegistrarCompraPayload } from '../../inventario/types';
import { Sucursal } from '../../sucursales/types';

interface CompraRow {
  productoId: string;
  descripcion: string;
  marca: string;
  cantidad: string;
  precioCostoPactado: string;
}

export function NuevaCompra() {
  const { user } = useAuth();
  const [proveedores, setProveedores] = useState<Proveedor[]>([]);
  const [sucursales, setSucursales] = useState<Sucursal[]>([]);
  const [productosBusqueda, setProductosBusqueda] = useState<ProductoInventario[]>([]);
  const [detalle, setDetalle] = useState<CompraRow[]>([]);
  const [search, setSearch] = useState('');
  const [proveedorId, setProveedorId] = useState('');
  const [selectedSucursalId, setSelectedSucursalId] = useState(user?.sucursalId ?? '');
  const [snackbar, setSnackbar] = useState('');
  const [loading, setLoading] = useState(false);

  const isSuperAdmin = user?.role === 'SUPERADMIN';
  const sucursalCompraId = isSuperAdmin ? selectedSucursalId : user?.sucursalId ?? '';

  const fetchProveedores = async () => {
    const data = await invoke<Proveedor[]>('get_proveedores');
    setProveedores(data);
  };

  const fetchSucursales = async () => {
    const data = await invoke<Sucursal[]>('get_sucursales');
    setSucursales(data);
    if (!selectedSucursalId && data.length > 0) {
      setSelectedSucursalId(user?.sucursalId || data[0].id);
    }
  };

  const fetchProductos = async () => {
    if (!sucursalCompraId) return;
    const query = search.trim();
    const data = query
      ? await invoke<ProductoInventario[]>('buscar_productos_por_sucursal', { sucursalId: sucursalCompraId, query })
      : await invoke<ProductoInventario[]>('get_productos_por_sucursal', { sucursalId: sucursalCompraId });
    setProductosBusqueda(data);
  };

  useEffect(() => {
    fetchProveedores().catch((error) => console.error('Error proveedores:', error));
    fetchSucursales().catch((error) => console.error('Error sucursales:', error));
  }, []);

  useEffect(() => {
    fetchProductos().catch((error) => console.error('Error productos:', error));
  }, [search, sucursalCompraId]);

  const total = useMemo(
    () =>
      detalle.reduce((acc, row) => {
        const cantidad = Number(row.cantidad || 0);
        const costo = Number(row.precioCostoPactado || 0);
        return acc + cantidad * costo;
      }, 0),
    [detalle],
  );

  const addProducto = (producto: ProductoInventario) => {
    const exists = detalle.some((row) => row.productoId === producto.id);
    if (exists) return;
    setDetalle((prev) => [
      ...prev,
      {
        productoId: producto.id,
        descripcion: producto.descripcion,
        marca: producto.marca,
        cantidad: '1',
        precioCostoPactado: String(producto.precioCosto ?? 0),
      },
    ]);
  };

  const updateRow = (productoId: string, field: 'cantidad' | 'precioCostoPactado', value: string) => {
    setDetalle((prev) => prev.map((row) => (row.productoId === productoId ? { ...row, [field]: value } : row)));
  };

  const removeRow = (productoId: string) => {
    setDetalle((prev) => prev.filter((row) => row.productoId !== productoId));
  };

  const clearForm = () => {
    setSearch('');
    setDetalle([]);
    setProveedorId('');
  };

  const handleRegistrar = async () => {
    if (!proveedorId || !sucursalCompraId || detalle.length === 0) return;
    setLoading(true);
    try {
      const payload: RegistrarCompraPayload = {
        id: crypto.randomUUID(),
        proveedorId,
        sucursalId: sucursalCompraId,
        fecha: new Date().toISOString(),
        detalles: detalle.map((row) => ({
          id: crypto.randomUUID(),
          productoId: row.productoId,
          cantidad: Number(row.cantidad || 0),
          precioCostoPactado: Number(row.precioCostoPactado || 0),
        })),
      };
      await invoke('registrar_compra', { compra: payload });
      clearForm();
      setSnackbar('Entrada registrada correctamente.');
      fetchProductos().catch((error) => console.error('Error productos:', error));
    } catch (error) {
      console.error('Error al registrar compra:', error);
      setSnackbar(`Error al registrar compra: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3 }}>
        Compras (Entradas de Almacén)
      </Typography>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'grid', gap: 2, gridTemplateColumns: { xs: '1fr', md: '1fr 1fr 2fr' } }}>
          <TextField select label="Proveedor" value={proveedorId} onChange={(e) => setProveedorId(e.target.value)} required>
            {proveedores.map((proveedor) => (
              <MenuItem key={proveedor.id} value={proveedor.id}>
                {proveedor.nombre}
              </MenuItem>
            ))}
          </TextField>
          {isSuperAdmin ? (
            <TextField select label="Sucursal" value={selectedSucursalId} onChange={(e) => setSelectedSucursalId(e.target.value)}>
              {sucursales.map((sucursal) => (
                <MenuItem key={sucursal.id} value={sucursal.id}>
                  {sucursal.nombre}
                </MenuItem>
              ))}
            </TextField>
          ) : (
            <TextField label="Sucursal" value={sucursales.find((s) => s.id === sucursalCompraId)?.nombre || ''} disabled />
          )}
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
          Agregar productos
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
                <TableCell sx={{ fontWeight: 600 }}>Cantidad</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Costo pactado</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Subtotal</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {detalle.map((row) => {
                const subtotal = Number(row.cantidad || 0) * Number(row.precioCostoPactado || 0);
                return (
                  <TableRow key={row.productoId} hover>
                    <TableCell>{row.descripcion}</TableCell>
                    <TableCell>{row.marca || '-'}</TableCell>
                    <TableCell sx={{ width: 140 }}>
                      <TextField
                        type="number"
                        size="small"
                        value={row.cantidad}
                        onChange={(e) => updateRow(row.productoId, 'cantidad', e.target.value)}
                        slotProps={{ htmlInput: { min: 0, step: '0.01' } }}
                      />
                    </TableCell>
                    <TableCell sx={{ width: 180 }}>
                      <TextField
                        type="number"
                        size="small"
                        value={row.precioCostoPactado}
                        onChange={(e) => updateRow(row.productoId, 'precioCostoPactado', e.target.value)}
                        slotProps={{ htmlInput: { min: 0, step: '0.01' } }}
                      />
                    </TableCell>
                    <TableCell>${subtotal.toFixed(2)}</TableCell>
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
                  <TableCell colSpan={6} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    Agrega productos para registrar la entrada.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mt: 2 }}>
        <Typography variant="h6" sx={{ fontWeight: 700 }}>
          Total: ${total.toFixed(2)}
        </Typography>
        <Button
          variant="contained"
          startIcon={<SaveIcon />}
          onClick={handleRegistrar}
          disabled={loading || !proveedorId || !sucursalCompraId || detalle.length === 0}
        >
          Registrar entrada
        </Button>
      </Box>

      <Snackbar open={Boolean(snackbar)} autoHideDuration={3000} onClose={() => setSnackbar('')}>
        <Alert onClose={() => setSnackbar('')} severity={snackbar.startsWith('Error') ? 'error' : 'success'} variant="filled">
          {snackbar}
        </Alert>
      </Snackbar>
    </Box>
  );
}

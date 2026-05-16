import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Box,
  Button,
  MenuItem,
  Paper,
  Snackbar,
  TextField,
  Typography,
} from '@mui/material';
import { BuildCircle as AjusteIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { ProductoInventario } from '../types';
import { Sucursal } from '../../sucursales/types';

const motivosSugeridos = [
  'Defecto de fábrica',
  'Daño por humedad',
  'Robo/Extravío',
  'Caducidad',
  'Ajuste por conteo físico',
];

export function MermasAjustesView() {
  const { user } = useAuth();
  const [sucursales, setSucursales] = useState<Sucursal[]>([]);
  const [selectedSucursalId, setSelectedSucursalId] = useState(user?.sucursalId ?? '');
  const [search, setSearch] = useState('');
  const [productos, setProductos] = useState<ProductoInventario[]>([]);
  const [productoId, setProductoId] = useState('');
  const [tipoMovimiento, setTipoMovimiento] = useState<'MERMA' | 'AJUSTE'>('MERMA');
  const [motivo, setMotivo] = useState(motivosSugeridos[0]);
  const [cantidad, setCantidad] = useState('');
  const [snackbar, setSnackbar] = useState('');
  const [loading, setLoading] = useState(false);

  const isSuperAdmin = user?.role === 'SUPERADMIN';
  const sucursalTrabajo = isSuperAdmin ? selectedSucursalId : user?.sucursalId ?? '';
  const productoSeleccionado = productos.find((item) => item.id === productoId) || null;

  const fetchSucursales = async () => {
    const data = await invoke<Sucursal[]>('get_sucursales');
    setSucursales(data);
    if (!selectedSucursalId && data.length > 0) {
      setSelectedSucursalId(user?.sucursalId || data[0].id);
    }
  };

  const fetchProductos = async () => {
    if (!sucursalTrabajo) return;
    const query = search.trim();
    const data = query
      ? await invoke<ProductoInventario[]>('buscar_productos_por_sucursal', { sucursalId: sucursalTrabajo, query })
      : await invoke<ProductoInventario[]>('get_productos_por_sucursal', { sucursalId: sucursalTrabajo });
    setProductos(data);
  };

  useEffect(() => {
    fetchSucursales().catch((error) => console.error('Error sucursales:', error));
  }, []);

  useEffect(() => {
    fetchProductos().catch((error) => console.error('Error productos:', error));
  }, [sucursalTrabajo, search]);

  const clearForm = () => {
    setProductoId('');
    setCantidad('');
    setTipoMovimiento('MERMA');
    setMotivo(motivosSugeridos[0]);
  };

  const handleRegistrar = async () => {
    if (!user?.id || !sucursalTrabajo || !productoId) {
      setSnackbar('Completa sucursal y producto.');
      return;
    }
    const qty = Number(cantidad || 0);
    if (qty <= 0) {
      setSnackbar('La cantidad debe ser mayor a cero.');
      return;
    }
    if (productoSeleccionado && qty > productoSeleccionado.stock) {
      setSnackbar('La cantidad supera el stock disponible.');
      return;
    }

    setLoading(true);
    try {
      await invoke('registrar_merma_ajuste', {
        movimiento: {
          id: crypto.randomUUID(),
          productoId,
          sucursalId: sucursalTrabajo,
          usuarioId: user.id,
          cantidad: qty,
          tipoMovimiento,
          motivo,
          fecha: new Date().toISOString(),
        },
      });
      setSnackbar('Ajuste registrado correctamente.');
      clearForm();
      fetchProductos().catch((error) => console.error('Error recargando inventario:', error));
    } catch (error) {
      setSnackbar(`Error al registrar ajuste: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Box sx={{ maxWidth: 1000, mx: 'auto', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3 }}>
        Registro de Merma y Ajustes
      </Typography>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'grid', gap: 2, gridTemplateColumns: { xs: '1fr', md: '1fr 2fr' } }}>
          {isSuperAdmin ? (
            <TextField select label="Sucursal" value={selectedSucursalId} onChange={(e) => setSelectedSucursalId(e.target.value)}>
              {sucursales.map((sucursal) => (
                <MenuItem key={sucursal.id} value={sucursal.id}>{sucursal.nombre}</MenuItem>
              ))}
            </TextField>
          ) : (
            <TextField label="Sucursal" value={sucursales.find((s) => s.id === sucursalTrabajo)?.nombre || ''} disabled />
          )}
          <TextField
            label="Buscar producto por descripción, código o clave"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            fullWidth
          />
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
        <Box sx={{ display: 'grid', gap: 2, gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' } }}>
          <TextField
            select
            label="Producto"
            value={productoId}
            onChange={(e) => setProductoId(e.target.value)}
            fullWidth
          >
            {productos.map((producto) => (
              <MenuItem key={producto.id} value={producto.id}>
                {producto.descripcion}
              </MenuItem>
            ))}
          </TextField>
          <TextField
            label="Stock actual"
            value={productoSeleccionado ? productoSeleccionado.stock : ''}
            disabled
            fullWidth
          />

          <TextField select label="Tipo de movimiento" value={tipoMovimiento} onChange={(e) => setTipoMovimiento(e.target.value as 'MERMA' | 'AJUSTE')}>
            <MenuItem value="MERMA">MERMA</MenuItem>
            <MenuItem value="AJUSTE">AJUSTE</MenuItem>
          </TextField>

          <TextField
            label="Cantidad"
            type="number"
            value={cantidad}
            onChange={(e) => setCantidad(e.target.value)}
            fullWidth
            slotProps={{ htmlInput: { min: 0.01, step: '0.01' } }}
          />

          <TextField
            select
            label="Motivo"
            value={motivo}
            onChange={(e) => setMotivo(e.target.value)}
            fullWidth
            sx={{ gridColumn: { md: '1 / span 2' } }}
          >
            {motivosSugeridos.map((item) => (
              <MenuItem key={item} value={item}>{item}</MenuItem>
            ))}
          </TextField>
        </Box>

        <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 2 }}>
          <Button
            variant="contained"
            startIcon={<AjusteIcon />}
            onClick={handleRegistrar}
            disabled={loading || !productoId || !cantidad || !motivo.trim()}
          >
            Registrar Ajuste
          </Button>
        </Box>
      </Paper>

      <Snackbar open={Boolean(snackbar)} autoHideDuration={3200} onClose={() => setSnackbar('')}>
        <Alert onClose={() => setSnackbar('')} severity={snackbar.startsWith('Error') ? 'error' : 'success'} variant="filled">
          {snackbar}
        </Alert>
      </Snackbar>
    </Box>
  );
}

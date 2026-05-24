import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  CircularProgress,
  MenuItem,
  Paper,
  Snackbar,
  TextField,
  Typography,
} from '@mui/material';
import { BuildCircle as AjusteIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { ProductoInventario } from '../types';
import { useDebouncedValue } from '../../shared/hooks/useDebouncedValue';

const motivosSugeridos = [
  'Defecto de fábrica',
  'Daño por humedad',
  'Robo/Extravío',
  'Caducidad',
  'Ajuste por conteo físico',
];

export function MermasAjustesView() {
  const { user } = useAuth();
  const { sucursales } = useCatalogos();
  const [selectedSucursalId, setSelectedSucursalId] = useState(user?.sucursalId ?? '');
  const [search, setSearch] = useState('');
  const [productos, setProductos] = useState<ProductoInventario[]>([]);
  const [productoSeleccionado, setProductoSeleccionado] = useState<ProductoInventario | null>(null);
  const [tipoMovimiento, setTipoMovimiento] = useState<'MERMA' | 'AJUSTE'>('MERMA');
  const [motivo, setMotivo] = useState(motivosSugeridos[0]);
  const [cantidad, setCantidad] = useState('');
  const [snackbar, setSnackbar] = useState('');
  const [loading, setLoading] = useState(false);

  const isSuperAdmin = user?.role === 'SUPERADMIN';
  const sucursalTrabajo = isSuperAdmin ? selectedSucursalId : user?.sucursalId ?? '';
  const searchDebounced = useDebouncedValue(search, 300);

  useEffect(() => {
    if (!selectedSucursalId && sucursales.length > 0) {
      setSelectedSucursalId(user?.sucursalId || sucursales[0].id);
    }
  }, [selectedSucursalId, sucursales, user?.sucursalId]);

  useEffect(() => {
    const query = searchDebounced.trim();
    if (!sucursalTrabajo || query.length <= 2) {
      setProductos([]);
      return;
    }
    let active = true;
    invoke<ProductoInventario[]>('buscar_productos_por_sucursal', { sucursalId: sucursalTrabajo, query })
      .then((data) => {
        if (active) setProductos(data);
      })
      .catch((error) => {
        console.error('Error productos:', error);
        if (active) setProductos([]);
      });
    return () => {
      active = false;
    };
  }, [sucursalTrabajo, searchDebounced]);

  const clearForm = () => {
    setProductoSeleccionado(null);
    setSearch('');
    setProductos([]);
    setCantidad('');
    setTipoMovimiento('MERMA');
    setMotivo(motivosSugeridos[0]);
  };

  const handleRegistrar = async () => {
    if (!user?.id || !sucursalTrabajo || !productoSeleccionado?.id) {
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
          productoId: productoSeleccionado.id,
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
          <Autocomplete
            freeSolo
            options={productos}
            getOptionLabel={(option) => (typeof option === 'string' ? option : option.descripcion)}
            filterOptions={(options) => options}
            noOptionsText="Escribe al menos 3 caracteres para buscar coincidencias"
            inputValue={search}
            onInputChange={(_, value, reason) => {
              if (reason === 'reset') {
                setSearch('');
                setProductos([]);
                return;
              }
              setSearch(value);
              if (reason !== 'input' || !value.trim()) {
                setProductos([]);
              }
            }}
            onChange={(_, value) => {
              if (!value) return;
              if (typeof value === 'string') {
                const selected = productos.find((item) =>
                  item.descripcion.toLowerCase().includes(value.trim().toLowerCase()),
                );
                if (selected) {
                  setProductoSeleccionado(selected);
                }
              } else {
                setProductoSeleccionado(value);
              }
              setSearch('');
              setProductos([]);
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

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
        <Box sx={{ display: 'grid', gap: 2, gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' } }}>
          <TextField label="Producto" value={productoSeleccionado?.descripcion || ''} disabled fullWidth />
          <TextField
            label="Stock actual"
            value={productoSeleccionado ? productoSeleccionado.stock : ''}
            disabled
            fullWidth
          />

          <TextField select label="Tipo de movimiento" value={tipoMovimiento} onChange={(e) => setTipoMovimiento(e.target.value as 'MERMA' | 'AJUSTE')}>
            <MenuItem value="MERMA">Merma</MenuItem>
            <MenuItem value="AJUSTE">Ajuste</MenuItem>
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
            startIcon={loading ? <CircularProgress size={18} color="inherit" /> : <AjusteIcon />}
            onClick={handleRegistrar}
            disabled={loading || !productoSeleccionado?.id || !cantidad || !motivo.trim()}
          >
            {loading ? 'Registrando...' : 'Registrar Ajuste'}
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

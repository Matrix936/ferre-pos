import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Autocomplete,
  Box,
  Button,
  CircularProgress,
  MenuItem,
  Paper,
  TextField,
  Typography,
} from '@mui/material';
import { BuildCircle as AjusteIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { ProductoInventario } from '../types';
import { FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { useDebouncedValue } from '../../shared/hooks/useDebouncedValue';
import { useFeedback } from '../../shared/hooks/useFeedback';

const motivosSugeridos = [
  'Defecto de fábrica',
  'Daño por humedad',
  'Robo/Extravío',
  'Caducidad',
  'Ajuste por conteo físico',
];

const QUANTITY_PATTERN = /^\d+(\.\d{0,3})?$/;
type TipoMovimiento = 'MERMA' | 'AJUSTE_ENTRADA' | 'AJUSTE_SALIDA';

export function MermasAjustesView() {
  const { user } = useAuth();
  const { sucursales } = useCatalogos();
  const [selectedSucursalId, setSelectedSucursalId] = useState(user?.sucursalId ?? '');
  const [search, setSearch] = useState('');
  const [productos, setProductos] = useState<ProductoInventario[]>([]);
  const [productoSeleccionado, setProductoSeleccionado] = useState<ProductoInventario | null>(null);
  const [tipoMovimiento, setTipoMovimiento] = useState<TipoMovimiento>('MERMA');
  const [motivo, setMotivo] = useState(motivosSugeridos[0]);
  const [cantidad, setCantidad] = useState('');
  const [loading, setLoading] = useState(false);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();

  const isSuperAdmin = user?.role === 'SUPERADMIN';
  const sucursalTrabajo = isSuperAdmin ? selectedSucursalId : user?.sucursalId ?? '';
  const searchDebounced = useDebouncedValue(search, 300);
  const cantidadValida =
    QUANTITY_PATTERN.test(cantidad.trim()) &&
    Number(cantidad) > 0 &&
    (
      tipoMovimiento === 'AJUSTE_ENTRADA' ||
      !productoSeleccionado ||
      Number(cantidad) <= productoSeleccionado.stock
    );

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
      showFeedback('Completa sucursal y producto.', 'warning');
      return;
    }
    if (!cantidadValida) {
      showFeedback('La cantidad debe ser mayor a cero, máximo 3 decimales y no superar stock si es salida.', 'warning');
      return;
    }
    const qty = Number(cantidad || 0);
    if (tipoMovimiento !== 'AJUSTE_ENTRADA' && productoSeleccionado && qty > productoSeleccionado.stock) {
      showFeedback('La cantidad supera el stock disponible.', 'warning');
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
      showFeedback('Ajuste registrado correctamente.');
      clearForm();
    } catch (error) {
      showFeedback(`Error al registrar ajuste: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
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

          <TextField select label="Tipo de movimiento" value={tipoMovimiento} onChange={(e) => setTipoMovimiento(e.target.value as TipoMovimiento)}>
            <MenuItem value="MERMA">Merma</MenuItem>
            <MenuItem value="AJUSTE_SALIDA">Ajuste de salida</MenuItem>
            <MenuItem value="AJUSTE_ENTRADA">Ajuste de entrada</MenuItem>
          </TextField>

          <TextField
            label="Cantidad"
            type="number"
            value={cantidad}
            onChange={(e) => setCantidad(e.target.value)}
            error={Boolean(cantidad) && !cantidadValida}
            helperText={
              Boolean(cantidad) && !cantidadValida
                ? 'Mayor a 0, máximo 3 decimales y no debe superar stock en salidas.'
                : tipoMovimiento === 'AJUSTE_ENTRADA'
                  ? 'Entrada por conteo físico o corrección autorizada.'
                  : 'Salida de inventario por pérdida, daño o corrección.'
            }
            fullWidth
            slotProps={{ htmlInput: { min: 0.001, step: '0.001', inputMode: 'decimal' } }}
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
            disabled={loading || !productoSeleccionado?.id || !cantidadValida || !motivo.trim()}
          >
            {loading ? 'Registrando...' : 'Registrar Ajuste'}
          </Button>
        </Box>
      </Paper>

      <FeedbackSnackbar message={feedbackMessage} severity={feedbackSeverity} onClose={closeFeedback} />
    </Box>
  );
}

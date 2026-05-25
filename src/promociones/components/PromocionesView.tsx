import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Checkbox,
  Chip,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  FormControlLabel,
  InputAdornment,
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
import {
  Add as AddIcon,
  CalendarMonth as CalendarIcon,
  Delete as DeleteIcon,
  LocalOffer as PromoIcon,
  Save as SaveIcon,
  Storefront as StorefrontIcon,
} from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';

type TipoDescuento = 'PORCENTAJE' | 'MONTO_FIJO';
type TipoAlcance = '' | 'PRODUCTO' | 'CATEGORIA' | 'MARCA';

interface Promocion {
  id: string;
  nombre: string;
  tipoDescuento: TipoDescuento;
  valor: number;
  fechaInicio: string;
  fechaFin: string;
  activo: boolean;
  productoId?: string | null;
  categoriaId?: string | null;
  marca?: string | null;
  sucursalIds: string[];
}

interface ProductoPromocionPrecio {
  id: string;
  codigoBarras: string;
  codigoProveedor: string;
  claveProducto: string;
  descripcion: string;
  marca: string;
  categoria: string;
  unidad: string;
  precioCosto: number;
  precioVenta: number;
  precioCostoMin: number;
  precioCostoMax: number;
  precioVentaMin: number;
  precioVentaMax: number;
  sucursalesConPrecio: number;
}

const toLocalInput = (value: string) => {
  if (!value) return '';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value.slice(0, 16);
  const offsetMs = date.getTimezoneOffset() * 60000;
  return new Date(date.getTime() - offsetMs).toISOString().slice(0, 16);
};

const fromLocalInput = (value: string) => (value ? new Date(value).toISOString() : '');

export function PromocionesView() {
  const { user } = useAuth();
  const { sucursales } = useCatalogos();
  const [promociones, setPromociones] = useState<Promocion[]>([]);
  const [productos, setProductos] = useState<ProductoPromocionPrecio[]>([]);
  const [open, setOpen] = useState(false);
  const [error, setError] = useState('');
  const [saving, setSaving] = useState(false);
  const [currentId, setCurrentId] = useState('');
  const [nombre, setNombre] = useState('');
  const [tipoDescuento, setTipoDescuento] = useState<TipoDescuento>('PORCENTAJE');
  const [valor, setValor] = useState('');
  const [fechaInicio, setFechaInicio] = useState('');
  const [fechaFin, setFechaFin] = useState('');
  const [activo, setActivo] = useState(true);
  const [productoId, setProductoId] = useState('');
  const [productoInput, setProductoInput] = useState('');
  const [categoriaId, setCategoriaId] = useState('');
  const [categoriaInput, setCategoriaInput] = useState('');
  const [marcaId, setMarcaId] = useState('');
  const [marcaInput, setMarcaInput] = useState('');
  const [tipoAlcance, setTipoAlcance] = useState<TipoAlcance>('');
  const [valorError, setValorError] = useState('');
  const [alcanceError, setAlcanceError] = useState('');
  const [fechaError, setFechaError] = useState('');
  const [sucursalesError, setSucursalesError] = useState('');
  const [sucursalIds, setSucursalIds] = useState<string[]>([]);
  const [deletingId, setDeletingId] = useState('');

  const isSuperAdmin = user?.role === 'SUPERADMIN';
  const sucursalesDisponibles = isSuperAdmin
    ? sucursales
    : sucursales.filter((sucursal) => sucursal.id === user?.sucursalId);
  const allSelected = sucursalesDisponibles.length > 0 && sucursalIds.length === sucursalesDisponibles.length;

  const categorias = useMemo(
    () => Array.from(new Set(productos.map((producto) => producto.categoria).filter(Boolean))).sort(),
    [productos],
  );

  const marcas = useMemo(
    () => Array.from(new Set(productos.map((producto) => producto.marca).filter(Boolean))).sort(),
    [productos],
  );

  const productosOpciones = useMemo(() => {
    const query = productoInput.trim().toLowerCase();
    if (query.length < 2) return productoId ? productos.filter((producto) => producto.id === productoId) : [];
    return productos
      .filter((producto) =>
        [producto.descripcion, producto.codigoBarras, producto.codigoProveedor, producto.claveProducto, producto.marca]
          .some((value) => value.toLowerCase().includes(query)),
      )
      .slice(0, 25);
  }, [productoInput, productoId, productos]);

  const categoriasOpciones = useMemo(() => {
    const query = categoriaInput.trim().toLowerCase();
    if (query.length < 2) return categoriaId ? [categoriaId] : [];
    return categorias.filter((categoria) => categoria.toLowerCase().includes(query)).slice(0, 20);
  }, [categoriaInput, categoriaId, categorias]);

  const marcasOpciones = useMemo(() => {
    const query = marcaInput.trim().toLowerCase();
    if (query.length < 2) return marcaId ? [marcaId] : [];
    return marcas.filter((marca) => marca.toLowerCase().includes(query)).slice(0, 20);
  }, [marcaInput, marcaId, marcas]);

  const productoSeleccionado = useMemo(
    () => productos.find((producto) => producto.id === productoId) ?? null,
    [productoId, productos],
  );

  const getScopedSucursalIds = (ids: string[] = sucursalIds) => {
    if (!isSuperAdmin) return user?.sucursalId ? [user.sucursalId] : [];
    return ids;
  };

  const fetchProductosPromociones = async (ids: string[] = sucursalIds) => {
    const data = await invoke<ProductoPromocionPrecio[]>('get_productos_para_promociones', {
      sucursalIds: getScopedSucursalIds(ids),
    });
    setProductos(data);
  };

  const fetchData = async () => {
    const [promosData, productosData] = await Promise.all([
      invoke<Promocion[]>('get_promociones'),
      invoke<ProductoPromocionPrecio[]>('get_productos_para_promociones', {
        sucursalIds: getScopedSucursalIds(isSuperAdmin ? [] : user?.sucursalId ? [user.sucursalId] : []),
      }),
    ]);
    setPromociones(promosData);
    setProductos(productosData);
  };

  useEffect(() => {
    fetchData().catch((err) => setError(String(err)));
  }, [user?.role, user?.sucursalId]);

  const resetForm = () => {
    const defaultInicio = new Date();
    const defaultFin = new Date();
    defaultFin.setDate(defaultFin.getDate() + 7);
    setCurrentId(crypto.randomUUID());
    setNombre('');
    setTipoDescuento('PORCENTAJE');
    setValor('');
    setFechaInicio(toLocalInput(defaultInicio.toISOString()));
    setFechaFin(toLocalInput(defaultFin.toISOString()));
    setActivo(true);
    setProductoId('');
    setProductoInput('');
    setCategoriaId('');
    setCategoriaInput('');
    setMarcaId('');
    setMarcaInput('');
    setTipoAlcance('');
    setValorError('');
    setAlcanceError('');
    setFechaError('');
    setSucursalesError('');
    setSucursalIds(isSuperAdmin ? [] : user?.sucursalId ? [user.sucursalId] : []);
    setError('');
  };

  const handleOpen = (promo?: Promocion) => {
    if (promo) {
      setCurrentId(promo.id);
      setNombre(promo.nombre);
      setTipoDescuento(promo.tipoDescuento);
      setValor(String(promo.valor));
      setFechaInicio(toLocalInput(promo.fechaInicio));
      setFechaFin(toLocalInput(promo.fechaFin));
      setActivo(promo.activo);
      setProductoId(promo.productoId || '');
      setProductoInput(productos.find((producto) => producto.id === promo.productoId)?.descripcion ?? '');
      setCategoriaId(promo.categoriaId || '');
      setCategoriaInput(promo.categoriaId || '');
      setMarcaId(promo.marca || '');
      setMarcaInput(promo.marca || '');
      setTipoAlcance(promo.productoId ? 'PRODUCTO' : promo.categoriaId ? 'CATEGORIA' : promo.marca ? 'MARCA' : '');
      setValorError('');
      setAlcanceError('');
      setFechaError('');
      setSucursalesError('');
      setSucursalIds(isSuperAdmin ? promo.sucursalIds : user?.sucursalId ? [user.sucursalId] : []);
      setError('');
      fetchProductosPromociones(isSuperAdmin ? promo.sucursalIds : user?.sucursalId ? [user.sucursalId] : []).catch((err) => setError(String(err)));
    } else {
      resetForm();
      fetchProductosPromociones(isSuperAdmin ? [] : user?.sucursalId ? [user.sucursalId] : []).catch((err) => setError(String(err)));
    }
    setOpen(true);
  };

  const toggleAll = () => {
    if (!isSuperAdmin) return;
    const next = allSelected ? [] : sucursalesDisponibles.map((sucursal) => sucursal.id);
    setSucursalesError('');
    setSucursalIds(next);
    fetchProductosPromociones(next).catch((err) => setError(String(err)));
  };

  const toggleSucursal = (id: string) => {
    if (!isSuperAdmin) return;
    setSucursalIds((prev) => {
      const next = prev.includes(id) ? prev.filter((item) => item !== id) : [...prev, id];
      fetchProductosPromociones(next).catch((err) => setError(String(err)));
      return next;
    });
  };

  const handleTipoAlcanceChange = (value: TipoAlcance) => {
    setTipoAlcance(value);
    setProductoId('');
    setProductoInput('');
    setCategoriaId('');
    setCategoriaInput('');
    setMarcaId('');
    setMarcaInput('');
    setAlcanceError('');
    setSucursalesError('');
  };

  const validateValor = () => {
    const numericValue = Number(valor || 0);
    if (!Number.isFinite(numericValue) || numericValue <= 0) {
      return tipoDescuento === 'PORCENTAJE'
        ? 'El porcentaje debe ser mayor a 0.'
        : 'El monto debe ser mayor a 0.';
    }
    if (tipoDescuento === 'PORCENTAJE' && (numericValue < 1 || numericValue > 100)) {
      return 'El porcentaje debe estar entre 1 y 100.';
    }
    if (tipoDescuento === 'MONTO_FIJO' && productoSeleccionado && numericValue > productoSeleccionado.precioVenta) {
      return 'El descuento no puede superar el precio de venta del producto.';
    }
    if (tipoAlcance === 'PRODUCTO' && productoSeleccionado) {
      const descuento = tipoDescuento === 'PORCENTAJE'
        ? productoSeleccionado.precioVenta * (numericValue / 100)
        : numericValue;
      const precioFinal = productoSeleccionado.precioVenta - descuento;
      const costo = productoSeleccionado.precioCosto || 0;
      if (costo > 0 && precioFinal < costo) {
        return '¡Cuidado! Este descuento supera tu margen de ganancia y venderás este producto perdiendo dinero.';
      }
    }
    return '';
  };

  const validateAlcance = () => {
    if (!tipoAlcance) return 'Selecciona cómo se aplicará el descuento.';
    if (tipoAlcance === 'PRODUCTO' && !productoId) return 'Selecciona un producto para esta promoción.';
    if (tipoAlcance === 'CATEGORIA' && !categoriaId) return 'Selecciona una categoría para esta promoción.';
    if (tipoAlcance === 'MARCA' && !marcaId) return 'Selecciona una marca para esta promoción.';
    return '';
  };

  const validateFechas = () => {
    if (!fechaInicio || !fechaFin) return 'Captura fecha de inicio y fecha fin.';
    const inicio = new Date(fechaInicio).getTime();
    const fin = new Date(fechaFin).getTime();
    if (!Number.isFinite(inicio) || !Number.isFinite(fin)) return 'Las fechas de la promoción no son válidas.';
    if (fin <= inicio) return 'La fecha fin debe ser posterior a la fecha de inicio.';
    return '';
  };

  const validateSucursales = (ids: string[]) => (
    ids.length === 0 ? 'Selecciona al menos una sucursal para aplicar la promoción.' : ''
  );

  const handleSave = async () => {
    const finalSucursalIds = isSuperAdmin ? sucursalIds : user?.sucursalId ? [user.sucursalId] : [];

    const nextAlcanceError = validateAlcance();
    const nextValorError = validateValor();
    const nextFechaError = validateFechas();
    const nextSucursalesError = validateSucursales(finalSucursalIds);
    setAlcanceError(nextAlcanceError);
    setValorError(nextValorError);
    setFechaError(nextFechaError);
    setSucursalesError(nextSucursalesError);
    if (nextAlcanceError || nextValorError || nextFechaError || nextSucursalesError) {
      return;
    }

    setSaving(true);
    setError('');
    try {
      await invoke('guardar_promocion', {
        promocion: {
          id: currentId,
          nombre,
          tipoDescuento,
          valor: Number(valor || 0),
          fechaInicio: fromLocalInput(fechaInicio),
          fechaFin: fromLocalInput(fechaFin),
          activo,
          productoId: productoId || null,
          categoriaId: categoriaId || null,
          marca: marcaId || null,
          sucursalIds: finalSucursalIds,
        },
      });
      setOpen(false);
      await fetchData();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('¿Eliminar esta promoción?')) return;
    setDeletingId(id);
    try {
      await invoke('eliminar_promocion', { id });
      await fetchData();
    } catch (err) {
      setError(String(err));
    } finally {
      setDeletingId('');
    }
  };

  const getSucursalNombre = (id: string) => sucursales.find((sucursal) => sucursal.id === id)?.nombre || id;
  const getProductoNombre = (id?: string | null) => productos.find((producto) => producto.id === id)?.descripcion || id || '-';
  const formatRange = (min: number, max: number) => (
    Math.abs(min - max) < 0.005 ? `$${min.toFixed(2)}` : `$${min.toFixed(2)} - $${max.toFixed(2)}`
  );

  const getAlcanceLabel = (promo: Promocion) => {
    if (promo.productoId) return getProductoNombre(promo.productoId);
    if (promo.categoriaId) return `Categoría: ${promo.categoriaId}`;
    if (promo.marca) return `Marca: ${promo.marca}`;
    return 'Sin alcance';
  };

  const getEstadoPromo = (promo: Promocion) => {
    const now = Date.now();
    const inicio = new Date(promo.fechaInicio).getTime();
    const fin = new Date(promo.fechaFin).getTime();
    if (!promo.activo) return { label: 'Inactiva', color: 'default' as const };
    if (Number.isFinite(inicio) && now < inicio) return { label: 'Programada', color: 'info' as const };
    if (Number.isFinite(fin) && now > fin) return { label: 'Expirada', color: 'warning' as const };
    return { label: 'Vigente', color: 'success' as const };
  };

  const promocionesVigentes = promociones.filter((promo) => getEstadoPromo(promo).label === 'Vigente').length;
  const promocionesProgramadas = promociones.filter((promo) => getEstadoPromo(promo).label === 'Programada').length;

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: { xs: 'flex-start', md: 'center' }, mb: 3, gap: 2, flexWrap: 'wrap' }}>
        <Box>
          <Typography variant="h5" sx={{ fontWeight: 700 }}>
            Promociones y descuentos
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Administra descuentos por producto, categoría o marca con alcance por sucursal.
          </Typography>
        </Box>
        <Button variant="contained" startIcon={<AddIcon />} onClick={() => handleOpen()} disableElevation sx={{ borderRadius: '10px' }}>
          Nueva promoción
        </Button>
      </Box>

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}

      <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: 'repeat(3, minmax(0, 1fr))' }, gap: 1.5, mb: 2 }}>
        {[
          { label: 'Total', value: promociones.length, icon: <PromoIcon fontSize="small" />, color: 'primary.main' },
          { label: 'Vigentes', value: promocionesVigentes, icon: <CalendarIcon fontSize="small" />, color: 'success.main' },
          { label: 'Programadas', value: promocionesProgramadas, icon: <StorefrontIcon fontSize="small" />, color: 'info.main' },
        ].map((item) => (
          <Paper
            key={item.label}
            elevation={0}
            sx={{
              p: 1.75,
              borderRadius: 2,
              border: '1px solid',
              borderColor: 'divider',
              display: 'flex',
              alignItems: 'center',
              gap: 1.5,
            }}
          >
            <Box sx={{ color: item.color, display: 'flex' }}>{item.icon}</Box>
            <Box>
              <Typography variant="caption" color="text.secondary" sx={{ fontWeight: 700, textTransform: 'uppercase' }}>
                {item.label}
              </Typography>
              <Typography variant="h6" sx={{ fontWeight: 800, lineHeight: 1 }}>
                {item.value}
              </Typography>
            </Box>
          </Paper>
        ))}
      </Box>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Promoción</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Alcance</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Descuento</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Vigencia</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Sucursales</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {promociones.map((promo) => {
                const estado = getEstadoPromo(promo);
                return (
                <TableRow key={promo.id} hover>
                  <TableCell sx={{ minWidth: 220 }}>
                    <Stack spacing={0.75}>
                      <Typography variant="body2" sx={{ fontWeight: 800 }}>{promo.nombre}</Typography>
                      <Chip label={estado.label} size="small" color={estado.color} sx={{ width: 'fit-content', borderRadius: '6px', fontWeight: 700 }} />
                    </Stack>
                  </TableCell>
                  <TableCell sx={{ maxWidth: 320 }}>
                    <Typography variant="body2" sx={{ fontWeight: 600 }}>
                      {getAlcanceLabel(promo)}
                    </Typography>
                    <Typography variant="caption" color="text.secondary">
                      {promo.productoId ? 'Producto específico' : promo.categoriaId ? 'Categoría completa' : 'Marca completa'}
                    </Typography>
                  </TableCell>
                  <TableCell>
                    <Chip
                      label={promo.tipoDescuento === 'PORCENTAJE' ? `${promo.valor}% OFF` : `$${promo.valor.toFixed(2)} OFF`}
                      color="primary"
                      variant="outlined"
                      size="small"
                      sx={{ borderRadius: '6px', fontWeight: 800 }}
                    />
                  </TableCell>
                  <TableCell>
                    <Stack spacing={0.25}>
                      <Typography variant="body2">{new Date(promo.fechaInicio).toLocaleDateString()}</Typography>
                      <Typography variant="caption" color="text.secondary">al {new Date(promo.fechaFin).toLocaleDateString()}</Typography>
                    </Stack>
                  </TableCell>
                  <TableCell>
                    <Box sx={{ display: 'flex', gap: 0.5, flexWrap: 'wrap' }}>
                      {promo.sucursalIds.map((id) => (
                        <Chip key={id} label={getSucursalNombre(id)} size="small" sx={{ borderRadius: '6px' }} />
                      ))}
                    </Box>
                  </TableCell>
                  <TableCell align="right">
                    <Button size="small" onClick={() => handleOpen(promo)} disabled={Boolean(deletingId)}>Editar</Button>
                    <Button
                      size="small"
                      color="error"
                      startIcon={deletingId === promo.id ? <CircularProgress size={16} /> : <DeleteIcon />}
                      onClick={() => handleDelete(promo.id)}
                      disabled={Boolean(deletingId)}
                    >
                      {deletingId === promo.id ? 'Eliminando...' : 'Eliminar'}
                    </Button>
                  </TableCell>
                </TableRow>
                );
              })}
              {promociones.length === 0 && (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay promociones configuradas.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Dialog open={open} onClose={saving ? undefined : () => setOpen(false)} maxWidth="md" fullWidth>
        <DialogTitle sx={{ fontWeight: 700, pb: 1 }}>
          {currentId ? 'Promoción' : 'Nueva promoción'}
          <Typography variant="body2" color="text.secondary">
            Define descuento, alcance, vigencia y sucursales.
          </Typography>
        </DialogTitle>
        <Divider />
        <DialogContent sx={{ '&&': { pt: 3 } }}>
          <Stack spacing={3}>
            <Box>
              <Typography variant="subtitle2" sx={{ fontWeight: 800, mb: 1.5 }}>
                Datos del descuento
              </Typography>
              <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '1.4fr 1fr 0.8fr 0.8fr' }, gap: 2 }}>
                <TextField label="Nombre" value={nombre} onChange={(e) => setNombre(e.target.value)} required />
                <TextField select label="Tipo" value={tipoDescuento} onChange={(e) => {
                  setTipoDescuento(e.target.value as TipoDescuento);
                  setValorError('');
                }}>
                  <MenuItem value="PORCENTAJE">Porcentaje</MenuItem>
                  <MenuItem value="MONTO_FIJO">Monto fijo</MenuItem>
                </TextField>
                <TextField
                  label="Valor"
                  type="number"
                  value={valor}
                  onChange={(e) => {
                    setValor(e.target.value);
                    setValorError('');
                  }}
                  required
                  error={Boolean(valorError)}
                  helperText={valorError || (tipoDescuento === 'PORCENTAJE' ? 'Entre 1 y 100.' : 'Monto neto a descontar.')}
                  slotProps={{
                    htmlInput: { min: tipoDescuento === 'PORCENTAJE' ? 1 : 0.01, max: tipoDescuento === 'PORCENTAJE' ? 100 : undefined, step: '0.01' },
                    input: {
                      startAdornment: tipoDescuento === 'MONTO_FIJO' ? <InputAdornment position="start">$</InputAdornment> : undefined,
                      endAdornment: tipoDescuento === 'PORCENTAJE' ? <InputAdornment position="end">%</InputAdornment> : undefined,
                    },
                  }}
                />
                <TextField select label="Estado" value={activo ? '1' : '0'} onChange={(e) => setActivo(e.target.value === '1')}>
                  <MenuItem value="1">Activa</MenuItem>
                  <MenuItem value="0">Inactiva</MenuItem>
                </TextField>
              </Box>
            </Box>

            <Box>
              <Typography variant="subtitle2" sx={{ fontWeight: 800, mb: 1.5 }}>
                Alcance
              </Typography>
              <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '260px 1fr' }, gap: 2, alignItems: 'start' }}>
                <TextField
                  select
                  label="Aplicar descuento por"
                  value={tipoAlcance}
                  onChange={(e) => handleTipoAlcanceChange(e.target.value as TipoAlcance)}
                  error={Boolean(alcanceError && !tipoAlcance)}
                  helperText={alcanceError && !tipoAlcance ? alcanceError : ' '}
                >
                  <MenuItem value="">Seleccionar</MenuItem>
                  <MenuItem value="PRODUCTO">Producto</MenuItem>
                  <MenuItem value="CATEGORIA">Categoría</MenuItem>
                  <MenuItem value="MARCA">Marca</MenuItem>
                </TextField>

                {tipoAlcance === 'PRODUCTO' && (
                  <Autocomplete
                    options={productosOpciones}
                    value={productos.find((producto) => producto.id === productoId) ?? null}
                    inputValue={productoInput}
                    onInputChange={(_, value, reason) => {
                      setProductoInput(value);
                      if (reason === 'clear') setProductoId('');
                      setAlcanceError('');
                    }}
                    onChange={(_, value) => {
                      setProductoId(value?.id ?? '');
                      setProductoInput(value?.descripcion ?? '');
                      setAlcanceError('');
                      setValorError('');
                    }}
                    getOptionLabel={(option) => `${option.descripcion}${option.marca ? ` · ${option.marca}` : ''}`}
                    isOptionEqualToValue={(option, value) => option.id === value.id}
                    filterOptions={(options) => options}
                    noOptionsText="Escribe al menos 2 letras para buscar productos"
                    renderInput={(params) => (
                      <TextField
                        {...params}
                        label="Producto"
                        error={Boolean(alcanceError)}
                        helperText={
                          alcanceError
                          || (productoSeleccionado
                            ? `Venta: ${formatRange(productoSeleccionado.precioVentaMin, productoSeleccionado.precioVentaMax)} · Costo: ${formatRange(productoSeleccionado.precioCostoMin, productoSeleccionado.precioCostoMax)}`
                            : '')
                        }
                      />
                    )}
                  />
                )}

                {tipoAlcance === 'CATEGORIA' && (
                  <Autocomplete
                    options={categoriasOpciones}
                    value={categoriaId || null}
                    inputValue={categoriaInput}
                    onInputChange={(_, value, reason) => {
                      setCategoriaInput(value);
                      if (reason === 'clear') setCategoriaId('');
                      setAlcanceError('');
                    }}
                    onChange={(_, value) => {
                      setCategoriaId(value ?? '');
                      setCategoriaInput(value ?? '');
                      setAlcanceError('');
                    }}
                    filterOptions={(options) => options}
                    noOptionsText="Escribe al menos 2 letras para buscar categoría"
                    renderInput={(params) => <TextField {...params} label="Categoría" error={Boolean(alcanceError)} helperText={alcanceError} />}
                  />
                )}

                {tipoAlcance === 'MARCA' && (
                  <Autocomplete
                    options={marcasOpciones}
                    value={marcaId || null}
                    inputValue={marcaInput}
                    onInputChange={(_, value, reason) => {
                      setMarcaInput(value);
                      if (reason === 'clear') setMarcaId('');
                      setAlcanceError('');
                    }}
                    onChange={(_, value) => {
                      setMarcaId(value ?? '');
                      setMarcaInput(value ?? '');
                      setAlcanceError('');
                    }}
                    filterOptions={(options) => options}
                    noOptionsText="Escribe al menos 2 letras para buscar marca"
                    renderInput={(params) => <TextField {...params} label="Marca" error={Boolean(alcanceError)} helperText={alcanceError} />}
                  />
                )}
              </Box>
            </Box>

            <Box>
              <Typography variant="subtitle2" sx={{ fontWeight: 800, mb: 1.5 }}>
                Vigencia
              </Typography>
              <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' }, gap: 2 }}>
                <TextField
                  label="Fecha inicio"
                  type="datetime-local"
                  value={fechaInicio}
                  onChange={(e) => {
                    setFechaInicio(e.target.value);
                    setFechaError('');
                  }}
                  error={Boolean(fechaError)}
                  helperText={fechaError || ' '}
                  slotProps={{ inputLabel: { shrink: true } }}
                  required
                />
                <TextField
                  label="Fecha fin"
                  type="datetime-local"
                  value={fechaFin}
                  onChange={(e) => {
                    setFechaFin(e.target.value);
                    setFechaError('');
                  }}
                  error={Boolean(fechaError)}
                  helperText={fechaError || ' '}
                  slotProps={{ inputLabel: { shrink: true } }}
                  required
                />
              </Box>
            </Box>

          <Box>
            <Typography variant="subtitle2" sx={{ fontWeight: 800, mb: 1 }}>
              Sucursales
            </Typography>
            {sucursalesError && (
              <Typography variant="body2" color="error" sx={{ mb: 1 }}>
                {sucursalesError}
              </Typography>
            )}
            {isSuperAdmin && (
              <FormControlLabel
                control={<Checkbox checked={allSelected} indeterminate={sucursalIds.length > 0 && !allSelected} onChange={toggleAll} />}
                label="Seleccionar todas"
              />
            )}
            <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', sm: 'repeat(2, 1fr)' }, gap: 0.5 }}>
              {sucursalesDisponibles.map((sucursal) => (
                <FormControlLabel
                  key={sucursal.id}
                  control={
                    <Checkbox
                      checked={sucursalIds.includes(sucursal.id)}
                      onChange={() => {
                        setSucursalesError('');
                        toggleSucursal(sucursal.id);
                      }}
                      disabled={!isSuperAdmin}
                    />
                  }
                  label={sucursal.nombre}
                />
              ))}
            </Box>
          </Box>
          </Stack>
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={() => setOpen(false)} disabled={saving}>Cancelar</Button>
          <Button
            variant="contained"
            startIcon={saving ? <CircularProgress size={18} color="inherit" /> : <SaveIcon />}
            onClick={handleSave}
            disabled={saving || !nombre || !valor || !fechaInicio || !fechaFin || !tipoAlcance || (isSuperAdmin && sucursalIds.length === 0)}
          >
            {saving ? 'Guardando...' : 'Guardar'}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

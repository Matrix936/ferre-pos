import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Autocomplete,
  Box,
  Button,
  CircularProgress,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  MenuItem,
  Paper,
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
  DeleteOutlined as DeleteOutlineIcon,
  Edit as EditIcon,
  LocalOffer as LocalOfferIcon,
  Print as PrintIcon,
  Save as SaveIcon,
} from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { InventarioSucursalPayload, ProductoCatalogo, ProductoInventario } from '../types';
import { TableActions } from '../../shared/components/TableActions';
import { useDebouncedValue } from '../../shared/hooks/useDebouncedValue';
import { EtiquetasPrecioModal } from './EtiquetasPrecioModal';

const QUANTITY_PATTERN = /^\d+(\.\d{0,3})?$/;
const MONEY_PATTERN = /^\d+(\.\d{0,2})?$/;
const isValidQuantity = (value: string) => {
  const trimmed = value.trim();
  if (!QUANTITY_PATTERN.test(trimmed)) return false;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) && parsed >= 0;
};
const isValidMoney = (value: string) => {
  const trimmed = value.trim();
  if (!MONEY_PATTERN.test(trimmed)) return false;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) && parsed >= 0;
};

export function InventarioView() {
  const { user } = useAuth();
  const { sucursales } = useCatalogos();
  const [inventario, setInventario] = useState<ProductoInventario[]>([]);
  const [catalogo, setCatalogo] = useState<ProductoCatalogo[]>([]);
  const [searchInput, setSearchInput] = useState('');
  const [searchApplied, setSearchApplied] = useState('');
  const [searchOptions, setSearchOptions] = useState<ProductoInventario[]>([]);
  const [selectedSucursalId, setSelectedSucursalId] = useState('');
  const [open, setOpen] = useState(false);
  const [productoSeleccionado, setProductoSeleccionado] = useState<ProductoCatalogo | null>(null);
  const [catalogoInput, setCatalogoInput] = useState('');
  const [stockSucursalId, setStockSucursalId] = useState('');
  const [stock, setStock] = useState('0');
  const [stockMinimo, setStockMinimo] = useState('0');
  const [costoPromedio, setCostoPromedio] = useState('0');
  const [precioVenta, setPrecioVenta] = useState('0');
  const [productosEtiqueta, setProductosEtiqueta] = useState<ProductoInventario[]>([]);
  const [deletingKey, setDeletingKey] = useState('');
  const [saving, setSaving] = useState(false);

  const userSucursalId = user?.sucursalId ?? '';
  const isSuperAdmin = user?.role === 'SUPERADMIN';
  const sucursalConsulta = isSuperAdmin ? selectedSucursalId : userSucursalId;
  const debouncedSearchInput = useDebouncedValue(searchInput, 300);
  const stockInvalido = Boolean(stock) && !isValidQuantity(stock);
  const stockMinimoInvalido = Boolean(stockMinimo) && !isValidQuantity(stockMinimo);
  const costoPromedioInvalido = Boolean(costoPromedio) && !isValidMoney(costoPromedio);
  const precioVentaInvalido = Boolean(precioVenta) && !isValidMoney(precioVenta);
  const formInvalido = stockInvalido || stockMinimoInvalido || costoPromedioInvalido || precioVentaInvalido;

  const fetchInventario = async () => {
    if (!sucursalConsulta) return;
    const query = searchApplied.trim();
    const data = query
      ? await invoke<ProductoInventario[]>('buscar_productos_por_sucursal', { sucursalId: sucursalConsulta, query })
      : await invoke<ProductoInventario[]>('get_productos_por_sucursal', { sucursalId: sucursalConsulta });
    setInventario(data);
  };

  const fetchCatalogo = async () => {
    const data = await invoke<ProductoCatalogo[]>('get_productos_catalogo');
    setCatalogo(data);
  };

  useEffect(() => {
    if (!selectedSucursalId && sucursales.length > 0) {
      setSelectedSucursalId(userSucursalId || sucursales[0].id);
    }
  }, [selectedSucursalId, sucursales, userSucursalId]);

  useEffect(() => {
    if (!isSuperAdmin && userSucursalId) setSelectedSucursalId(userSucursalId);
  }, [isSuperAdmin, userSucursalId]);

  useEffect(() => {
    fetchInventario().catch((error) => console.error('Error inventario:', error));
  }, [sucursalConsulta, searchApplied]);

  useEffect(() => {
    fetchCatalogo().catch((error) => console.error('Error catálogo productos:', error));
  }, []);

  useEffect(() => {
    const q = debouncedSearchInput.trim();
    if (!sucursalConsulta || q.length < 2) {
      setSearchOptions([]);
      if (!q) setSearchApplied('');
      return;
    }
    let active = true;
    invoke<ProductoInventario[]>('buscar_productos_por_sucursal', { sucursalId: sucursalConsulta, query: q })
      .then((data) => {
        if (active) {
          setSearchOptions(data);
          setSearchApplied(q);
        }
      })
      .catch((error) => {
        console.error('Error sugerencias inventario:', error);
        if (active) setSearchOptions([]);
      });
    return () => {
      active = false;
    };
  }, [sucursalConsulta, debouncedSearchInput]);

  const catalogoDisponible = useMemo(() => {
    const idsEnSucursal = new Set(inventario.map((item) => item.id));
    return catalogo.filter((item) => !idsEnSucursal.has(item.id) || item.id === productoSeleccionado?.id);
  }, [catalogo, inventario, productoSeleccionado?.id]);

  const catalogoOpciones = useMemo(() => {
    const query = catalogoInput.trim().toLowerCase();
    if (query.length < 2) return productoSeleccionado ? [productoSeleccionado] : [];
    return catalogoDisponible
      .filter((item) =>
        [item.descripcion, item.codigoBarras, item.codigoProveedor, item.claveProducto, item.marca, item.unidad]
          .some((value) => value.toLowerCase().includes(query)),
      )
      .slice(0, 25);
  }, [catalogoDisponible, catalogoInput, productoSeleccionado]);

  const openNew = () => {
    setProductoSeleccionado(null);
    setStockSucursalId(sucursalConsulta || userSucursalId);
    setStock('0');
    setStockMinimo('0');
    setCostoPromedio('0');
    setPrecioVenta('0');
    setCatalogoInput('');
    setOpen(true);
  };

  const openEdit = (producto: ProductoInventario) => {
    setProductoSeleccionado(catalogo.find((item) => item.id === producto.id) ?? {
      id: producto.id,
      codigoBarras: producto.codigoBarras,
      codigoProveedor: producto.codigoProveedor,
      proveedorId: producto.proveedorId,
      claveProducto: producto.claveProducto,
      descripcion: producto.descripcion,
      marca: producto.marca,
      categoria: producto.categoria,
      unidad: producto.unidad,
      precioCosto: 0,
      precioVenta: 0,
      satClaveProdServ: producto.satClaveProdServ,
      satClaveUnidad: producto.satClaveUnidad,
    });
    setStockSucursalId(producto.sucursalId || sucursalConsulta || userSucursalId);
    setStock(String(producto.stock ?? 0));
    setStockMinimo(String(producto.stockMinimo ?? 0));
    setCostoPromedio(String(producto.costoPromedio ?? producto.precioCosto ?? 0));
    setPrecioVenta(String(producto.precioVenta ?? 0));
    setCatalogoInput(producto.descripcion);
    setOpen(true);
  };

  const handleSave = async () => {
    if (saving) return;
    if (!productoSeleccionado || !stockSucursalId) return;
    if (formInvalido) return;
    const inventarioPayload: InventarioSucursalPayload = {
      sucursalId: stockSucursalId,
      stock: Number(stock || 0),
      stockMinimo: Number(stockMinimo || 0),
      costoPromedio: Number(costoPromedio || 0),
      precioVenta: Number(precioVenta || 0),
    };
    setSaving(true);
    try {
      await invoke('guardar_inventario_sucursal', {
        productoId: productoSeleccionado.id,
        inventario: inventarioPayload,
      });
      setOpen(false);
      await fetchInventario();
    } catch (error) {
      alert(`Error al guardar inventario: ${error}`);
    } finally {
      setSaving(false);
    }
  };

  const handleEliminarInventario = async (producto: ProductoInventario) => {
    const key = `${producto.id}-${producto.sucursalId}`;
    if (deletingKey) return;
    const confirmed = window.confirm(
      `¿Quitar "${producto.descripcion}" del inventario de esta sucursal? El producto seguirá existiendo en el catálogo general.`,
    );
    if (!confirmed) return;
    setDeletingKey(key);
    try {
      await invoke('eliminar_inventario_sucursal', {
        productoId: producto.id,
        sucursalId: producto.sucursalId,
      });
      await fetchInventario();
    } catch (error) {
      alert(`Error al eliminar del inventario: ${error}`);
    } finally {
      setDeletingKey('');
    }
  };

  const exportRows = useMemo(
    () =>
      inventario.map((producto) => ({
        codigoProveedor: producto.codigoProveedor,
        descripcion: producto.descripcion,
        marca: producto.marca,
        precioVenta: producto.precioVenta,
        costoPromedio: producto.costoPromedio,
        stock: producto.stock,
        stockMinimo: producto.stockMinimo,
      })),
    [inventario],
  );

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3, gap: 2, flexWrap: 'wrap' }}>
        <Typography variant="h5" sx={{ fontWeight: 700 }}>Inventario por sucursal</Typography>
        <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
          <Button
            variant="outlined"
            startIcon={<PrintIcon />}
            onClick={() => setProductosEtiqueta(inventario)}
            disabled={inventario.length === 0}
          >
            Imprimir etiquetas
          </Button>
          <Button variant="contained" startIcon={<AddIcon />} onClick={openNew} disableElevation>
            Agregar producto a sucursal
          </Button>
        </Box>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap', alignItems: 'center' }}>
          <Autocomplete
            freeSolo
            options={searchOptions}
            getOptionLabel={(option) => (typeof option === 'string' ? option : option.descripcion)}
            filterOptions={(options) => options}
            noOptionsText="Escribe al menos 2 letras para buscar coincidencias"
            inputValue={searchInput}
            onInputChange={(_, value, reason) => {
              if (reason === 'reset') {
                setSearchOptions([]);
                return;
              }
              setSearchInput(value);
              if (reason !== 'input' || !value.trim()) {
                setSearchOptions([]);
                setSearchApplied('');
              }
            }}
            onChange={(_, value) => {
              if (!value) return;
              const query = typeof value === 'string' ? value.trim() : value.descripcion.trim();
              if (!query) return;
              setSearchApplied(query);
              setSearchInput('');
              setSearchOptions([]);
            }}
            renderInput={(params) => (
              <TextField {...params} label="Buscar inventario por producto, código o marca" fullWidth />
            )}
            sx={{ flex: 1, minWidth: 320 }}
          />
          {isSuperAdmin && (
            <TextField select label="Sucursal consultada" value={selectedSucursalId} onChange={(e) => setSelectedSucursalId(e.target.value)} sx={{ minWidth: 260 }}>
              {sucursales.map((sucursal) => (
                <MenuItem key={sucursal.id} value={sucursal.id}>{sucursal.nombre}</MenuItem>
              ))}
            </TextField>
          )}
          <TableActions
            filename="inventario"
            rows={exportRows}
            columns={[
              { key: 'codigoProveedor', label: 'Código Proveedor' },
              { key: 'descripcion', label: 'Descripción' },
              { key: 'marca', label: 'Marca' },
              { key: 'precioVenta', label: 'Precio Venta' },
              { key: 'costoPromedio', label: 'Costo Promedio' },
              { key: 'stock', label: 'Stock Actual' },
              { key: 'stockMinimo', label: 'Stock Mínimo' },
            ]}
          />
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table sx={{ minWidth: 900 }}>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Código</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Producto</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Marca</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Precio venta</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Costo prom.</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Stock</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Mínimo</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {inventario.map((producto) => (
                <TableRow key={`${producto.id}-${producto.sucursalId}`} hover>
                  <TableCell>{producto.codigoProveedor || producto.claveProducto || '-'}</TableCell>
                  <TableCell>{producto.descripcion}</TableCell>
                  <TableCell><Chip label={producto.marca || 'Sin marca'} size="small" sx={{ borderRadius: '6px' }} /></TableCell>
                  <TableCell>${producto.precioVenta.toFixed(2)}</TableCell>
                  <TableCell>${(producto.costoPromedio ?? producto.precioCosto ?? 0).toFixed(2)}</TableCell>
                  <TableCell>{producto.stock}</TableCell>
                  <TableCell>{producto.stockMinimo}</TableCell>
                  <TableCell align="right">
                    <Button size="small" startIcon={<LocalOfferIcon />} onClick={() => setProductosEtiqueta([producto])}>
                      Etiqueta
                    </Button>
                    <Button size="small" startIcon={<EditIcon />} onClick={() => openEdit(producto)}>
                      Editar
                    </Button>
                    <Button
                      size="small"
                      color="error"
                      startIcon={<DeleteOutlineIcon />}
                      onClick={() => handleEliminarInventario(producto)}
                      disabled={deletingKey === `${producto.id}-${producto.sucursalId}`}
                    >
                      Eliminar
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
              {inventario.length === 0 && (
                <TableRow>
                  <TableCell colSpan={8} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay productos configurados en esta sucursal.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Dialog open={open} onClose={saving ? undefined : () => setOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle sx={{ fontWeight: 600 }}>Configurar inventario por sucursal</DialogTitle>
        <Divider />
        <DialogContent sx={{ pt: 3, display: 'flex', flexDirection: 'column', gap: 2 }}>
          <Autocomplete
            options={catalogoOpciones}
            value={productoSeleccionado}
            inputValue={catalogoInput}
            onInputChange={(_, value, reason) => {
              setCatalogoInput(value);
              if (reason === 'clear') setProductoSeleccionado(null);
            }}
            onChange={(_, value) => {
              setProductoSeleccionado(value);
              setCatalogoInput(value?.descripcion ?? '');
            }}
            getOptionLabel={(option) => `${option.descripcion}${option.marca ? ` · ${option.marca}` : ''}`}
            isOptionEqualToValue={(option, value) => option.id === value.id}
            filterOptions={(options) => options}
            noOptionsText="Escribe al menos 2 letras para buscar producto"
            renderInput={(params) => <TextField {...params} label="Producto" required />}
          />
          <TextField select label="Sucursal" value={stockSucursalId} onChange={(e) => setStockSucursalId(e.target.value)} required>
            {sucursales.map((sucursal) => (
              <MenuItem key={sucursal.id} value={sucursal.id}>{sucursal.nombre}</MenuItem>
            ))}
          </TextField>
          <TextField
            label="Stock actual"
            type="number"
            value={stock}
            onChange={(e) => setStock(e.target.value)}
            error={stockInvalido}
            helperText={stockInvalido ? 'Usa máximo 3 decimales.' : ' '}
            slotProps={{ htmlInput: { min: 0, step: '0.001', inputMode: 'decimal' } }}
          />
          <TextField
            label="Stock mínimo"
            type="number"
            value={stockMinimo}
            onChange={(e) => setStockMinimo(e.target.value)}
            error={stockMinimoInvalido}
            helperText={stockMinimoInvalido ? 'Usa máximo 3 decimales.' : ' '}
            slotProps={{ htmlInput: { min: 0, step: '0.001', inputMode: 'decimal' } }}
          />
          <TextField
            label="Costo promedio en esta sucursal"
            type="number"
            value={costoPromedio}
            onChange={(e) => setCostoPromedio(e.target.value)}
            error={costoPromedioInvalido}
            helperText={costoPromedioInvalido ? 'Usa máximo 2 decimales.' : ' '}
            slotProps={{ htmlInput: { min: 0, step: '0.01', inputMode: 'decimal' } }}
          />
          <TextField
            label="Precio venta en esta sucursal"
            type="number"
            value={precioVenta}
            onChange={(e) => setPrecioVenta(e.target.value)}
            error={precioVentaInvalido}
            helperText={precioVentaInvalido ? 'Usa máximo 2 decimales.' : ' '}
            slotProps={{ htmlInput: { min: 0, step: '0.01', inputMode: 'decimal' } }}
          />
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={() => setOpen(false)} disabled={saving}>Cancelar</Button>
          <Button
            variant="contained"
            startIcon={saving ? <CircularProgress size={18} color="inherit" /> : <SaveIcon />}
            onClick={handleSave}
            disabled={saving || !productoSeleccionado || !stockSucursalId || formInvalido}
          >
            {saving ? 'Guardando...' : 'Guardar'}
          </Button>
        </DialogActions>
      </Dialog>
      <EtiquetasPrecioModal
        open={productosEtiqueta.length > 0}
        productos={productosEtiqueta}
        onClose={() => setProductosEtiqueta([])}
      />
    </Box>
  );
}

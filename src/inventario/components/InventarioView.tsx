import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Autocomplete,
  Box,
  Button,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  LinearProgress,
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
import { InventarioSucursalPayload, ProductoCatalogo, ProductoCatalogoPage, ProductoInventario, ProductoInventarioPage } from '../types';
import { AsyncButton } from '../../shared/components/AsyncButton';
import { ConfirmActionDialog } from '../../shared/components/ConfirmActionDialog';
import { FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { TablePager } from '../../shared/components/TablePager';
import { TableActions } from '../../shared/components/TableActions';
import { useDebouncedValue } from '../../shared/hooks/useDebouncedValue';
import { useDialogHotkeys } from '../../shared/hooks/useDialogHotkeys';
import { useFeedback } from '../../shared/hooks/useFeedback';
import { dialogActionsSx, dialogContentSx } from '../../shared/ui/patterns';
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
  const [deleteTarget, setDeleteTarget] = useState<ProductoInventario | null>(null);
  const [saving, setSaving] = useState(false);
  const [loadingRows, setLoadingRows] = useState(false);
  const [loadingCatalogo, setLoadingCatalogo] = useState(false);
  const [totalRows, setTotalRows] = useState(0);
  const [page, setPage] = useState(0);
  const [pageSize, setPageSizeState] = useState(10);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();

  const userSucursalId = user?.sucursalId ?? '';
  const isSuperAdmin = user?.role === 'SUPERADMIN';
  const sucursalConsulta = isSuperAdmin ? selectedSucursalId : userSucursalId;
  const debouncedSearchInput = useDebouncedValue(searchInput, 300);
  const debouncedCatalogoInput = useDebouncedValue(catalogoInput, 250);
  const stockInvalido = Boolean(stock) && !isValidQuantity(stock);
  const stockMinimoInvalido = Boolean(stockMinimo) && !isValidQuantity(stockMinimo);
  const costoPromedioInvalido = Boolean(costoPromedio) && !isValidMoney(costoPromedio);
  const precioVentaInvalido = Boolean(precioVenta) && !isValidMoney(precioVenta);
  const formInvalido = stockInvalido || stockMinimoInvalido || costoPromedioInvalido || precioVentaInvalido;

  const fetchInventario = async (options?: { active?: () => boolean }) => {
    if (!sucursalConsulta) return;
    setLoadingRows(true);
    const data = await invoke<ProductoInventarioPage>('get_productos_por_sucursal_page', {
      sucursalId: sucursalConsulta,
      query: searchApplied.trim(),
      page,
      pageSize,
    });
    if (options?.active && !options.active()) return;
    setInventario(data.rows);
    setTotalRows(data.total);
    setLoadingRows(false);
  };

  const fetchCatalogo = async (query: string, options?: { active?: () => boolean }) => {
    const cleanQuery = query.trim();
    if (cleanQuery.length < 2) {
      setCatalogo(productoSeleccionado ? [productoSeleccionado] : []);
      setLoadingCatalogo(false);
      return;
    }
    setLoadingCatalogo(true);
    const data = await invoke<ProductoCatalogoPage>('get_productos_catalogo_page', {
      query: cleanQuery,
      page: 0,
      pageSize: 25,
    });
    if (options?.active && !options.active()) return;
    setCatalogo(data.rows);
    setLoadingCatalogo(false);
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
    setPage(0);
  }, [sucursalConsulta, searchApplied]);

  useEffect(() => {
    let active = true;
    fetchInventario({ active: () => active }).catch((error) => {
      if (active) {
        console.error('Error inventario:', error);
        setLoadingRows(false);
      }
    });
    return () => {
      active = false;
    };
  }, [sucursalConsulta, searchApplied, page, pageSize]);

  useEffect(() => {
    let active = true;
    fetchCatalogo(debouncedCatalogoInput, { active: () => active }).catch((error) => {
      if (active) {
        console.error('Error catálogo productos:', error);
        setLoadingCatalogo(false);
      }
    });
    return () => {
      active = false;
    };
  }, [debouncedCatalogoInput, productoSeleccionado?.id]);

  useEffect(() => {
    const q = debouncedSearchInput.trim();
    if (!q) {
      if (!q) setSearchApplied('');
      return;
    }
    if (q.length >= 2) setSearchApplied(q);
  }, [sucursalConsulta, debouncedSearchInput]);

  const catalogoOpciones = useMemo(() => {
    if (productoSeleccionado && catalogoInput.trim().length < 2) return [productoSeleccionado];
    return catalogo;
  }, [catalogo, catalogoInput, productoSeleccionado]);

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
      showFeedback('Inventario actualizado correctamente.');
    } catch (error) {
      showFeedback(`Error al guardar inventario: ${error}`, 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleEliminarInventario = async (producto: ProductoInventario) => {
    const key = `${producto.id}-${producto.sucursalId}`;
    if (deletingKey) return;
    setDeletingKey(key);
    try {
      await invoke('eliminar_inventario_sucursal', {
        productoId: producto.id,
        sucursalId: producto.sucursalId,
      });
      await fetchInventario();
      setDeleteTarget(null);
      showFeedback('Producto retirado del inventario de la sucursal.');
    } catch (error) {
      showFeedback(`Error al eliminar del inventario: ${error}`, 'error');
    } finally {
      setDeletingKey('');
    }
  };

  const inventarioSaveDisabled = saving || !productoSeleccionado || !stockSucursalId || formInvalido;
  const closeInventarioDialog = () => setOpen(false);
  useDialogHotkeys({
    open,
    disabled: inventarioSaveDisabled,
    cancelDisabled: saving,
    onConfirm: handleSave,
    onCancel: closeInventarioDialog,
  });

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
  const totalPages = Math.max(1, Math.ceil(totalRows / pageSize));
  const fromRow = totalRows === 0 ? 0 : page * pageSize + 1;
  const toRow = Math.min((page + 1) * pageSize, totalRows);
  const setPageSize = (value: number) => {
    setPageSizeState(value);
    setPage(0);
  };

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
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
          <TextField
            label="Buscar inventario por producto, código o marca"
            value={searchInput}
            onChange={(event) => setSearchInput(event.target.value)}
            helperText={searchInput.trim() && searchInput.trim().length < 2 ? 'Escribe al menos 2 letras para buscar.' : ' '}
            fullWidth
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
        {loadingRows && <LinearProgress />}
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
                <TableCell sx={{ fontWeight: 600 }}>Acciones</TableCell>
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
                  <TableCell>
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
                      onClick={() => setDeleteTarget(producto)}
                      disabled={deletingKey === `${producto.id}-${producto.sucursalId}`}
                    >
                      Eliminar
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
              {!loadingRows && inventario.length === 0 && (
                <TableRow>
                  <TableCell colSpan={8} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay productos configurados en esta sucursal.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
        <TablePager
          page={page}
          pageSize={pageSize}
          totalPages={totalPages}
          totalRows={totalRows}
          fromRow={fromRow}
          toRow={toRow}
          canPreviousPage={page > 0 && !loadingRows}
          canNextPage={page < totalPages - 1 && !loadingRows}
          onPreviousPage={() => setPage((current) => Math.max(0, current - 1))}
          onNextPage={() => setPage((current) => Math.min(totalPages - 1, current + 1))}
          onPageSizeChange={setPageSize}
          rowLabel="productos"
        />
      </Paper>

      <Dialog open={open} onClose={saving ? undefined : () => setOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle sx={{ fontWeight: 600 }}>Configurar inventario por sucursal</DialogTitle>
        <Divider />
        <DialogContent sx={dialogContentSx}>
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
            loading={loadingCatalogo}
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
        <DialogActions sx={{ ...dialogActionsSx, p: 3, pt: 1 }}>
          <Button onClick={() => setOpen(false)} disabled={saving}>Cancelar</Button>
          <AsyncButton
            variant="contained"
            startIcon={<SaveIcon />}
            onClick={handleSave}
            disabled={inventarioSaveDisabled}
            loading={saving}
            loadingText="Guardando..."
          >
            Guardar
          </AsyncButton>
        </DialogActions>
      </Dialog>
      <EtiquetasPrecioModal
        open={productosEtiqueta.length > 0}
        productos={productosEtiqueta}
        onClose={() => setProductosEtiqueta([])}
      />
      <ConfirmActionDialog
        open={Boolean(deleteTarget)}
        title="Quitar del inventario"
        message={`¿Quitar "${deleteTarget?.descripcion ?? ''}" del inventario de esta sucursal? El producto seguirá existiendo en el catálogo general.`}
        confirmText="Quitar"
        confirmColor="error"
        loading={Boolean(deletingKey)}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={() => {
          if (deleteTarget) return handleEliminarInventario(deleteTarget);
        }}
      />
      <FeedbackSnackbar message={feedbackMessage} severity={feedbackSeverity} onClose={closeFeedback} />
    </Box>
  );
}

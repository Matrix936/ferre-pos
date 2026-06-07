import { ReactNode, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Checkbox,
  Chip,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  FormControlLabel,
  LinearProgress,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  Tab,
  Tabs,
  TextField,
  Typography,
} from '@mui/material';
import { Add as AddIcon, Edit as EditIcon, Save as SaveIcon } from '@mui/icons-material';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { ProductoCatalogo } from '../../inventario/types';
import { AsyncButton } from '../../shared/components/AsyncButton';
import { FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { TablePager } from '../../shared/components/TablePager';
import { TableActions } from '../../shared/components/TableActions';
import { useDialogHotkeys } from '../../shared/hooks/useDialogHotkeys';
import { useDebouncedValue } from '../../shared/hooks/useDebouncedValue';
import { useFeedback } from '../../shared/hooks/useFeedback';
import { dialogActionsSx, dialogContentSx } from '../../shared/ui/patterns';

interface ProductoCatalogoPage {
  rows: ProductoCatalogo[];
  total: number;
}

function ProductoTabPanel({ value, index, children }: { value: number; index: number; children: ReactNode }) {
  if (value !== index) return null;
  return <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' }, gap: 2.5 }}>{children}</Box>;
}

const filterByText = <T,>(items: T[], inputValue: string, getText: (item: T) => string) => {
  const query = inputValue.trim().toLowerCase();
  if (query.length < 2) return [];
  return items
    .filter((item) => getText(item).toLowerCase().includes(query))
    .slice(0, 20);
};

const emptyProduct = (): ProductoCatalogo => ({
  id: crypto.randomUUID(),
  codigoBarras: '',
  codigoProveedor: '',
  proveedorId: '',
  claveProducto: '',
  descripcion: '',
  marca: '',
  categoria: '',
  unidad: '',
  precioCosto: 0,
  precioVenta: 0,
  satClaveProdServ: '',
  satClaveUnidad: '',
  precio1: 0,
  precio2: 0,
  precio3: 0,
  precio4: 0,
  mayoreoApartir: 0,
  aGranel: false,
  noEnCatalogo: false,
  ventasNegativas: false,
  caducidad: null,
  fotos: '',
  descripcionCatalogo: '',
});

export function ProductosView() {
  const { proveedores, marcas, categorias, unidades } = useCatalogos();
  const [productos, setProductos] = useState<ProductoCatalogo[]>([]);
  const [search, setSearch] = useState('');
  const [open, setOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [producto, setProducto] = useState<ProductoCatalogo>(emptyProduct);
  const [warning, setWarning] = useState('');
  const [saving, setSaving] = useState(false);
  const [loadingRows, setLoadingRows] = useState(false);
  const [productoTab, setProductoTab] = useState(0);
  const [totalRows, setTotalRows] = useState(0);
  const [page, setPage] = useState(0);
  const [pageSize, setPageSizeState] = useState(10);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();
  const debouncedSearch = useDebouncedValue(search, 250);

  const fetchProductos = async (options?: { active?: () => boolean }) => {
    setLoadingRows(true);
    const data = await invoke<ProductoCatalogoPage>('get_productos_catalogo_page', {
      query: debouncedSearch.trim(),
      page,
      pageSize,
    });
    if (options?.active && !options.active()) return;
    setProductos(data.rows);
    setTotalRows(data.total);
    setLoadingRows(false);
  };

  useEffect(() => {
    setPage(0);
  }, [debouncedSearch]);

  useEffect(() => {
    let active = true;
    fetchProductos({ active: () => active }).catch((error) => {
      if (active) {
        console.error('Error productos catálogo:', error);
        setLoadingRows(false);
      }
    });
    return () => {
      active = false;
    };
  }, [debouncedSearch, page, pageSize]);

  const totalPages = Math.max(1, Math.ceil(totalRows / pageSize));
  const fromRow = totalRows === 0 ? 0 : page * pageSize + 1;
  const toRow = Math.min((page + 1) * pageSize, totalRows);
  const setPageSize = (value: number) => {
    setPageSizeState(value);
    setPage(0);
  };

  const unidadSeleccionada = useMemo(
    () => unidades.find((unidad) => unidad.nombre === producto.unidad) ?? null,
    [producto.unidad, unidades],
  );

  const satClaveUnidadDerivada = unidadSeleccionada?.claveSat ?? '';

  const handleOpen = (item?: ProductoCatalogo) => {
    setWarning('');
    setEditMode(Boolean(item));
    setProducto(item ? { ...item } : emptyProduct());
    setProductoTab(0);
    setOpen(true);
  };

  const update = <K extends keyof ProductoCatalogo>(key: K, value: ProductoCatalogo[K]) => {
    setProducto((prev) => ({ ...prev, [key]: value }));
  };

  const handleSave = async () => {
    if (saving) return;
    if (!producto.proveedorId.trim()) {
      setWarning('Selecciona un proveedor válido.');
      return;
    }
    setSaving(true);
    try {
      const payload = {
        ...producto,
        codigoBarras: producto.codigoBarras.trim(),
        codigoProveedor: producto.codigoProveedor.trim(),
        proveedorId: producto.proveedorId.trim(),
        claveProducto: producto.claveProducto.trim(),
        descripcion: producto.descripcion.trim(),
        marca: producto.marca.trim(),
        categoria: producto.categoria.trim(),
        unidad: producto.unidad.trim(),
        satClaveProdServ: producto.satClaveProdServ.trim().toUpperCase(),
        satClaveUnidad: satClaveUnidadDerivada.trim().toUpperCase(),
        precio1: Number(producto.precio1 || 0),
        precio2: Number(producto.precio2 || 0),
        precio3: Number(producto.precio3 || 0),
        precio4: Number(producto.precio4 || 0),
        mayoreoApartir: Number(producto.mayoreoApartir || 0),
        aGranel: Boolean(producto.aGranel),
        noEnCatalogo: Boolean(producto.noEnCatalogo),
        ventasNegativas: Boolean(producto.ventasNegativas),
        caducidad: producto.caducidad || null,
        fotos: producto.fotos?.trim() ?? '',
        descripcionCatalogo: producto.descripcionCatalogo?.trim() ?? '',
        precioCosto: 0,
        precioVenta: 0,
      };
      if (editMode) {
        await invoke('update_producto_catalogo', { productoId: producto.id, producto: payload });
      } else {
        await invoke('create_producto_catalogo', { producto: payload });
      }
      setOpen(false);
      await fetchProductos();
      showFeedback(editMode ? 'Producto actualizado correctamente.' : 'Producto creado correctamente.');
    } catch (error) {
      showFeedback(`Error al guardar: ${error}`, 'error');
    } finally {
      setSaving(false);
    }
  };

  const productoSaveDisabled =
    saving ||
    !producto.descripcion.trim() ||
    !producto.proveedorId.trim() ||
    !producto.marca.trim() ||
    !producto.categoria.trim() ||
    !producto.unidad.trim() ||
    !producto.satClaveProdServ.trim() ||
    !satClaveUnidadDerivada.trim();

  const closeProductoDialog = () => setOpen(false);
  useDialogHotkeys({
    open,
    disabled: productoSaveDisabled,
    cancelDisabled: saving,
    onConfirm: handleSave,
    onCancel: closeProductoDialog,
  });

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3, gap: 2, flexWrap: 'wrap' }}>
        <Typography variant="h5" sx={{ fontWeight: 700 }}>Productos</Typography>
        <Button variant="contained" startIcon={<AddIcon />} onClick={() => handleOpen()} disableElevation>
          Nuevo producto
        </Button>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
          <TextField
            label="Buscar producto"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            fullWidth
          />
          <TableActions
            filename="productos"
            rows={productos.map((item) => ({
              codigoProveedor: item.codigoProveedor,
              descripcion: item.descripcion,
              marca: item.marca,
              unidad: item.unidad,
              satProducto: item.satClaveProdServ,
              satUnidad: item.satClaveUnidad,
            }))}
            columns={[
              { key: 'codigoProveedor', label: 'Código proveedor' },
              { key: 'descripcion', label: 'Descripción' },
              { key: 'marca', label: 'Marca' },
              { key: 'unidad', label: 'Unidad' },
              { key: 'satProducto', label: 'SAT Producto' },
              { key: 'satUnidad', label: 'SAT Unidad' },
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
                <TableCell sx={{ fontWeight: 600 }}>Código proveedor</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Descripción</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Marca</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Unidad</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Clave SAT</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {productos.map((item) => (
                <TableRow key={item.id} hover>
                  <TableCell>{item.codigoProveedor || '-'}</TableCell>
                  <TableCell>
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 0.75, flexWrap: 'wrap' }}>
                      <Box component="span">{item.descripcion}</Box>
                      {item.aGranel && <Chip size="small" label="Granel" sx={{ height: 22, borderRadius: 1 }} />}
                      {Number(item.mayoreoApartir || 0) > 0 && (
                        <Chip size="small" color="primary" label="Mayoreo" sx={{ height: 22, borderRadius: 1, fontWeight: 700 }} />
                      )}
                    </Box>
                  </TableCell>
                  <TableCell>{item.marca || '-'}</TableCell>
                  <TableCell>{item.unidad || '-'}</TableCell>
                  <TableCell>{item.satClaveProdServ || '-'}</TableCell>
                  <TableCell>
                    <Button size="small" startIcon={<EditIcon />} onClick={() => handleOpen(item)}>
                      Editar
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
              {!loadingRows && productos.length === 0 && (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay productos registrados.
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

      <Dialog open={open} onClose={saving ? undefined : () => setOpen(false)} maxWidth="md" fullWidth>
        <DialogTitle sx={{ fontWeight: 600 }}>{editMode ? 'Editar producto' : 'Nuevo producto'}</DialogTitle>
        <Divider />
        <DialogContent sx={{ ...dialogContentSx, gap: 2.5 }}>
          {warning && <Alert severity="warning">{warning}</Alert>}
          <Tabs
            value={productoTab}
            onChange={(_, value) => setProductoTab(value)}
            variant="scrollable"
            scrollButtons="auto"
            sx={{ minHeight: 42, borderBottom: '1px solid', borderColor: 'divider' }}
          >
            <Tab label="Datos generales" sx={{ minHeight: 42, textTransform: 'none', fontWeight: 700 }} />
            <Tab label="Comercial" sx={{ minHeight: 42, textTransform: 'none', fontWeight: 700 }} />
            <Tab label="Catálogo" sx={{ minHeight: 42, textTransform: 'none', fontWeight: 700 }} />
          </Tabs>

          <ProductoTabPanel value={productoTab} index={0}>
            <TextField label="Código de barras" value={producto.codigoBarras} onChange={(e) => update('codigoBarras', e.target.value)} fullWidth />
            <TextField label="Código de proveedor" value={producto.codigoProveedor} onChange={(e) => update('codigoProveedor', e.target.value)} fullWidth />
            <Autocomplete
              options={proveedores}
              value={proveedores.find((proveedor) => proveedor.id === producto.proveedorId) ?? null}
              onChange={(_, value) => update('proveedorId', value?.id ?? '')}
              getOptionLabel={(option) => option.nombre}
              isOptionEqualToValue={(option, value) => typeof value !== 'string' && option.id === value.id}
              filterOptions={(options, state) => filterByText(options, state.inputValue, (option) => option.nombre)}
              noOptionsText="Escribe al menos 2 letras para buscar proveedor"
              renderInput={(params) => <TextField {...params} label="Proveedor" fullWidth required />}
            />
            <TextField label="Clave interna" value={producto.claveProducto} onChange={(e) => update('claveProducto', e.target.value)} fullWidth />
            <TextField label="Descripción" value={producto.descripcion} onChange={(e) => update('descripcion', e.target.value)} fullWidth required />
            <Autocomplete
              options={marcas}
              value={marcas.find((marca) => marca.nombre === producto.marca) ?? null}
              onChange={(_, value) => update('marca', value?.nombre ?? '')}
              getOptionLabel={(option) => option.nombre}
              isOptionEqualToValue={(option, value) => option.id === value.id}
              filterOptions={(options, state) => filterByText(options, state.inputValue, (option) => option.nombre)}
              noOptionsText="Escribe al menos 2 letras para buscar marcas"
              renderInput={(params) => <TextField {...params} label="Marca" fullWidth />}
            />
            <Autocomplete
              options={categorias}
              value={categorias.find((categoria) => categoria.nombre === producto.categoria) ?? null}
              onChange={(_, value) => update('categoria', value?.nombre ?? '')}
              getOptionLabel={(option) => option.nombre}
              isOptionEqualToValue={(option, value) => option.id === value.id}
              filterOptions={(options, state) => filterByText(options, state.inputValue, (option) => option.nombre)}
              noOptionsText="Escribe al menos 2 letras para buscar categorías"
              renderInput={(params) => <TextField {...params} label="Categoría" fullWidth />}
            />
            <Autocomplete
              options={unidades}
              value={unidades.find((unidad) => unidad.nombre === producto.unidad) ?? null}
              onChange={(_, value) => {
                update('unidad', value?.nombre ?? '');
                update('satClaveUnidad', value?.claveSat ?? '');
              }}
              getOptionLabel={(option) => option.nombre}
              isOptionEqualToValue={(option, value) => option.id === value.id}
              filterOptions={(options, state) => filterByText(options, state.inputValue, (option) => `${option.nombre} ${option.claveSat ?? ''}`)}
              noOptionsText="Escribe al menos 2 letras para buscar unidades"
              renderInput={(params) => (
                <TextField
                  {...params}
                  label="Unidad"
                  helperText={satClaveUnidadDerivada ? `Clave SAT: ${satClaveUnidadDerivada}` : ' '}
                  fullWidth
                />
              )}
            />
            <TextField
              label="Clave Producto/Servicio SAT"
              value={producto.satClaveProdServ}
              onChange={(e) => update('satClaveProdServ', e.target.value.toUpperCase())}
              required
              helperText="Ej. 27111700"
              slotProps={{ htmlInput: { maxLength: 8 } }}
            />
          </ProductoTabPanel>

          <ProductoTabPanel value={productoTab} index={1}>
            <Alert severity="info" sx={{ gridColumn: { xs: 'auto', md: '1 / -1' } }}>
              Estos precios especiales se aplican automáticamente en Punto de Venta cuando la cantidad alcanza el mínimo de mayoreo.
            </Alert>
            <TextField
              label="Mayoreo a partir de"
              type="number"
              value={producto.mayoreoApartir ?? 0}
              onChange={(e) => update('mayoreoApartir', Number(e.target.value || 0))}
              helperText="Cantidad mínima para activar precio de mayoreo"
              slotProps={{ htmlInput: { min: 0, step: '0.001' } }}
            />
            <TextField
              label="Precio mayoreo 1"
              type="number"
              value={producto.precio1 ?? 0}
              onChange={(e) => update('precio1', Number(e.target.value || 0))}
              slotProps={{ htmlInput: { min: 0, step: '0.01' } }}
            />
            <TextField
              label="Precio mayoreo 2"
              type="number"
              value={producto.precio2 ?? 0}
              onChange={(e) => update('precio2', Number(e.target.value || 0))}
              slotProps={{ htmlInput: { min: 0, step: '0.01' } }}
            />
            <TextField
              label="Precio mayoreo 3"
              type="number"
              value={producto.precio3 ?? 0}
              onChange={(e) => update('precio3', Number(e.target.value || 0))}
              slotProps={{ htmlInput: { min: 0, step: '0.01' } }}
            />
            <TextField
              label="Precio mayoreo 4"
              type="number"
              value={producto.precio4 ?? 0}
              onChange={(e) => update('precio4', Number(e.target.value || 0))}
              slotProps={{ htmlInput: { min: 0, step: '0.01' } }}
            />
            <Box sx={{ gridColumn: { xs: 'auto', md: '1 / -1' }, display: 'flex', gap: 1, flexWrap: 'wrap' }}>
              <FormControlLabel
                control={<Checkbox checked={Boolean(producto.aGranel)} onChange={(e) => update('aGranel', e.target.checked)} />}
                label="Producto a granel"
              />
              <FormControlLabel
                control={<Checkbox checked={Boolean(producto.ventasNegativas)} onChange={(e) => update('ventasNegativas', e.target.checked)} />}
                label="Permitir venta sin existencia"
              />
            </Box>
          </ProductoTabPanel>

          <ProductoTabPanel value={productoTab} index={2}>
            <TextField
              label="Descripción para catálogo"
              value={producto.descripcionCatalogo ?? ''}
              onChange={(e) => update('descripcionCatalogo', e.target.value)}
              multiline
              minRows={3}
              sx={{ gridColumn: { xs: 'auto', md: '1 / -1' } }}
            />
            <TextField
              label="URL o ruta de fotos"
              value={producto.fotos ?? ''}
              onChange={(e) => update('fotos', e.target.value)}
              fullWidth
            />
            <TextField
              label="Caducidad"
              type="date"
              value={producto.caducidad ?? ''}
              onChange={(e) => update('caducidad', e.target.value || null)}
              slotProps={{ inputLabel: { shrink: true } }}
              fullWidth
            />
            <FormControlLabel
              sx={{ gridColumn: { xs: 'auto', md: '1 / -1' } }}
              control={<Checkbox checked={Boolean(producto.noEnCatalogo)} onChange={(e) => update('noEnCatalogo', e.target.checked)} />}
              label="Ocultar de catálogo público"
            />
          </ProductoTabPanel>
        </DialogContent>
        <DialogActions sx={{ ...dialogActionsSx, p: 3, pt: 1 }}>
          <Button onClick={() => setOpen(false)} disabled={saving}>Cancelar</Button>
          <AsyncButton
            variant="contained"
            startIcon={<SaveIcon />}
            onClick={handleSave}
            disabled={productoSaveDisabled}
            loading={saving}
            loadingText="Guardando..."
          >
            Guardar
          </AsyncButton>
        </DialogActions>
      </Dialog>
      <FeedbackSnackbar message={feedbackMessage} severity={feedbackSeverity} onClose={closeFeedback} />
    </Box>
  );
}

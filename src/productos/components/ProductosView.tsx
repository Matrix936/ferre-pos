import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Box,
  Button,
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
import { Add as AddIcon, Edit as EditIcon, Save as SaveIcon } from '@mui/icons-material';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { ProductoCatalogo } from '../../inventario/types';
import { TableActions } from '../../shared/components/TableActions';

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
});

export function ProductosView() {
  const { proveedores, marcas, unidades } = useCatalogos();
  const [productos, setProductos] = useState<ProductoCatalogo[]>([]);
  const [search, setSearch] = useState('');
  const [open, setOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [producto, setProducto] = useState<ProductoCatalogo>(emptyProduct);
  const [warning, setWarning] = useState('');

  const fetchProductos = async () => {
    const data = await invoke<ProductoCatalogo[]>('get_productos_catalogo');
    setProductos(data);
  };

  useEffect(() => {
    fetchProductos().catch((error) => console.error('Error productos catálogo:', error));
  }, []);

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return productos;
    return productos.filter((item) =>
      [item.descripcion, item.codigoBarras, item.codigoProveedor, item.claveProducto, item.marca, item.categoria]
        .some((value) => value.toLowerCase().includes(q)),
    );
  }, [productos, search]);

  const handleOpen = (item?: ProductoCatalogo) => {
    setWarning('');
    setEditMode(Boolean(item));
    setProducto(item ? { ...item } : emptyProduct());
    setOpen(true);
  };

  const update = <K extends keyof ProductoCatalogo>(key: K, value: ProductoCatalogo[K]) => {
    setProducto((prev) => ({ ...prev, [key]: value }));
  };

  const handleSave = async () => {
    if (!producto.proveedorId.trim()) {
      setWarning('Selecciona un proveedor válido.');
      return;
    }
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
        satClaveUnidad: producto.satClaveUnidad.trim().toUpperCase(),
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
    } catch (error) {
      alert(`Error al guardar: ${error}`);
    }
  };

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
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
            rows={filtered.map((item) => ({
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
        <TableContainer>
          <Table sx={{ minWidth: 900 }}>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Código proveedor</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Descripción</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Marca</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Unidad</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Clave SAT</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filtered.map((item) => (
                <TableRow key={item.id} hover>
                  <TableCell>{item.codigoProveedor || '-'}</TableCell>
                  <TableCell>{item.descripcion}</TableCell>
                  <TableCell>{item.marca || '-'}</TableCell>
                  <TableCell>{item.unidad || '-'}</TableCell>
                  <TableCell>{item.satClaveProdServ || '-'}</TableCell>
                  <TableCell align="right">
                    <Button size="small" startIcon={<EditIcon />} onClick={() => handleOpen(item)}>
                      Editar
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
              {filtered.length === 0 && (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay productos registrados.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Dialog open={open} onClose={() => setOpen(false)} maxWidth="md" fullWidth>
        <DialogTitle sx={{ fontWeight: 600 }}>{editMode ? 'Editar producto' : 'Nuevo producto'}</DialogTitle>
        <Divider />
        <DialogContent sx={{ pt: 3 }}>
          <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' }, gap: 2.5 }}>
            {warning && (
              <Alert severity="warning" sx={{ gridColumn: { xs: 'auto', md: '1 / -1' } }}>
                {warning}
              </Alert>
            )}
            <TextField label="Código de barras" value={producto.codigoBarras} onChange={(e) => update('codigoBarras', e.target.value)} fullWidth />
            <TextField label="Código de proveedor" value={producto.codigoProveedor} onChange={(e) => update('codigoProveedor', e.target.value)} fullWidth />
            <TextField select label="Proveedor" value={producto.proveedorId} onChange={(e) => update('proveedorId', e.target.value)} fullWidth required>
              <MenuItem value=""><em>Selecciona proveedor</em></MenuItem>
              {proveedores.map((proveedor) => (
                <MenuItem key={proveedor.id} value={proveedor.id}>{proveedor.nombre}</MenuItem>
              ))}
            </TextField>
            <TextField label="Clave interna" value={producto.claveProducto} onChange={(e) => update('claveProducto', e.target.value)} fullWidth />
            <TextField label="Descripción" value={producto.descripcion} onChange={(e) => update('descripcion', e.target.value)} fullWidth required />
            <TextField select label="Marca" value={producto.marca} onChange={(e) => update('marca', e.target.value)} fullWidth>
              <MenuItem value=""><em>Sin marca</em></MenuItem>
              {marcas.map((marca) => (
                <MenuItem key={marca.id} value={marca.nombre}>{marca.nombre}</MenuItem>
              ))}
            </TextField>
            <TextField label="Categoría" value={producto.categoria} onChange={(e) => update('categoria', e.target.value)} fullWidth />
            <TextField
              select
              label="Unidad"
              value={producto.unidad}
              onChange={(e) => {
                const selected = unidades.find((unidad) => unidad.nombre === e.target.value);
                update('unidad', e.target.value);
                if (selected?.claveSat) update('satClaveUnidad', selected.claveSat);
              }}
              fullWidth
            >
              <MenuItem value=""><em>Sin unidad</em></MenuItem>
              {unidades.map((unidad) => (
                <MenuItem key={unidad.id} value={unidad.nombre}>{unidad.nombre}</MenuItem>
              ))}
            </TextField>
            <TextField
              label="Clave Producto/Servicio SAT"
              value={producto.satClaveProdServ}
              onChange={(e) => update('satClaveProdServ', e.target.value.toUpperCase())}
              required
              helperText="Ej. 27111700"
              slotProps={{ htmlInput: { maxLength: 8 } }}
            />
            <TextField
              label="Clave Unidad SAT"
              value={producto.satClaveUnidad}
              onChange={(e) => update('satClaveUnidad', e.target.value.toUpperCase())}
              required
              helperText="Ej. H87, KGM"
              slotProps={{ htmlInput: { maxLength: 3 } }}
            />
          </Box>
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={() => setOpen(false)}>Cancelar</Button>
          <Button
            variant="contained"
            startIcon={<SaveIcon />}
            onClick={handleSave}
            disabled={!producto.descripcion.trim() || !producto.proveedorId.trim() || !producto.satClaveProdServ.trim() || !producto.satClaveUnidad.trim()}
          >
            Guardar
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

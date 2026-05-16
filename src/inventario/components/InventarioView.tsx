import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Box,
  Button,
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
import { Add as AddIcon, Edit as EditIcon, Save as SaveIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { InventarioSucursalPayload, ProductoInventario, ProductoPayload, Proveedor } from '../types';
import { TableActions } from '../../shared/components/TableActions';
import { Sucursal } from '../../sucursales/types';

export function InventarioView() {
  const { user } = useAuth();
  const [productos, setProductos] = useState<ProductoInventario[]>([]);
  const [sucursales, setSucursales] = useState<Sucursal[]>([]);
  const [proveedores, setProveedores] = useState<Proveedor[]>([]);
  const [search, setSearch] = useState('');
  const [open, setOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [currentId, setCurrentId] = useState('');
  const [codigoBarras, setCodigoBarras] = useState('');
  const [codigoProveedor, setCodigoProveedor] = useState('');
  const [proveedorId, setProveedorId] = useState('');
  const [claveProducto, setClaveProducto] = useState('');
  const [descripcion, setDescripcion] = useState('');
  const [marca, setMarca] = useState('');
  const [categoria, setCategoria] = useState('');
  const [unidad, setUnidad] = useState('');
  const [precioCosto, setPrecioCosto] = useState('0');
  const [precioVenta, setPrecioVenta] = useState('0');
  const [stock, setStock] = useState('0');
  const [stockMinimo, setStockMinimo] = useState('0');
  const [selectedSucursalId, setSelectedSucursalId] = useState('');
  const [stockSucursalId, setStockSucursalId] = useState('');

  const userSucursalId = user?.sucursalId ?? '';
  const isSuperAdmin = user?.role === 'SUPERADMIN';
  const sucursalConsulta = isSuperAdmin ? selectedSucursalId : userSucursalId;

  const fetchSucursales = async () => {
    try {
      const data = await invoke<Sucursal[]>('get_sucursales');
      setSucursales(data);
      if (!selectedSucursalId && data.length > 0) {
        setSelectedSucursalId(userSucursalId || data[0].id);
      }
    } catch (error) {
      console.error('Error al obtener sucursales:', error);
    }
  };

  const fetchProveedores = async () => {
    try {
      const data = await invoke<Proveedor[]>('get_proveedores');
      setProveedores(data);
    } catch (error) {
      console.error('Error al obtener proveedores:', error);
    }
  };

  const fetchProductos = async () => {
    if (!sucursalConsulta) return;

    try {
      const query = search.trim();
      const data = query
        ? await invoke<ProductoInventario[]>('buscar_productos_por_sucursal', { sucursalId: sucursalConsulta, query })
        : await invoke<ProductoInventario[]>('get_productos_por_sucursal', { sucursalId: sucursalConsulta });
      setProductos(data);
    } catch (error) {
      console.error('Error al obtener inventario:', error);
    }
  };

  useEffect(() => {
    fetchSucursales();
    fetchProveedores();
  }, []);

  useEffect(() => {
    if (!isSuperAdmin && userSucursalId) {
      setSelectedSucursalId(userSucursalId);
    }
  }, [isSuperAdmin, userSucursalId]);

  useEffect(() => {
    fetchProductos();
  }, [sucursalConsulta, search]);

  const resetForm = () => {
    setCurrentId(crypto.randomUUID());
    setCodigoBarras('');
    setCodigoProveedor('');
    setProveedorId('');
    setClaveProducto('');
    setDescripcion('');
    setMarca('');
    setCategoria('');
    setUnidad('');
    setPrecioCosto('0');
    setPrecioVenta('0');
    setStock('0');
    setStockMinimo('0');
    setStockSucursalId(sucursalConsulta || userSucursalId);
  };

  const handleOpen = (producto?: ProductoInventario) => {
    if (producto) {
      setEditMode(true);
      setCurrentId(producto.id);
      setCodigoBarras(producto.codigoBarras ?? '');
      setCodigoProveedor(producto.codigoProveedor ?? '');
      setProveedorId(producto.proveedorId ?? '');
      setClaveProducto(producto.claveProducto ?? '');
      setDescripcion(producto.descripcion ?? '');
      setMarca(producto.marca ?? '');
      setCategoria(producto.categoria ?? '');
      setUnidad(producto.unidad ?? '');
      setPrecioCosto(String(producto.precioCosto ?? 0));
      setPrecioVenta(String(producto.precioVenta ?? 0));
      setStock(String(producto.stock ?? 0));
      setStockMinimo(String(producto.stockMinimo ?? 0));
      setStockSucursalId(producto.sucursalId || sucursalConsulta || userSucursalId);
    } else {
      setEditMode(false);
      resetForm();
    }

    setOpen(true);
  };

  const handleClose = () => setOpen(false);

  const handleSave = async () => {
    if (!stockSucursalId) return;

    const producto: ProductoPayload = {
      id: currentId,
      codigoBarras: codigoBarras.trim(),
      codigoProveedor: codigoProveedor.trim(),
      proveedorId: proveedorId.trim(),
      claveProducto: claveProducto.trim(),
      descripcion: descripcion.trim(),
      marca: marca.trim(),
      categoria: categoria.trim(),
      unidad: unidad.trim(),
      precioCosto: Number(precioCosto || 0),
      precioVenta: Number(precioVenta || 0),
    };

    const inventario: InventarioSucursalPayload = {
      sucursalId: stockSucursalId,
      stock: Number(stock || 0),
      stockMinimo: Number(stockMinimo || 0),
    };

    try {
      if (editMode) {
        await invoke('update_producto', { productoId: currentId, producto, inventario });
      } else {
        await invoke('create_producto', { producto, inventario });
      }
      handleClose();
      fetchProductos();
    } catch (error) {
      console.error('Error al guardar producto:', error);
      alert(`Error al guardar: ${error}`);
    }
  };

  const exportRows = useMemo(
    () =>
      productos.map((producto) => ({
        codigoProveedor: producto.codigoProveedor,
        descripcion: producto.descripcion,
        marca: producto.marca,
        precioVenta: producto.precioVenta,
        stock: producto.stock,
      })),
    [productos],
  );

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3, gap: 2, flexWrap: 'wrap' }}>
        <Typography variant="h5" sx={{ fontWeight: 700, color: 'text.primary' }}>
          Inventario por sucursal
        </Typography>
        <Button variant="contained" startIcon={<AddIcon />} onClick={() => handleOpen()} disableElevation sx={{ borderRadius: '8px', px: 3 }}>
          Nuevo producto
        </Button>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap', alignItems: 'center' }}>
          <TextField
            label="Buscar por descripción, código de barras, código proveedor o clave Truper"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            fullWidth
          />
          {isSuperAdmin && (
            <TextField
              select
              label="Sucursal consultada"
              value={selectedSucursalId}
              onChange={(event) => setSelectedSucursalId(event.target.value)}
              sx={{ minWidth: 260 }}
            >
              {sucursales.map((sucursal) => (
                <MenuItem key={sucursal.id} value={sucursal.id}>
                  {sucursal.nombre}
                </MenuItem>
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
              { key: 'stock', label: 'Stock Actual' },
            ]}
          />
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table sx={{ minWidth: 750 }}>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Código de Proveedor</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Descripción</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Marca</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Precio de Venta</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Stock Actual</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {productos.map((producto) => (
                <TableRow key={producto.id} hover>
                  <TableCell>{producto.codigoProveedor || '-'}</TableCell>
                  <TableCell>{producto.descripcion}</TableCell>
                  <TableCell>
                    <Chip label={producto.marca || 'Sin marca'} size="small" sx={{ borderRadius: '6px', fontWeight: 500 }} />
                  </TableCell>
                  <TableCell>${producto.precioVenta.toFixed(2)}</TableCell>
                  <TableCell>{producto.stock}</TableCell>
                  <TableCell align="right">
                    <Button size="small" startIcon={<EditIcon />} onClick={() => handleOpen(producto)}>
                      Editar
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
              {productos.length === 0 && (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay productos para esta sucursal.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Dialog open={open} onClose={handleClose} maxWidth="md" fullWidth slotProps={{ paper: { sx: { borderRadius: 2 } } }}>
        <DialogTitle sx={{ fontWeight: 600, pb: 1 }}>{editMode ? 'Editar producto' : 'Nuevo producto'}</DialogTitle>
        <Divider />
        <DialogContent sx={{ pt: 3 }}>
          <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' }, gap: 2.5 }}>
            <TextField label="Código de barras" value={codigoBarras} onChange={(e) => setCodigoBarras(e.target.value)} fullWidth helperText="Editable manualmente para simular lector QR/barras" />
            <TextField label="Código de proveedor" value={codigoProveedor} onChange={(e) => setCodigoProveedor(e.target.value)} fullWidth />
            <TextField
              select
              label="Proveedor"
              value={proveedorId}
              onChange={(e) => setProveedorId(e.target.value)}
              fullWidth
            >
              <MenuItem value="">Sin proveedor</MenuItem>
              {proveedores.map((proveedor) => (
                <MenuItem key={proveedor.id} value={proveedor.id}>
                  {proveedor.nombre}
                </MenuItem>
              ))}
            </TextField>
            <TextField label="Clave Truper / producto" value={claveProducto} onChange={(e) => setClaveProducto(e.target.value)} fullWidth />
            <TextField label="Descripción" value={descripcion} onChange={(e) => setDescripcion(e.target.value)} fullWidth required />
            <TextField label="Marca" value={marca} onChange={(e) => setMarca(e.target.value)} fullWidth />
            <TextField label="Categoría" value={categoria} onChange={(e) => setCategoria(e.target.value)} fullWidth />
            <TextField label="Unidad" value={unidad} onChange={(e) => setUnidad(e.target.value)} fullWidth />
            <TextField label="Precio costo" type="number" value={precioCosto} onChange={(e) => setPrecioCosto(e.target.value)} fullWidth />
            <TextField label="Precio venta" type="number" value={precioVenta} onChange={(e) => setPrecioVenta(e.target.value)} fullWidth />
            <TextField label="Stock actual" type="number" value={stock} onChange={(e) => setStock(e.target.value)} fullWidth />
            <TextField label="Stock mínimo" type="number" value={stockMinimo} onChange={(e) => setStockMinimo(e.target.value)} fullWidth />
            {isSuperAdmin && (
              <TextField
                select
                label="Sucursal para este stock"
                value={stockSucursalId}
                onChange={(e) => setStockSucursalId(e.target.value)}
                fullWidth
              >
                {sucursales.map((sucursal) => (
                  <MenuItem key={sucursal.id} value={sucursal.id}>
                    {sucursal.nombre}
                  </MenuItem>
                ))}
              </TextField>
            )}
          </Box>
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={handleClose}>Cancelar</Button>
          <Button
            onClick={handleSave}
            variant="contained"
            disableElevation
            startIcon={<SaveIcon />}
            disabled={!descripcion.trim() || !stockSucursalId}
          >
            Guardar
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

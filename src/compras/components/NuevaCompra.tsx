import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Autocomplete,
  Box,
  Button,
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
import { Delete as DeleteIcon, Save as SaveIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { ProductoInventario, RegistrarCompraPayload } from '../../inventario/types';
import { AsyncButton } from '../../shared/components/AsyncButton';
import { FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { TablePager } from '../../shared/components/TablePager';
import { useDebouncedValue } from '../../shared/hooks/useDebouncedValue';
import { useFeedback } from '../../shared/hooks/useFeedback';
import { useLocalPagination } from '../../shared/hooks/useLocalPagination';

interface CompraRow {
  productoId: string;
  descripcion: string;
  marca: string;
  cantidad: string;
  precioCostoPactado: string;
}

const QUANTITY_PATTERN = /^\d+(\.\d{0,3})?$/;
const MONEY_PATTERN = /^\d+(\.\d{0,2})?$/;

export function NuevaCompra() {
  const { user } = useAuth();
  const { proveedores, sucursales } = useCatalogos();
  const [productosBusqueda, setProductosBusqueda] = useState<ProductoInventario[]>([]);
  const [detalle, setDetalle] = useState<CompraRow[]>([]);
  const [search, setSearch] = useState('');
  const [proveedorId, setProveedorId] = useState('');
  const [selectedSucursalId, setSelectedSucursalId] = useState(user?.sucursalId ?? '');
  const [loading, setLoading] = useState(false);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();

  const isSuperAdmin = user?.role === 'SUPERADMIN';
  const sucursalCompraId = isSuperAdmin ? selectedSucursalId : user?.sucursalId ?? '';
  const searchDebounced = useDebouncedValue(search, 300);

  useEffect(() => {
    if (!selectedSucursalId && sucursales.length > 0) {
      setSelectedSucursalId(user?.sucursalId || sucursales[0].id);
    }
  }, [selectedSucursalId, sucursales, user?.sucursalId]);

  useEffect(() => {
    const query = searchDebounced.trim();
    if (!sucursalCompraId || query.length <= 2) {
      setProductosBusqueda([]);
      return;
    }
    let active = true;
    invoke<ProductoInventario[]>('buscar_productos_para_compra', { sucursalId: sucursalCompraId, query })
      .then((data) => {
        if (active) setProductosBusqueda(data);
      })
      .catch((error) => {
        console.error('Error productos:', error);
        if (active) setProductosBusqueda([]);
      });
    return () => {
      active = false;
    };
  }, [searchDebounced, sucursalCompraId]);

  const total = useMemo(
    () =>
      detalle.reduce((acc, row) => {
        const cantidad = Number(row.cantidad || 0);
        const costo = Number(row.precioCostoPactado || 0);
        return acc + cantidad * costo;
      }, 0),
    [detalle],
  );
  const detallePager = useLocalPagination(detalle);
  const detalleValido = detalle.every((row) => {
    const cantidad = row.cantidad.trim();
    const costo = row.precioCostoPactado.trim();
    return (
      QUANTITY_PATTERN.test(cantidad) &&
      MONEY_PATTERN.test(costo) &&
      Number(cantidad) > 0 &&
      Number(costo) > 0
    );
  });

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
    if (loading) return;
    if (!proveedorId || !sucursalCompraId || detalle.length === 0) return;
    if (!detalleValido) {
      showFeedback('Revisa cantidades y costos. Cantidad máximo 3 decimales y costo máximo 2 decimales.', 'warning');
      return;
    }
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
      showFeedback('Entrada registrada correctamente.');
      setProductosBusqueda([]);
    } catch (error) {
      console.error('Error al registrar compra:', error);
      showFeedback(`Error al registrar compra: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const registrarDisabled = loading || !proveedorId || !sucursalCompraId || detalle.length === 0 || !detalleValido;

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
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
          <Autocomplete
            freeSolo
            options={productosBusqueda}
            getOptionLabel={(option) => (typeof option === 'string' ? option : option.descripcion)}
            filterOptions={(options) => options}
            noOptionsText="Escribe al menos 3 caracteres para buscar coincidencias"
            inputValue={search}
            onInputChange={(_, value, reason) => {
              if (reason === 'reset') {
                setSearch('');
                setProductosBusqueda([]);
                return;
              }
              setSearch(value);
              if (reason !== 'input' || !value.trim()) {
                setProductosBusqueda([]);
              }
            }}
            onChange={(_, value) => {
              if (!value) return;
              if (typeof value === 'string') {
                const selected = productosBusqueda.find((item) =>
                  item.descripcion.toLowerCase().includes(value.trim().toLowerCase()),
                );
                if (selected) {
                  addProducto(selected);
                }
              } else {
                addProducto(value);
              }
              setSearch('');
              setProductosBusqueda([]);
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
                    ${option.precioVenta.toFixed(2)}
                  </Typography>
                </Box>
              </Box>
            )}
            renderInput={(params) => (
              <TextField
                {...params}
                label="Buscar producto por descripción, código o clave"
                fullWidth
              />
            )}
          />
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
                <TableCell sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {detallePager.paginatedRows.map((row) => {
                const subtotal = Number(row.cantidad || 0) * Number(row.precioCostoPactado || 0);
                const cantidadInvalida =
                  Boolean(row.cantidad) &&
                  (!QUANTITY_PATTERN.test(row.cantidad.trim()) || Number(row.cantidad) <= 0);
                const costoInvalido =
                  Boolean(row.precioCostoPactado) &&
                  (!MONEY_PATTERN.test(row.precioCostoPactado.trim()) || Number(row.precioCostoPactado) <= 0);
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
                        error={cantidadInvalida}
                        helperText={cantidadInvalida ? 'Mayor a 0, máximo 3 decimales.' : ' '}
                        slotProps={{ htmlInput: { min: 0.001, step: '0.001', inputMode: 'decimal' } }}
                      />
                    </TableCell>
                    <TableCell sx={{ width: 180 }}>
                      <TextField
                        type="number"
                        size="small"
                        value={row.precioCostoPactado}
                        onChange={(e) => updateRow(row.productoId, 'precioCostoPactado', e.target.value)}
                        error={costoInvalido}
                        helperText={costoInvalido ? 'Mayor a 0, máximo 2 decimales.' : ' '}
                        slotProps={{ htmlInput: { min: 0.01, step: '0.01', inputMode: 'decimal' } }}
                      />
                    </TableCell>
                    <TableCell>${subtotal.toFixed(2)}</TableCell>
                    <TableCell>
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
        <TablePager
          page={detallePager.page}
          pageSize={detallePager.pageSize}
          totalPages={detallePager.totalPages}
          totalRows={detallePager.totalRows}
          fromRow={detallePager.fromRow}
          toRow={detallePager.toRow}
          canPreviousPage={detallePager.canPreviousPage}
          canNextPage={detallePager.canNextPage}
          onPreviousPage={detallePager.previousPage}
          onNextPage={detallePager.nextPage}
          onPageSizeChange={detallePager.setPageSize}
          rowLabel="productos"
        />
      </Paper>

      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mt: 2 }}>
        <Typography variant="h6" sx={{ fontWeight: 700 }}>
          Total: ${total.toFixed(2)}
        </Typography>
        <AsyncButton
          variant="contained"
          startIcon={<SaveIcon />}
          onClick={handleRegistrar}
          disabled={registrarDisabled}
          loading={loading}
          loadingText="Registrando..."
        >
          Registrar entrada
        </AsyncButton>
      </Box>

      <FeedbackSnackbar message={feedbackMessage} severity={feedbackSeverity} onClose={closeFeedback} />
    </Box>
  );
}

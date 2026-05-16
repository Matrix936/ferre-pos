import { FormEvent, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Badge,
  Box,
  Button,
  DialogContentText,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  List,
  ListItemButton,
  ListItemText,
  MenuItem,
  Paper,
  Snackbar,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from '@mui/material';
import { Delete as DeleteIcon, Payments as PaymentsIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { Cliente, ProductoInventario, RegistrarVentaPayload } from '../../inventario/types';

interface VentaRow {
  productoId: string;
  descripcion: string;
  marca: string;
  cantidad: string;
  precioVenta: string;
}

interface TicketEnEspera {
  id: number;
  referencia: string;
  productos: VentaRow[];
  total: number;
}

export function NuevaVenta() {
  const { user } = useAuth();
  const [productos, setProductos] = useState<ProductoInventario[]>([]);
  const [carrito, setCarrito] = useState<VentaRow[]>([]);
  const [busqueda, setBusqueda] = useState('');
  const [metodoPago, setMetodoPago] = useState('EFECTIVO');
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [clienteId, setClienteId] = useState('');
  const [openCobrar, setOpenCobrar] = useState(false);
  const [efectivoRecibido, setEfectivoRecibido] = useState('');
  const [snackbar, setSnackbar] = useState('');
  const [loading, setLoading] = useState(false);
  const [ticketsEnEspera, setTicketsEnEspera] = useState<TicketEnEspera[]>([]);
  const [openEspera, setOpenEspera] = useState(false);
  const [referenciaEspera, setReferenciaEspera] = useState('');
  const [openRecuperar, setOpenRecuperar] = useState(false);

  const sucursalId = user?.sucursalId ?? '';
  const canUseCredito = user?.role === 'SUPERADMIN' || user?.role === 'ADMIN';

  const fetchProductos = async () => {
    if (!sucursalId) return;
    const data = await invoke<ProductoInventario[]>('get_productos_por_sucursal', { sucursalId });
    setProductos(data);
  };

  useEffect(() => {
    fetchProductos().catch((error) => console.error('Error productos:', error));
  }, [sucursalId]);

  useEffect(() => {
    if (!canUseCredito) return;
    invoke<Cliente[]>('get_clientes')
      .then((data) => setClientes(data))
      .catch((error) => console.error('Error clientes:', error));
  }, [canUseCredito]);

  const total = useMemo(
    () =>
      carrito.reduce((acc, row) => {
        const cantidad = Number(row.cantidad || 0);
        const precio = Number(row.precioVenta || 0);
        return acc + cantidad * precio;
      }, 0),
    [carrito],
  );

  const cambio = useMemo(() => Number(efectivoRecibido || 0) - total, [efectivoRecibido, total]);

  const addProducto = (producto: ProductoInventario) => {
    setCarrito((prev) => {
      const idx = prev.findIndex((item) => item.productoId === producto.id);
      if (idx >= 0) {
        return prev.map((item, index) =>
          index === idx ? { ...item, cantidad: String(Number(item.cantidad || 0) + 1) } : item,
        );
      }
      return [
        ...prev,
        {
          productoId: producto.id,
          descripcion: producto.descripcion,
          marca: producto.marca,
          cantidad: '1',
          precioVenta: String(producto.precioVenta ?? 0),
        },
      ];
    });
  };

  const handleBuscarEnter = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const q = busqueda.trim().toLowerCase();
    if (!q) return;
    const producto = productos.find(
      (item) =>
        item.descripcion.toLowerCase().includes(q) ||
        item.codigoBarras.toLowerCase().includes(q) ||
        item.codigoProveedor.toLowerCase().includes(q) ||
        item.claveProducto.toLowerCase().includes(q),
    );
    if (!producto) {
      setSnackbar('No se encontró el producto.');
      return;
    }
    addProducto(producto);
    setBusqueda('');
  };

  const updateRow = (productoId: string, field: 'cantidad' | 'precioVenta', value: string) => {
    setCarrito((prev) => prev.map((row) => (row.productoId === productoId ? { ...row, [field]: value } : row)));
  };

  const removeRow = (productoId: string) => {
    setCarrito((prev) => prev.filter((row) => row.productoId !== productoId));
  };

  const clearVenta = () => {
    setCarrito([]);
    setBusqueda('');
    setMetodoPago('EFECTIVO');
    setEfectivoRecibido('');
    setClienteId('');
  };

  const handleGuardarEnEspera = () => {
    if (carrito.length === 0) {
      setSnackbar('No hay productos para poner en espera.');
      return;
    }
    if (!referenciaEspera.trim()) {
      setSnackbar('Ingresa una referencia para el ticket en espera.');
      return;
    }

    const ticket: TicketEnEspera = {
      id: Date.now(),
      referencia: referenciaEspera.trim(),
      productos: carrito,
      total,
    };

    setTicketsEnEspera((prev) => [ticket, ...prev]);
    clearVenta();
    setOpenEspera(false);
    setReferenciaEspera('');
    setSnackbar('Ticket puesto en espera.');
  };

  const handleRecuperarTicket = (ticket: TicketEnEspera) => {
    setCarrito(ticket.productos);
    setTicketsEnEspera((prev) => prev.filter((item) => item.id !== ticket.id));
    setOpenRecuperar(false);
    setSnackbar('Ticket recuperado correctamente.');
  };

  const confirmarCobro = async () => {
    if (!user?.id || !sucursalId || carrito.length === 0) return;
    if (metodoPago === 'EFECTIVO' && cambio < 0) {
      setSnackbar('El efectivo recibido es insuficiente.');
      return;
    }
    if (metodoPago === 'CREDITO' && !clienteId) {
      setSnackbar('Selecciona un cliente para venta a crédito.');
      return;
    }

    setLoading(true);
    try {
      const payload: RegistrarVentaPayload = {
        id: crypto.randomUUID(),
        usuarioId: user.id,
        sucursalId,
        fecha: new Date().toISOString(),
        metodoPago,
        clienteId: metodoPago === 'CREDITO' ? clienteId : undefined,
        detalles: carrito.map((row) => ({
          id: crypto.randomUUID(),
          productoId: row.productoId,
          cantidad: Number(row.cantidad || 0),
          precioVentaPactado: Number(row.precioVenta || 0),
        })),
      };

      await invoke('registrar_venta', { venta: payload });
      setOpenCobrar(false);
      clearVenta();
      setSnackbar('Venta registrada correctamente.');
      fetchProductos().catch((error) => console.error('Error productos:', error));
    } catch (error) {
      console.error('Error al registrar venta:', error);
      setSnackbar(`Error al registrar venta: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3 }}>
        Punto de Venta
      </Typography>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box component="form" onSubmit={handleBuscarEnter}>
          <TextField
            label="Buscar por descripción, código de barras, código proveedor o clave. Presiona Enter para agregar."
            value={busqueda}
            onChange={(e) => setBusqueda(e.target.value)}
            fullWidth
            autoFocus
          />
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Producto</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Marca</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Cantidad</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Precio</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Importe</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {carrito.map((row) => (
                <TableRow key={row.productoId} hover>
                  <TableCell>{row.descripcion}</TableCell>
                  <TableCell>{row.marca || '-'}</TableCell>
                  <TableCell sx={{ width: 140 }}>
                    <TextField
                      size="small"
                      type="number"
                      value={row.cantidad}
                      onChange={(e) => updateRow(row.productoId, 'cantidad', e.target.value)}
                      slotProps={{ htmlInput: { min: 1, step: '0.01' } }}
                    />
                  </TableCell>
                  <TableCell sx={{ width: 180 }}>
                    <TextField
                      size="small"
                      type="number"
                      value={row.precioVenta}
                      onChange={(e) => updateRow(row.productoId, 'precioVenta', e.target.value)}
                      slotProps={{ htmlInput: { min: 0, step: '0.01' } }}
                    />
                  </TableCell>
                  <TableCell>${(Number(row.cantidad || 0) * Number(row.precioVenta || 0)).toFixed(2)}</TableCell>
                  <TableCell align="right">
                    <Button size="small" color="error" startIcon={<DeleteIcon />} onClick={() => removeRow(row.productoId)}>
                      Quitar
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
              {carrito.length === 0 && (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    Carrito vacío.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Paper elevation={0} sx={{ p: 3, borderRadius: 2, border: '1px solid', borderColor: 'divider', mt: 2 }}>
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 2, flexWrap: 'wrap' }}>
          <Typography variant="h3" sx={{ fontWeight: 800, color: 'primary.main' }}>
            ${total.toFixed(2)}
          </Typography>
          <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
            <Button
              variant="outlined"
              size="large"
              onClick={() => setOpenEspera(true)}
              disabled={carrito.length === 0}
            >
              Poner en Espera
            </Button>
            <Button
              variant="outlined"
              size="large"
              onClick={() => setOpenRecuperar(true)}
            >
              <Badge color="primary" badgeContent={ticketsEnEspera.length} max={99}>
                <Box component="span" sx={{ px: 1 }}>Recuperar Ticket</Box>
              </Badge>
            </Button>
            <Button
              variant="contained"
              size="large"
              startIcon={<PaymentsIcon />}
              onClick={() => setOpenCobrar(true)}
              disabled={carrito.length === 0}
            >
              Cobrar
            </Button>
          </Box>
        </Box>
      </Paper>

      <Dialog open={openEspera} onClose={() => setOpenEspera(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Poner ticket en espera</DialogTitle>
        <DialogContent sx={{ pt: 2, display: 'flex', flexDirection: 'column', gap: 2 }}>
          <DialogContentText>
            Agrega una referencia rápida para identificar este ticket.
          </DialogContentText>
          <TextField
            label="Referencia"
            value={referenciaEspera}
            onChange={(e) => setReferenciaEspera(e.target.value)}
            placeholder="Ej: Cliente de las mangueras"
            autoFocus
          />
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <Button onClick={() => setOpenEspera(false)}>Cancelar</Button>
          <Button variant="contained" onClick={handleGuardarEnEspera}>
            Confirmar
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={openRecuperar} onClose={() => setOpenRecuperar(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Tickets en espera</DialogTitle>
        <DialogContent sx={{ pt: 1 }}>
          {ticketsEnEspera.length === 0 ? (
            <Typography color="text.secondary">No hay tickets en espera.</Typography>
          ) : (
            <List>
              {ticketsEnEspera.map((ticket) => (
                <ListItemButton key={ticket.id} onClick={() => handleRecuperarTicket(ticket)}>
                  <ListItemText
                    primary={ticket.referencia}
                    secondary={`${new Date(ticket.id).toLocaleTimeString()} · Total: $${ticket.total.toFixed(2)}`}
                  />
                </ListItemButton>
              ))}
            </List>
          )}
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <Button onClick={() => setOpenRecuperar(false)}>Cerrar</Button>
        </DialogActions>
      </Dialog>

      <Dialog open={openCobrar} onClose={() => setOpenCobrar(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Cobrar venta</DialogTitle>
        <DialogContent sx={{ pt: 2, display: 'flex', flexDirection: 'column', gap: 2 }}>
          <TextField select label="Método de pago" value={metodoPago} onChange={(e) => setMetodoPago(e.target.value)}>
            <MenuItem value="EFECTIVO">Efectivo</MenuItem>
            <MenuItem value="TARJETA">Tarjeta</MenuItem>
            <MenuItem value="TRANSFERENCIA">Transferencia</MenuItem>
            {canUseCredito && <MenuItem value="CREDITO">Crédito</MenuItem>}
          </TextField>
          {metodoPago === 'CREDITO' && canUseCredito && (
            <TextField
              select
              label="Cliente"
              value={clienteId}
              onChange={(e) => setClienteId(e.target.value)}
            >
              {clientes.map((cliente) => (
                <MenuItem key={cliente.id} value={cliente.id}>
                  {cliente.nombre} - Saldo: ${cliente.saldoDeudor.toFixed(2)} / Límite: ${cliente.limiteCredito.toFixed(2)}
                </MenuItem>
              ))}
            </TextField>
          )}
          <TextField label="Total a cobrar" value={`$${total.toFixed(2)}`} disabled />
          <TextField
            label="Efectivo recibido"
            type="number"
            value={efectivoRecibido}
            onChange={(e) => setEfectivoRecibido(e.target.value)}
            disabled={metodoPago !== 'EFECTIVO'}
            slotProps={{ htmlInput: { min: 0, step: '0.01' } }}
          />
          <TextField label="Cambio" value={`$${Math.max(cambio, 0).toFixed(2)}`} disabled />
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <Button onClick={() => setOpenCobrar(false)}>Cancelar</Button>
          <Button variant="contained" onClick={confirmarCobro} disabled={loading}>
            Confirmar cobro
          </Button>
        </DialogActions>
      </Dialog>

      <Snackbar open={Boolean(snackbar)} autoHideDuration={3000} onClose={() => setSnackbar('')}>
        <Alert onClose={() => setSnackbar('')} severity={snackbar.startsWith('Error') ? 'error' : 'success'} variant="filled">
          {snackbar}
        </Alert>
      </Snackbar>
    </Box>
  );
}

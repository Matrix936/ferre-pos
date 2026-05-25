import { ReactNode, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Autocomplete,
  Badge,
  Box,
  Button,
  Chip,
  CircularProgress,
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
  AddCircleOutlined as AddCircleOutlineIcon,
  Delete as DeleteIcon,
  PauseCircleOutlined as PauseCircleOutlineIcon,
  Payments as PaymentsIcon,
  Search as SearchIcon,
  Undo as UndoIcon,
} from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { Cliente, ProductoInventario, RegistrarVentaPayload } from '../../inventario/types';
import { useBarcodeScanner } from '../../shared/hooks/useBarcodeScanner';
import { useDebouncedValue } from '../../shared/hooks/useDebouncedValue';

interface VentaRow {
  productoId: string;
  descripcion: string;
  marca: string;
  cantidad: string;
  precioVenta: string;
  precioOriginal?: number | null;
  precioDescontado?: number | null;
  nombrePromo?: string | null;
  promocionId?: string | null;
}

interface TicketEnEspera {
  id: number;
  referencia: string;
  productos: VentaRow[];
  total: number;
}

interface CajaActualResumen {
  sesion: {
    id: string;
    estado: string;
  };
}

const toCents = (value: number) => Math.round((Number.isFinite(value) ? value : 0) * 100);
const fromCents = (value: number) => value / 100;
const formatMoney = (value: number) => `$${fromCents(toCents(value)).toFixed(2)}`;
const getPrecioOriginal = (row: VentaRow) => Number(row.precioOriginal || row.precioVenta || 0);
const getPrecioFinal = (row: VentaRow) => Number(row.precioVenta || 0);
const getDescuentoUnitario = (row: VentaRow) => Math.max(0, getPrecioOriginal(row) - getPrecioFinal(row));
const isValidQuantity = (value: string) => {
  const trimmed = value.trim();
  if (!/^\d+(\.\d{0,3})?$/.test(trimmed)) return false;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) && parsed > 0;
};
const isValidMoney = (value: string) => {
  const trimmed = value.trim();
  if (!/^\d+(\.\d{0,2})?$/.test(trimmed)) return false;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) && parsed > 0;
};

function ShortcutHint({ keys, label }: { keys: string; label: string }) {
  return (
    <Box
      component="span"
      sx={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: 0.75,
        color: 'text.secondary',
        fontSize: '0.78rem',
        whiteSpace: 'nowrap',
      }}
    >
      <Box
        component="kbd"
        sx={{
          px: 0.75,
          py: 0.2,
          borderRadius: 0.75,
          border: '1px solid',
          borderColor: 'divider',
          bgcolor: 'background.paper',
          color: 'text.primary',
          fontFamily: 'inherit',
          fontSize: '0.72rem',
          fontWeight: 700,
          lineHeight: 1.4,
        }}
      >
        {keys}
      </Box>
      {label}
    </Box>
  );
}

function ButtonContentWithShortcut({ children, shortcut }: { children: ReactNode; shortcut: string }) {
  return (
    <Box component="span" sx={{ display: 'inline-flex', alignItems: 'center', gap: 1 }}>
      {children}
      <Box
        component="kbd"
        sx={{
          px: 0.65,
          py: 0.1,
          borderRadius: 0.75,
          border: '1px solid',
          borderColor: 'currentColor',
          fontFamily: 'inherit',
          fontSize: '0.68rem',
          fontWeight: 700,
          lineHeight: 1.35,
          opacity: 0.72,
        }}
      >
        {shortcut}
      </Box>
    </Box>
  );
}

export function NuevaVenta() {
  const { user } = useAuth();
  const [carrito, setCarrito] = useState<VentaRow[]>([]);
  const [busqueda, setBusqueda] = useState('');
  const [opcionesBusqueda, setOpcionesBusqueda] = useState<ProductoInventario[]>([]);
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
  const [cajaAbierta, setCajaAbierta] = useState(false);
  const [openVentaRapida, setOpenVentaRapida] = useState(false);
  const [ventaRapidaDescripcion, setVentaRapidaDescripcion] = useState('');
  const [ventaRapidaCantidad, setVentaRapidaCantidad] = useState('1');
  const [ventaRapidaPrecio, setVentaRapidaPrecio] = useState('');
  const [selectedRowIndex, setSelectedRowIndex] = useState<number | null>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const referenciaInputRef = useRef<HTMLInputElement>(null);
  const efectivoInputRef = useRef<HTMLInputElement>(null);
  const lastProductoAddedRef = useRef<{ id: string; at: number } | null>(null);

  const sucursalId = user?.sucursalId ?? '';
  const canUseCredito = user?.role === 'SUPERADMIN' || user?.role === 'ADMIN';
  const canUseVentaRapida = canUseCredito;
  const busquedaDebounced = useDebouncedValue(busqueda, 300);

  const fetchProductos = async () => {
    if (!sucursalId) return;
    await invoke<ProductoInventario[]>('get_productos_por_sucursal', { sucursalId });
  };

  const fetchCajaActual = async () => {
    if (!user?.id || !sucursalId) {
      setCajaAbierta(false);
      return;
    }
    const data = await invoke<CajaActualResumen | null>('get_caja_actual', {
      usuarioId: user.id,
      sucursalId,
    });
    setCajaAbierta(Boolean(data && data.sesion.estado === 'ABIERTA'));
  };

  useEffect(() => {
    fetchProductos().catch((error) => console.error('Error productos:', error));
  }, [sucursalId]);

  useEffect(() => {
    fetchCajaActual().catch((error) => {
      console.error('Error caja actual:', error);
      setCajaAbierta(false);
    });
  }, [user?.id, sucursalId]);

  useEffect(() => {
    if (!canUseCredito) return;
    invoke<Cliente[]>('get_clientes')
      .then((data) => setClientes(data))
      .catch((error) => console.error('Error clientes:', error));
  }, [canUseCredito]);

  const total = useMemo(
    () =>
      fromCents(carrito.reduce((acc, row) => {
        const cantidad = Number(row.cantidad || 0);
        const precio = Number(row.precioVenta || 0);
        return acc + toCents(cantidad * precio);
      }, 0)),
    [carrito],
  );
  const selectedCliente = useMemo(
    () => clientes.find((cliente) => cliente.id === clienteId) ?? null,
    [clientes, clienteId],
  );
  const creditoDisponible = selectedCliente
    ? fromCents(toCents(selectedCliente.limiteCredito) - toCents(selectedCliente.saldoDeudor))
    : 0;
  const creditoInsuficiente =
    metodoPago === 'CREDITO' &&
    Boolean(selectedCliente) &&
    toCents(total) > toCents(Math.max(creditoDisponible, 0));

  const subtotalSinDescuento = useMemo(
    () =>
      fromCents(carrito.reduce((acc, row) => {
        const cantidad = Number(row.cantidad || 0);
        return acc + toCents(cantidad * getPrecioOriginal(row));
      }, 0)),
    [carrito],
  );

  const ahorroTotal = useMemo(
    () =>
      fromCents(carrito.reduce((acc, row) => {
        const cantidad = Number(row.cantidad || 0);
        if (!row.nombrePromo) return acc;
        return acc + toCents(getDescuentoUnitario(row) * cantidad);
      }, 0)),
    [carrito],
  );

  const productosConPromocion = useMemo(
    () => carrito.filter((row) => row.nombrePromo && getDescuentoUnitario(row) > 0).length,
    [carrito],
  );

  const cambio = useMemo(() => fromCents(toCents(Number(efectivoRecibido || 0)) - toCents(total)), [efectivoRecibido, total]);
  const carritoInvalido = useMemo(
    () => carrito.some((row) => !isValidQuantity(row.cantidad) || !isValidMoney(row.precioVenta)),
    [carrito],
  );
  const efectivoFormatoInvalido = metodoPago === 'EFECTIVO' && !/^\d+(\.\d{0,2})?$/.test(efectivoRecibido.trim() || '0');
  const cobroBloqueado =
    loading ||
    carrito.length === 0 ||
    !cajaAbierta ||
    carritoInvalido ||
    efectivoFormatoInvalido ||
    (metodoPago === 'EFECTIVO' && cambio < 0) ||
    (metodoPago === 'CREDITO' && (!clienteId || creditoInsuficiente));

  const focusSearch = () => {
    window.setTimeout(() => searchInputRef.current?.focus(), 0);
  };

  const addProducto = (producto: ProductoInventario) => {
    setCarrito((prev) => {
      const idx = prev.findIndex((item) => item.productoId === producto.id);
      if (idx >= 0) {
        setSelectedRowIndex(idx);
        return prev.map((item, index) =>
          index === idx
            ? {
                ...item,
                descripcion: producto.descripcion,
                marca: producto.marca,
                cantidad: String(Number(item.cantidad || 0) + 1),
                precioVenta: String(producto.precioVenta ?? item.precioVenta ?? 0),
                precioOriginal: producto.precioOriginal ?? null,
                precioDescontado: producto.precioDescontado ?? null,
                nombrePromo: producto.nombrePromo ?? null,
                promocionId: producto.promocionId ?? null,
              }
            : item,
        );
      }
      setSelectedRowIndex(prev.length);
      return [
        ...prev,
        {
          productoId: producto.id,
          descripcion: producto.descripcion,
          marca: producto.marca,
          cantidad: '1',
          precioVenta: String(producto.precioVenta ?? 0),
          precioOriginal: producto.precioOriginal ?? null,
          precioDescontado: producto.precioDescontado ?? null,
          nombrePromo: producto.nombrePromo ?? null,
          promocionId: producto.promocionId ?? null,
        },
      ];
    });
  };

  const refreshCarritoPromociones = async () => {
    if (!sucursalId || carrito.length === 0) return;
    const productosActualizados = await invoke<ProductoInventario[]>('get_productos_por_sucursal', { sucursalId });
    const index = new Map(productosActualizados.map((producto) => [producto.id, producto]));
    setCarrito((prev) =>
      prev.map((row) => {
        if (row.productoId === 'VENTA-DIVERSA') return row;
        const producto = index.get(row.productoId);
        if (!producto) return row;
        return {
          ...row,
          descripcion: producto.descripcion,
          marca: producto.marca,
          precioVenta: String(producto.precioVenta ?? row.precioVenta ?? 0),
          precioOriginal: producto.precioOriginal ?? null,
          precioDescontado: producto.precioDescontado ?? null,
          nombrePromo: producto.nombrePromo ?? null,
          promocionId: producto.promocionId ?? null,
        };
      }),
    );
  };

  const prepararCobro = async () => {
    if (loading || carrito.length === 0 || !cajaAbierta) return;
    if (carritoInvalido) {
      setSnackbar('Revisa cantidades y precios antes de cobrar.');
      return;
    }
    try {
      await refreshCarritoPromociones();
    } catch (error) {
      console.error('Error refrescando promociones antes de cobrar:', error);
    }
    setOpenCobrar(true);
  };

  const addProductoFromSearch = (producto: ProductoInventario) => {
    const now = Date.now();
    const last = lastProductoAddedRef.current;
    if (last?.id === producto.id && now - last.at < 180) {
      return;
    }
    lastProductoAddedRef.current = { id: producto.id, at: now };
    addProducto(producto);
  };

  const handleBarcodeScan = useCallback(
    async (code: string) => {
      if (!sucursalId) return;
      if (!cajaAbierta) {
        setSnackbar('Abre caja antes de escanear productos.');
        return;
      }

      try {
        const data = await invoke<ProductoInventario[]>('buscar_productos_por_sucursal', {
          sucursalId,
          query: code,
        });
        const normalizedCode = code.trim().toLowerCase();
        const producto =
          data.find(
            (item) =>
              item.codigoBarras.toLowerCase() === normalizedCode ||
              item.codigoProveedor.toLowerCase() === normalizedCode ||
              item.claveProducto.toLowerCase() === normalizedCode,
          ) ?? data[0];

        if (!producto) {
          setSnackbar(`No se encontró el código ${code}.`);
          return;
        }

        addProductoFromSearch(producto);
        setBusqueda('');
        setOpcionesBusqueda([]);
      } catch (error) {
        console.error('Error al leer código de barras:', error);
        setSnackbar(`Error al leer código de barras: ${error}`);
      }
    },
    [sucursalId, cajaAbierta],
  );

  useBarcodeScanner(handleBarcodeScan, {
    enabled: Boolean(sucursalId),
    maxDelayMs: 50,
    minLength: 3,
  });

  useEffect(() => {
    const q = busquedaDebounced.trim();
    if (!sucursalId || q.length <= 2) {
      setOpcionesBusqueda([]);
      return;
    }
    let active = true;
    invoke<ProductoInventario[]>('buscar_productos_por_sucursal', { sucursalId, query: q })
      .then((data) => {
        if (active) setOpcionesBusqueda(data);
      })
      .catch((error) => {
        console.error('Error buscando sugerencias:', error);
        if (active) setOpcionesBusqueda([]);
      });
    return () => {
      active = false;
    };
  }, [sucursalId, busquedaDebounced]);

  const handleAgregarDesdeBusqueda = (rawValue: string) => {
    const q = rawValue.trim().toLowerCase();
    if (!q) return;
    const producto = opcionesBusqueda.find(
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
    addProductoFromSearch(producto);
    setBusqueda('');
    setOpcionesBusqueda([]);
  };

  const updateRow = (productoId: string, field: 'cantidad', value: string) => {
    setCarrito((prev) => prev.map((row) => (row.productoId === productoId ? { ...row, [field]: value } : row)));
  };

  const removeRow = (productoId: string) => {
    setCarrito((prev) => prev.filter((row) => row.productoId !== productoId));
    setSelectedRowIndex(null);
  };

  const clearVenta = () => {
    setCarrito([]);
    setBusqueda('');
    setMetodoPago('EFECTIVO');
    setEfectivoRecibido('');
    setClienteId('');
    setSelectedRowIndex(null);
  };

  const handleAgregarVentaRapida = async () => {
    if (!canUseVentaRapida || !sucursalId) return;
    const cantidad = Number(ventaRapidaCantidad || 0);
    const precio = Number(ventaRapidaPrecio || 0);
    if (!ventaRapidaDescripcion.trim()) {
      setSnackbar('Describe el artículo diverso.');
      return;
    }
    if (cantidad <= 0 || precio <= 0) {
      setSnackbar('Cantidad y precio deben ser mayores a cero.');
      return;
    }
    const existing = carrito.find((row) => row.productoId === 'VENTA-DIVERSA');
    if (existing && Number(existing.precioVenta || 0) !== fromCents(toCents(precio))) {
      setSnackbar('Solo puede haber un artículo diverso por ticket si cambia el precio.');
      return;
    }

    try {
      const producto = await invoke<ProductoInventario>('asegurar_producto_venta_diversa', { sucursalId });
      const precioNormalizado = fromCents(toCents(precio));
      setCarrito((prev) => {
        const idx = prev.findIndex((row) => row.productoId === producto.id);
        if (idx >= 0) {
          setSelectedRowIndex(idx);
          return prev.map((row, index) =>
            index === idx
              ? {
                  ...row,
                  descripcion: row.descripcion || ventaRapidaDescripcion.trim(),
                  cantidad: String(Number(row.cantidad || 0) + cantidad),
                  precioVenta: String(precioNormalizado),
                }
              : row,
          );
        }
        setSelectedRowIndex(prev.length);
        return [
          ...prev,
          {
            productoId: producto.id,
            descripcion: ventaRapidaDescripcion.trim(),
            marca: producto.marca || 'Sin marca',
            cantidad: String(cantidad),
            precioVenta: String(precioNormalizado),
          },
        ];
      });
      setVentaRapidaDescripcion('');
      setVentaRapidaCantidad('1');
      setVentaRapidaPrecio('');
      setOpenVentaRapida(false);
      setSnackbar('Artículo diverso agregado.');
    } catch (error) {
      setSnackbar(`Error al agregar artículo diverso: ${error}`);
    }
  };

  const handleNuevaVenta = () => {
    if (carrito.length === 0 && !busqueda && !clienteId && !efectivoRecibido) {
      focusSearch();
      return;
    }
    clearVenta();
    setSnackbar('Venta limpia. Lista para capturar.');
    focusSearch();
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
    if (loading) return;
    if (!user?.id || !sucursalId || carrito.length === 0) return;
    if (!cajaAbierta) {
      setSnackbar('No puedes cobrar porque la caja está cerrada.');
      return;
    }
    if (carritoInvalido) {
      setSnackbar('Revisa cantidades y precios antes de cobrar.');
      return;
    }
    if (efectivoFormatoInvalido) {
      setSnackbar('El efectivo recibido debe tener máximo 2 decimales.');
      return;
    }
    if (metodoPago === 'EFECTIVO' && cambio < 0) {
      setSnackbar('El efectivo recibido es insuficiente.');
      return;
    }
    if (metodoPago === 'CREDITO' && !clienteId) {
      setSnackbar('Selecciona un cliente para venta a crédito.');
      return;
    }
    if (creditoInsuficiente) {
      setSnackbar('El crédito disponible del cliente no alcanza para esta venta.');
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
        efectivoRecibido: metodoPago === 'EFECTIVO' ? fromCents(toCents(Number(efectivoRecibido || 0))) : undefined,
        cambioEntregado: metodoPago === 'EFECTIVO' ? fromCents(toCents(Math.max(cambio, 0))) : undefined,
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
      fetchCajaActual().catch((error) => console.error('Error caja actual:', error));
      focusSearch();
    } catch (error) {
      console.error('Error al registrar venta:', error);
      setSnackbar(`Error al registrar venta: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    if (openEspera) {
      window.setTimeout(() => referenciaInputRef.current?.focus(), 0);
    }
  }, [openEspera]);

  useEffect(() => {
    if (openCobrar && metodoPago === 'EFECTIVO') {
      window.setTimeout(() => efectivoInputRef.current?.focus(), 0);
    }
  }, [openCobrar, metodoPago]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      const isTyping = target?.tagName === 'INPUT' || target?.tagName === 'TEXTAREA' || target?.getAttribute('role') === 'combobox';
      const key = event.key.toUpperCase();
      if (!isTyping && key === 'ARROWDOWN') {
        event.preventDefault();
        setSelectedRowIndex((prev) => {
          if (carrito.length === 0) return null;
          return prev === null ? 0 : Math.min(prev + 1, carrito.length - 1);
        });
        return;
      }

      if (!isTyping && key === 'ARROWUP') {
        event.preventDefault();
        setSelectedRowIndex((prev) => {
          if (carrito.length === 0) return null;
          return prev === null ? 0 : Math.max(prev - 1, 0);
        });
        return;
      }

      if (!isTyping && key === 'DELETE' && selectedRowIndex !== null && carrito[selectedRowIndex]) {
        event.preventDefault();
        removeRow(carrito[selectedRowIndex].productoId);
        return;
      }

      if (!key.startsWith('F')) return;

      if (key === 'F1') {
        event.preventDefault();
        if (canUseVentaRapida) setOpenVentaRapida(true);
        return;
      }

      if (key === 'F2') {
        event.preventDefault();
        setOpenCobrar(false);
        setOpenEspera(false);
        setOpenRecuperar(false);
        setOpenVentaRapida(false);
        focusSearch();
        return;
      }

      if (key === 'F4') {
        event.preventDefault();
        if (!loading && carrito.length > 0 && cajaAbierta) void prepararCobro();
        return;
      }

      if (key === 'F12') {
        event.preventDefault();
        if (!loading && carrito.length > 0 && cajaAbierta) void prepararCobro();
        return;
      }

      if (key === 'F6') {
        event.preventDefault();
        if (carrito.length > 0) setOpenEspera(true);
        return;
      }

      if (key === 'F7') {
        event.preventDefault();
        setOpenRecuperar(true);
        return;
      }

      if (key === 'F8') {
        event.preventDefault();
        handleNuevaVenta();
        return;
      }

      if (key === 'F10') {
        event.preventDefault();
        if (loading) return;
        if (openCobrar) {
          confirmarCobro();
          return;
        }
        if (openEspera) {
          handleGuardarEnEspera();
          return;
        }
        if (openVentaRapida) {
          handleAgregarVentaRapida();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [carrito, selectedRowIndex, busqueda, clienteId, efectivoRecibido, openCobrar, openEspera, openVentaRapida, metodoPago, cambio, total, loading, cajaAbierta, canUseVentaRapida, ventaRapidaDescripcion, ventaRapidaCantidad, ventaRapidaPrecio, carritoInvalido, efectivoFormatoInvalido, creditoInsuficiente]);

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2, height: 'calc(100vh - 112px)', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: 2, mb: 3, flexWrap: 'wrap', flexShrink: 0 }}>
        <Box>
          <Typography variant="h5" sx={{ fontWeight: 700 }}>
            Punto de Venta
          </Typography>
          <Stack direction="row" spacing={1.5} useFlexGap sx={{ mt: 1, flexWrap: 'wrap' }}>
            {canUseVentaRapida && <ShortcutHint keys="F1" label="Artículo diverso" />}
            <ShortcutHint keys="F2" label="Buscar" />
            <ShortcutHint keys="F4" label="Cobrar" />
            <ShortcutHint keys="F6" label="Espera" />
            <ShortcutHint keys="F7" label="Recuperar" />
            <ShortcutHint keys="F8" label="Nueva venta" />
            <ShortcutHint keys="F10" label="Confirmar" />
            <ShortcutHint keys="F12" label="Cobro rápido" />
          </Stack>
        </Box>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2, flexShrink: 0 }}>
        {!cajaAbierta && (
          <Alert severity="warning" sx={{ mb: 2 }}>
            Caja cerrada. Puedes preparar el carrito, pero abre caja antes de cobrar.
          </Alert>
        )}
        <Box>
          <Autocomplete
            freeSolo
            options={opcionesBusqueda}
            getOptionLabel={(option) => (typeof option === 'string' ? option : option.descripcion)}
            filterOptions={(options) => options}
            noOptionsText="Escribe al menos 3 caracteres para buscar coincidencias"
            inputValue={busqueda}
            onInputChange={(_, value, reason) => {
              if (reason === 'reset') {
                setBusqueda('');
                setOpcionesBusqueda([]);
                return;
              }
              setBusqueda(value);
              if (reason !== 'input' || !value.trim()) {
                setOpcionesBusqueda([]);
              }
            }}
            onChange={(_, value) => {
              if (!value) return;
              if (typeof value === 'string') {
                handleAgregarDesdeBusqueda(value);
                return;
              }
              addProductoFromSearch(value);
              setBusqueda('');
              setOpcionesBusqueda([]);
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
                    {option.nombrePromo && option.precioOriginal ? (
                      <Box component="span" sx={{ display: 'inline-flex', alignItems: 'center', gap: 1, flexWrap: 'wrap', justifyContent: 'flex-end' }}>
                        <Chip
                          label={option.nombrePromo}
                          color="success"
                          size="small"
                          sx={{ height: 20, borderRadius: '6px', fontWeight: 700 }}
                        />
                        <Box component="span" sx={{ color: 'text.secondary', textDecoration: 'line-through', fontWeight: 500 }}>
                          ${option.precioOriginal.toFixed(2)}
                        </Box>
                        <Box component="span">${option.precioVenta.toFixed(2)}</Box>
                      </Box>
                    ) : (
                      <>${option.precioVenta.toFixed(2)}</>
                    )}
                  </Typography>
                </Box>
              </Box>
            )}
            renderInput={(params) => (
              <TextField
                {...params}
                label="Buscar producto"
                placeholder="Descripción, código de barras, código proveedor o clave"
                autoFocus
                inputRef={searchInputRef}
                onKeyDown={(event) => {
                  if (event.key === 'Enter' && busqueda.trim()) {
                    const highlightedOption = (event.target as HTMLInputElement).getAttribute('aria-activedescendant');
                    if (highlightedOption) {
                      return;
                    }
                    event.preventDefault();
                    handleAgregarDesdeBusqueda(busqueda);
                  }
                }}
                helperText="F2 enfoca este campo. Enter agrega la coincidencia más cercana."
              />
            )}
          />
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden', flex: 1, minHeight: 0 }}>
        <TableContainer sx={{ maxHeight: '100%', overflow: 'auto' }}>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Producto</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Marca</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Cantidad</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Precio</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Descuento</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Importe</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {carrito.map((row, index) => (
                <TableRow
                  key={`${row.productoId}-${index}`}
                  hover
                  selected={selectedRowIndex === index}
                  onClick={() => setSelectedRowIndex(index)}
                >
                  <TableCell>
                    <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flexWrap: 'wrap' }}>
                      <Box component="span">{row.descripcion}</Box>
                      {row.nombrePromo && (
                        <Chip
                          label={row.nombrePromo}
                          size="small"
                          color="success"
                          sx={{ height: 22, borderRadius: '6px', fontWeight: 700 }}
                        />
                      )}
                    </Box>
                  </TableCell>
                  <TableCell>{row.marca || '-'}</TableCell>
                  <TableCell sx={{ width: 140 }}>
                    <TextField
                      size="small"
                      type="number"
                      value={row.cantidad}
                      onChange={(e) => updateRow(row.productoId, 'cantidad', e.target.value)}
                      error={Boolean(row.cantidad) && !isValidQuantity(row.cantidad)}
                      helperText={Boolean(row.cantidad) && !isValidQuantity(row.cantidad) ? 'Cantidad inválida' : ' '}
                      slotProps={{ htmlInput: { min: 0.001, step: '0.001', inputMode: 'decimal' } }}
                    />
                  </TableCell>
                  <TableCell sx={{ width: 180 }}>
                    {row.nombrePromo && row.precioOriginal ? (
                      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.25 }}>
                        <Typography variant="caption" sx={{ color: 'text.secondary', textDecoration: 'line-through' }}>
                          {formatMoney(row.precioOriginal)}
                        </Typography>
                        <Typography variant="body2" sx={{ fontWeight: 800 }}>
                          {formatMoney(Number(row.precioVenta || 0))}
                        </Typography>
                      </Box>
                    ) : (
                      <Typography variant="body2" sx={{ fontWeight: 700 }}>
                        {formatMoney(Number(row.precioVenta || 0))}
                      </Typography>
                    )}
                  </TableCell>
                  <TableCell sx={{ width: 150 }}>
                    {row.nombrePromo && getDescuentoUnitario(row) > 0 ? (
                      <Stack spacing={0.25}>
                        <Typography variant="body2" sx={{ color: 'success.main', fontWeight: 800 }}>
                          -{formatMoney(getDescuentoUnitario(row) * Number(row.cantidad || 0))}
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          {formatMoney(getDescuentoUnitario(row))} c/u
                        </Typography>
                      </Stack>
                    ) : (
                      <Typography variant="body2" color="text.secondary">-</Typography>
                    )}
                  </TableCell>
                  <TableCell>{formatMoney(Number(row.cantidad || 0) * Number(row.precioVenta || 0))}</TableCell>
                  <TableCell align="right">
                    <Button size="small" color="error" startIcon={<DeleteIcon />} onClick={() => removeRow(row.productoId)}>
                      Quitar
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
              {carrito.length === 0 && (
                <TableRow>
                  <TableCell colSpan={7} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    Carrito vacío.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Paper
        elevation={0}
        sx={{
          p: 3,
          borderRadius: 2,
          border: '1px solid',
          borderColor: 'divider',
          mt: 2,
          flexShrink: 0,
          position: 'sticky',
          bottom: 0,
          zIndex: 2,
          bgcolor: 'background.paper',
        }}
      >
        <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 2, flexWrap: 'wrap' }}>
          <Box>
            {ahorroTotal > 0 && (
              <Stack direction="row" spacing={1} sx={{ mb: 0.5, alignItems: 'center' }}>
                <Typography variant="body2" color="text.secondary">
                  Subtotal:
                </Typography>
                <Typography variant="body2" sx={{ textDecoration: 'line-through', color: 'text.secondary', fontWeight: 700 }}>
                  {formatMoney(subtotalSinDescuento)}
                </Typography>
                <Chip
                  label={`${productosConPromocion} ${productosConPromocion === 1 ? 'promo' : 'promos'}`}
                  color="success"
                  size="small"
                  sx={{ height: 22, borderRadius: '6px', fontWeight: 800 }}
                />
              </Stack>
            )}
            <Typography variant="h3" sx={{ fontWeight: 800, color: 'primary.main' }}>
              {formatMoney(total)}
            </Typography>
            {ahorroTotal > 0 && (
              <Typography variant="body2" sx={{ color: 'success.main', fontWeight: 700 }}>
                Ahorro total: {formatMoney(ahorroTotal)}
              </Typography>
            )}
          </Box>
          <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
            {canUseVentaRapida && (
              <Button
                variant="outlined"
                size="large"
                startIcon={<AddCircleOutlineIcon />}
                onClick={() => setOpenVentaRapida(true)}
              >
                <ButtonContentWithShortcut shortcut="F1">Artículo diverso</ButtonContentWithShortcut>
              </Button>
            )}
            <Button
              variant="outlined"
              size="large"
              startIcon={<PauseCircleOutlineIcon />}
              onClick={() => setOpenEspera(true)}
              disabled={carrito.length === 0}
            >
              <ButtonContentWithShortcut shortcut="F6">Poner en Espera</ButtonContentWithShortcut>
            </Button>
            <Button
              variant="outlined"
              size="large"
              startIcon={<UndoIcon />}
              onClick={() => setOpenRecuperar(true)}
            >
              <Badge color="primary" badgeContent={ticketsEnEspera.length} max={99}>
                <Box component="span" sx={{ px: 1 }}>
                  <ButtonContentWithShortcut shortcut="F7">Recuperar Ticket</ButtonContentWithShortcut>
                </Box>
              </Badge>
            </Button>
            <Button
              variant="outlined"
              size="large"
              startIcon={<SearchIcon />}
              onClick={handleNuevaVenta}
            >
              <ButtonContentWithShortcut shortcut="F8">Nueva venta</ButtonContentWithShortcut>
            </Button>
            <Button
              variant="contained"
              size="large"
              startIcon={<PaymentsIcon />}
              onClick={() => void prepararCobro()}
              disabled={loading || carrito.length === 0 || !cajaAbierta || carritoInvalido}
            >
              <ButtonContentWithShortcut shortcut="F4">Cobrar</ButtonContentWithShortcut>
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
            inputRef={referenciaInputRef}
            autoFocus
          />
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <Button onClick={() => setOpenEspera(false)}>Cancelar</Button>
          <Button variant="contained" onClick={handleGuardarEnEspera}>
            <ButtonContentWithShortcut shortcut="F10">Confirmar</ButtonContentWithShortcut>
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
                    secondary={`${new Date(ticket.id).toLocaleTimeString()} · Total: ${formatMoney(ticket.total)}`}
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

      <Dialog open={openVentaRapida} onClose={() => setOpenVentaRapida(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Artículo diverso</DialogTitle>
        <DialogContent sx={{ '&&': { pt: 2.5 }, display: 'flex', flexDirection: 'column', gap: 2 }}>
          <Alert severity="info">
            Uso controlado para artículos sin código. Solo para personal autorizado.
          </Alert>
          <TextField
            label="Descripción"
            value={ventaRapidaDescripcion}
            onChange={(e) => setVentaRapidaDescripcion(e.target.value)}
            placeholder="Ej: Cable por metro, tornillería surtida"
            autoFocus
          />
          <TextField
            label="Cantidad"
            type="number"
            value={ventaRapidaCantidad}
            onChange={(e) => setVentaRapidaCantidad(e.target.value)}
            slotProps={{ htmlInput: { min: 0.01, step: '0.01' } }}
          />
          <TextField
            label="Precio neto"
            type="number"
            value={ventaRapidaPrecio}
            onChange={(e) => setVentaRapidaPrecio(e.target.value)}
            slotProps={{ htmlInput: { min: 0.01, step: '0.01' } }}
          />
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <Button onClick={() => setOpenVentaRapida(false)}>Cancelar</Button>
          <Button
            variant="contained"
            onClick={handleAgregarVentaRapida}
            disabled={!ventaRapidaDescripcion.trim() || Number(ventaRapidaCantidad || 0) <= 0 || Number(ventaRapidaPrecio || 0) <= 0}
          >
            Agregar
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={openCobrar} onClose={loading ? undefined : () => setOpenCobrar(false)} maxWidth="xs" fullWidth>
        <DialogTitle sx={{ pb: 1 }}>Cobrar venta</DialogTitle>
        <DialogContent sx={{ '&&': { pt: 2.5 }, display: 'flex', flexDirection: 'column', gap: 2 }}>
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
              error={creditoInsuficiente}
              helperText={
                selectedCliente
                  ? creditoInsuficiente
                    ? `Crédito disponible insuficiente: ${formatMoney(Math.max(creditoDisponible, 0))}.`
                    : `Disponible: ${formatMoney(Math.max(creditoDisponible, 0))}.`
                  : 'Selecciona un cliente con crédito autorizado.'
              }
            >
              {clientes.map((cliente) => {
                const disponible = fromCents(toCents(cliente.limiteCredito) - toCents(cliente.saldoDeudor));
                const sinCredito = cliente.limiteCredito <= 0 || disponible <= 0;
                return (
                <MenuItem key={cliente.id} value={cliente.id} disabled={sinCredito}>
                  {cliente.nombre} - Disponible: {formatMoney(Math.max(disponible, 0))}
                </MenuItem>
                );
              })}
            </TextField>
          )}
          {ahorroTotal > 0 && (
            <Paper
              elevation={0}
              sx={{
                p: 1.5,
                borderRadius: 1.5,
                border: '1px solid',
                borderColor: 'success.main',
                bgcolor: (theme) => theme.palette.mode === 'dark' ? 'rgba(46, 125, 50, 0.12)' : 'rgba(46, 125, 50, 0.08)',
              }}
            >
              <Stack spacing={0.75}>
                <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 2 }}>
                  <Typography variant="body2" color="text.secondary">Subtotal sin descuento</Typography>
                  <Typography variant="body2" sx={{ fontWeight: 700, textDecoration: 'line-through', color: 'text.secondary' }}>
                    {formatMoney(subtotalSinDescuento)}
                  </Typography>
                </Box>
                <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 2 }}>
                  <Typography variant="body2" sx={{ color: 'success.main', fontWeight: 700 }}>Descuentos aplicados</Typography>
                  <Typography variant="body2" sx={{ color: 'success.main', fontWeight: 800 }}>
                    -{formatMoney(ahorroTotal)}
                  </Typography>
                </Box>
              </Stack>
            </Paper>
          )}
          <TextField label="Total a cobrar" value={formatMoney(total)} disabled />
          <TextField
            label="Efectivo recibido"
            type="number"
            value={efectivoRecibido}
            onChange={(e) => setEfectivoRecibido(e.target.value)}
            disabled={metodoPago !== 'EFECTIVO'}
            inputRef={efectivoInputRef}
            error={efectivoFormatoInvalido || (metodoPago === 'EFECTIVO' && cambio < 0)}
            helperText={
              efectivoFormatoInvalido
                ? 'Usa máximo 2 decimales.'
                : metodoPago === 'EFECTIVO' && cambio < 0
                  ? 'Efectivo insuficiente.'
                  : ' '
            }
            slotProps={{ htmlInput: { min: 0, step: '0.01', inputMode: 'decimal' } }}
          />
          <TextField label="Cambio" value={formatMoney(Math.max(cambio, 0))} disabled />
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <Button onClick={() => setOpenCobrar(false)} disabled={loading}>Cancelar</Button>
          <Button
            variant="contained"
            onClick={confirmarCobro}
            disabled={cobroBloqueado}
            startIcon={loading ? <CircularProgress size={18} color="inherit" /> : undefined}
          >
            {loading ? 'Cobrando...' : <ButtonContentWithShortcut shortcut="F10">Confirmar cobro</ButtonContentWithShortcut>}
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

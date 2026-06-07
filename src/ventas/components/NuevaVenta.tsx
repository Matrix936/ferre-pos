import { ReactNode, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Autocomplete,
  Badge,
  Box,
  Button,
  Chip,
  Checkbox,
  DialogContentText,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  List,
  ListItemButton,
  ListItemText,
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
  AddCircleOutlined as AddCircleOutlineIcon,
  Delete as DeleteIcon,
  PauseCircleOutlined as PauseCircleOutlineIcon,
  Payments as PaymentsIcon,
  ReceiptLong as ReceiptLongIcon,
  Search as SearchIcon,
  Undo as UndoIcon,
} from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { useConfig } from '../../config/context/ConfigContext';
import { Cliente, ProductoInventario, RegistrarVentaPayload } from '../../inventario/types';
import { AsyncButton } from '../../shared/components/AsyncButton';
import { FeedbackSeverity, FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { TablePager } from '../../shared/components/TablePager';
import { useBarcodeScanner } from '../../shared/hooks/useBarcodeScanner';
import { useDebouncedValue } from '../../shared/hooks/useDebouncedValue';
import { useDialogHotkeys } from '../../shared/hooks/useDialogHotkeys';
import { useLocalPagination } from '../../shared/hooks/useLocalPagination';
import { dialogActionsSx, dialogContentSx } from '../../shared/ui/patterns';
import { buildEscposLogoRaster } from '../utils/escposLogo';

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
  promoTipoDescuento?: string | null;
  promoValor?: number | null;
  costoPromedio?: number;
  precioMostrador?: number;
  precio1?: number;
  precio2?: number;
  precio3?: number;
  precio4?: number;
  mayoreoApartir?: number;
  aGranel?: boolean;
  noEnCatalogo?: boolean;
  ventasNegativas?: boolean;
  tipoPrecioVendido?: string;
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

interface PerifericosConfig {
  impresoraTickets: string;
  impresoraEtiquetas: string;
  updatedAt: string;
}

interface EmpresaConfigFiscal {
  rfc: string;
  razonSocial: string;
  regimenFiscal: string;
  registroPatronal?: string | null;
  actualizadoAt: string;
}

const toCents = (value: number) => Math.round((Number.isFinite(value) ? value : 0) * 100);
const fromCents = (value: number) => value / 100;
const formatMoney = (value: number) => `$${fromCents(toCents(value)).toFixed(2)}`;
const getPrecioOriginal = (row: VentaRow) => Number(row.precioOriginal || row.precioVenta || 0);
const getPrecioFinal = (row: VentaRow) => Number(row.precioVenta || 0);
const getDescuentoUnitario = (row: VentaRow) => Math.max(0, getPrecioOriginal(row) - getPrecioFinal(row));
const isPrecioMayoreo = (row: VentaRow) => row.tipoPrecioVendido?.includes('MAYOREO') ?? false;
const precioConPromo = (precio: number, tipo?: string | null, valor?: number | null) => {
  if (!tipo || !valor || valor <= 0) return precio;
  if (tipo === 'PORCENTAJE') return fromCents(toCents(precio * (1 - valor / 100)));
  if (tipo === 'MONTO_FIJO') return fromCents(toCents(Math.max(0, precio - valor)));
  return precio;
};
const aplicarPromoPreview = (row: VentaRow, precioBase: number, tipoBase: string) => {
  const precioPromo = precioConPromo(precioBase, row.promoTipoDescuento, row.promoValor);
  const costo = Number(row.costoPromedio || 0);
  const promoValida = Boolean(row.promocionId && row.nombrePromo) && precioPromo < precioBase && precioPromo + 0.0001 >= costo;
  if (!promoValida) {
    return {
      precioVenta: precioBase,
      precioOriginal: tipoBase === 'MOSTRADOR' ? row.precioOriginal ?? null : row.precioMostrador ?? precioBase,
      tipoPrecioVendido: tipoBase,
    };
  }
  return {
    precioVenta: precioPromo,
    precioOriginal: precioBase,
    tipoPrecioVendido: `${tipoBase}+PROMO`,
  };
};
const resolvePrecioVentaRow = (row: VentaRow, cantidad: number) => {
  const precioMostrador = row.precioMostrador ?? row.precioOriginal ?? Number(row.precioVenta || 0);
  const mayoreo = Number(row.mayoreoApartir || 0);
  if (mayoreo > 0 && cantidad >= mayoreo) {
    const precioMayoreo = [row.precio1, row.precio2, row.precio3, row.precio4]
      .map((value) => Number(value || 0))
      .find((value) => value > 0 && value < precioMostrador);
    if (precioMayoreo) {
      return aplicarPromoPreview(row, precioMayoreo, 'MAYOREO');
    }
  }
  return aplicarPromoPreview(row, precioMostrador, 'MOSTRADOR');
};
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
  const { sucursales } = useCatalogos();
  const { logo } = useConfig();
  const [carrito, setCarrito] = useState<VentaRow[]>([]);
  const [busqueda, setBusqueda] = useState('');
  const [opcionesBusqueda, setOpcionesBusqueda] = useState<ProductoInventario[]>([]);
  const [metodoPago, setMetodoPago] = useState('EFECTIVO');
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [clienteId, setClienteId] = useState('');
  const [requiereFactura, setRequiereFactura] = useState(false);
  const [clienteRapidoNombre, setClienteRapidoNombre] = useState('');
  const [clienteRapidoTelefono, setClienteRapidoTelefono] = useState('');
  const [clienteRapidoDomicilio, setClienteRapidoDomicilio] = useState('');
  const [openCobrar, setOpenCobrar] = useState(false);
  const [efectivoRecibido, setEfectivoRecibido] = useState('');
  const [snackbar, setSnackbar] = useState('');
  const [snackbarSeverity, setSnackbarSeverity] = useState<FeedbackSeverity>('success');
  const [loading, setLoading] = useState(false);
  const [openConfirmarVenta, setOpenConfirmarVenta] = useState(false);
  const [recordarConfirmacionVenta, setRecordarConfirmacionVenta] = useState(false);
  const [ticketsEnEspera, setTicketsEnEspera] = useState<TicketEnEspera[]>([]);
  const [openEspera, setOpenEspera] = useState(false);
  const [referenciaEspera, setReferenciaEspera] = useState('');
  const [openRecuperar, setOpenRecuperar] = useState(false);
  const [cajaAbierta, setCajaAbierta] = useState(false);
  const [checkingCaja, setCheckingCaja] = useState(true);
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

  const fetchCajaActual = async () => {
    setCheckingCaja(true);
    if (!user?.id || !sucursalId) {
      setCajaAbierta(false);
      setCheckingCaja(false);
      return;
    }
    try {
      const data = await invoke<CajaActualResumen | null>('get_caja_actual', {
        usuarioId: user.id,
        sucursalId,
      });
      setCajaAbierta(Boolean(data && data.sesion.estado === 'ABIERTA'));
    } finally {
      setCheckingCaja(false);
    }
  };

  useEffect(() => {
    fetchCajaActual().catch((error) => {
      console.error('Error caja actual:', error);
      setCajaAbierta(false);
      setCheckingCaja(false);
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
        if (getDescuentoUnitario(row) <= 0) return acc;
        return acc + toCents(getDescuentoUnitario(row) * cantidad);
      }, 0)),
    [carrito],
  );

  const productosConDescuento = useMemo(
    () => carrito.filter((row) => getDescuentoUnitario(row) > 0).length,
    [carrito],
  );
  const carritoPager = useLocalPagination(carrito);
  const totalPiezasCarrito = useMemo(
    () => fromCents(carrito.reduce((acc, row) => acc + toCents(Number(row.cantidad || 0)), 0)),
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
    checkingCaja ||
    carrito.length === 0 ||
    !cajaAbierta ||
    carritoInvalido ||
    efectivoFormatoInvalido ||
    (metodoPago === 'EFECTIVO' && cambio < 0) ||
    (metodoPago === 'CREDITO' && (!clienteId || creditoInsuficiente));

  const focusSearch = () => {
    window.setTimeout(() => searchInputRef.current?.focus(), 0);
  };

  const showToast = (message: string, severity: 'success' | 'info' | 'warning' | 'error' = 'success') => {
    setSnackbarSeverity(severity);
    setSnackbar(message);
  };

  const addProducto = (producto: ProductoInventario) => {
    setCarrito((prev) => {
      const idx = prev.findIndex((item) => item.productoId === producto.id);
      if (idx >= 0) {
        setSelectedRowIndex(idx);
        return prev.map((item, index) => {
          if (index !== idx) return item;
          const cantidad = Number(item.cantidad || 0) + 1;
          const baseRow: VentaRow = {
            ...item,
            descripcion: producto.descripcion,
            marca: producto.marca,
            cantidad: String(cantidad),
            precioVenta: String(producto.precioVenta ?? item.precioVenta ?? 0),
            precioOriginal: producto.precioOriginal ?? null,
            precioDescontado: producto.precioDescontado ?? null,
            nombrePromo: producto.nombrePromo ?? null,
            promocionId: producto.promocionId ?? null,
            promoTipoDescuento: producto.promoTipoDescuento ?? null,
            promoValor: producto.promoValor ?? null,
            costoPromedio: producto.costoPromedio ?? 0,
            precioMostrador: producto.precioOriginal ?? producto.precioVenta ?? 0,
            precio1: producto.precio1 ?? 0,
            precio2: producto.precio2 ?? 0,
            precio3: producto.precio3 ?? 0,
            precio4: producto.precio4 ?? 0,
            mayoreoApartir: producto.mayoreoApartir ?? 0,
            aGranel: producto.aGranel ?? false,
            noEnCatalogo: producto.noEnCatalogo ?? false,
            ventasNegativas: producto.ventasNegativas ?? false,
            tipoPrecioVendido: producto.nombrePromo ? 'MOSTRADOR+PROMO' : 'MOSTRADOR',
          };
          const resolved = resolvePrecioVentaRow(baseRow, cantidad);
          return {
            ...baseRow,
            precioVenta: String(resolved.precioVenta),
            precioOriginal: resolved.precioOriginal,
            tipoPrecioVendido: resolved.tipoPrecioVendido,
          };
        });
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
          promoTipoDescuento: producto.promoTipoDescuento ?? null,
          promoValor: producto.promoValor ?? null,
          costoPromedio: producto.costoPromedio ?? 0,
          precioMostrador: producto.precioOriginal ?? producto.precioVenta ?? 0,
          precio1: producto.precio1 ?? 0,
          precio2: producto.precio2 ?? 0,
          precio3: producto.precio3 ?? 0,
          precio4: producto.precio4 ?? 0,
          mayoreoApartir: producto.mayoreoApartir ?? 0,
          aGranel: producto.aGranel ?? false,
          noEnCatalogo: producto.noEnCatalogo ?? false,
          ventasNegativas: producto.ventasNegativas ?? false,
          tipoPrecioVendido: producto.nombrePromo ? 'MOSTRADOR+PROMO' : 'MOSTRADOR',
        },
      ];
    });
  };

  const refreshCarritoPromociones = async () => {
    if (!sucursalId || carrito.length === 0) return;
    const productosActualizados = await Promise.all(
      carrito
        .filter((row) => row.productoId !== 'VENTA-DIVERSA')
        .map(async (row) => {
          const results = await invoke<ProductoInventario[]>('buscar_productos_por_sucursal', {
            sucursalId,
            query: row.productoId,
          });
          return results.find((producto) => producto.id === row.productoId) ?? null;
        }),
    );
    const index = new Map(productosActualizados.filter(Boolean).map((producto) => [producto!.id, producto!]));
    setCarrito((prev) =>
      prev.map((row) => {
        if (row.productoId === 'VENTA-DIVERSA') return row;
        const producto = index.get(row.productoId);
        if (!producto) return row;
        const baseRow: VentaRow = {
          ...row,
          descripcion: producto.descripcion,
          marca: producto.marca,
          precioVenta: String(producto.precioVenta ?? row.precioVenta ?? 0),
          precioOriginal: producto.precioOriginal ?? null,
          precioDescontado: producto.precioDescontado ?? null,
          nombrePromo: producto.nombrePromo ?? null,
          promocionId: producto.promocionId ?? null,
          promoTipoDescuento: producto.promoTipoDescuento ?? null,
          promoValor: producto.promoValor ?? null,
          costoPromedio: producto.costoPromedio ?? 0,
          precioMostrador: producto.precioOriginal ?? producto.precioVenta ?? 0,
          precio1: producto.precio1 ?? 0,
          precio2: producto.precio2 ?? 0,
          precio3: producto.precio3 ?? 0,
          precio4: producto.precio4 ?? 0,
          mayoreoApartir: producto.mayoreoApartir ?? 0,
          aGranel: producto.aGranel ?? false,
          noEnCatalogo: producto.noEnCatalogo ?? false,
          ventasNegativas: producto.ventasNegativas ?? false,
        };
        const resolved = resolvePrecioVentaRow(baseRow, Number(baseRow.cantidad || 0));
        return { ...baseRow, precioVenta: String(resolved.precioVenta), precioOriginal: resolved.precioOriginal, tipoPrecioVendido: resolved.tipoPrecioVendido };
      }),
    );
  };

  const prepararCobro = async () => {
    if (loading || checkingCaja || carrito.length === 0 || !cajaAbierta) return;
    if (carritoInvalido) {
      showToast('Revisa cantidades y precios antes de cobrar.', 'warning');
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
        showToast('Abre caja antes de escanear productos.', 'warning');
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
          showToast(`No se encontró el código ${code}.`, 'warning');
          return;
        }

        addProductoFromSearch(producto);
        setBusqueda('');
        setOpcionesBusqueda([]);
      } catch (error) {
        console.error('Error al leer código de barras:', error);
        showToast(`Error al leer código de barras: ${error}`, 'error');
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
      showToast('No se encontró el producto.', 'warning');
      return;
    }
    addProductoFromSearch(producto);
    setBusqueda('');
    setOpcionesBusqueda([]);
  };

  const updateRow = (productoId: string, field: 'cantidad', value: string) => {
    setCarrito((prev) =>
      prev.map((row) => {
        if (row.productoId !== productoId) return row;
        const updated = { ...row, [field]: value };
        const cantidad = Number(value || 0);
        if (!Number.isFinite(cantidad) || cantidad <= 0) return updated;
        const resolved = resolvePrecioVentaRow(updated, cantidad);
        return {
          ...updated,
          precioVenta: String(resolved.precioVenta),
          precioOriginal: resolved.precioOriginal,
          tipoPrecioVendido: resolved.tipoPrecioVendido,
        };
      }),
    );
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
    setRequiereFactura(false);
    setClienteRapidoNombre('');
    setClienteRapidoTelefono('');
    setClienteRapidoDomicilio('');
    setSelectedRowIndex(null);
  };

  const handleAgregarVentaRapida = async () => {
    if (!canUseVentaRapida || !sucursalId) return;
    const cantidad = Number(ventaRapidaCantidad || 0);
    const precio = Number(ventaRapidaPrecio || 0);
    if (!ventaRapidaDescripcion.trim()) {
      showToast('Describe el artículo diverso.', 'warning');
      return;
    }
    if (cantidad <= 0 || precio <= 0) {
      showToast('Cantidad y precio deben ser mayores a cero.', 'warning');
      return;
    }
    const existing = carrito.find((row) => row.productoId === 'VENTA-DIVERSA');
    if (existing && Number(existing.precioVenta || 0) !== fromCents(toCents(precio))) {
      showToast('Solo puede haber un artículo diverso por ticket si cambia el precio.', 'warning');
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
                  precioOriginal: precioNormalizado,
                  tipoPrecioVendido: 'DIVERSO',
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
            precioOriginal: precioNormalizado,
            tipoPrecioVendido: 'DIVERSO',
          },
        ];
      });
      setVentaRapidaDescripcion('');
      setVentaRapidaCantidad('1');
      setVentaRapidaPrecio('');
      setOpenVentaRapida(false);
      showToast('Artículo diverso agregado.', 'success');
    } catch (error) {
      showToast(`Error al agregar artículo diverso: ${error}`, 'error');
    }
  };

  const handleNuevaVenta = () => {
    if (carrito.length === 0 && !busqueda && !clienteId && !efectivoRecibido) {
      focusSearch();
      return;
    }
    clearVenta();
    showToast('Venta limpia. Lista para capturar.', 'info');
    focusSearch();
  };

  const handleGuardarEnEspera = () => {
    if (carrito.length === 0) {
      showToast('No hay productos para poner en espera.', 'warning');
      return;
    }
    if (!referenciaEspera.trim()) {
      showToast('Ingresa una referencia para el ticket en espera.', 'warning');
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
    showToast('Ticket puesto en espera.', 'success');
  };

  const imprimirTicketVenta = async (venta: RegistrarVentaPayload) => {
    const [config, empresaConfig] = await Promise.all([
      invoke<PerifericosConfig>('get_perifericos_config'),
      invoke<EmpresaConfigFiscal>('get_empresa_config').catch(() => null),
    ]);
    const printerName = config.impresoraTickets?.trim();
    if (!printerName) {
      throw new Error('No hay impresora de tickets configurada.');
    }
    const sucursal = sucursales.find((item) => item.id === venta.sucursalId);
    const logoBytes = await buildEscposLogoRaster(logo, 42).catch(() => undefined);
    await invoke('imprimir_ticket_y_abrir_caja', {
      printerName,
      paperWidth: 42,
      abrirCajon: metodoPago === 'EFECTIVO',
      ticket: {
        folio: venta.id.slice(0, 8).toUpperCase(),
        fecha: new Date(venta.fecha).toLocaleString(),
        cajero: user?.nombre ?? '',
        sucursal: sucursal?.nombre ?? sucursalId,
        logoBytes,
        empresaNombre: empresaConfig?.razonSocial,
        rfc: empresaConfig?.rfc,
        regimenFiscal: empresaConfig?.regimenFiscal,
        codigoPostal: sucursal?.codigoPostal,
        metodoPago,
        estado: 'COMPLETADA',
        productos: carrito.map((row) => {
          const cantidad = Number(row.cantidad || 0);
          const precio = Number(row.precioVenta || 0);
          return {
            descripcion: row.descripcion,
            marca: row.marca,
            cantidad,
            precioUnitario: precio,
            importe: fromCents(toCents(cantidad * precio)),
          };
        }),
        subtotal: subtotalSinDescuento,
        descuento: ahorroTotal,
        total,
        recibido: metodoPago === 'EFECTIVO' ? fromCents(toCents(Number(efectivoRecibido || 0))) : undefined,
        cambio: metodoPago === 'EFECTIVO' ? fromCents(toCents(Math.max(cambio, 0))) : undefined,
        mensaje: 'Gracias por su compra',
      },
    });
    return printerName;
  };

  const handleRecuperarTicket = (ticket: TicketEnEspera) => {
    setCarrito(ticket.productos);
    setTicketsEnEspera((prev) => prev.filter((item) => item.id !== ticket.id));
    setOpenRecuperar(false);
    showToast('Ticket recuperado correctamente.', 'success');
  };

  const confirmarCobro = async (confirmacionForzada = false) => {
    if (loading) return;
    if (!user?.id || !sucursalId || carrito.length === 0) return;
    if (!cajaAbierta) {
      showToast('No puedes cobrar porque la caja está cerrada.', 'warning');
      return;
    }
    if (carritoInvalido) {
      showToast('Revisa cantidades y precios antes de cobrar.', 'warning');
      return;
    }
    if (efectivoFormatoInvalido) {
      showToast('El efectivo recibido debe tener máximo 2 decimales.', 'warning');
      return;
    }
    if (metodoPago === 'EFECTIVO' && cambio < 0) {
      showToast('El efectivo recibido es insuficiente.', 'warning');
      return;
    }
    if (metodoPago === 'CREDITO' && !clienteId) {
      showToast('Selecciona un cliente para venta a crédito.', 'warning');
      return;
    }
    if (creditoInsuficiente) {
      showToast('El crédito disponible del cliente no alcanza para esta venta.', 'warning');
      return;
    }
    if (!confirmacionForzada && !recordarConfirmacionVenta) {
      setOpenConfirmarVenta(true);
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
        clienteRapidoNombre: clienteRapidoNombre.trim() || undefined,
        clienteRapidoTelefono: clienteRapidoTelefono.trim() || undefined,
        clienteRapidoDomicilio: clienteRapidoDomicilio.trim() || undefined,
        requiereFactura,
        detalles: carrito.map((row) => ({
          id: crypto.randomUUID(),
          productoId: row.productoId,
          cantidad: Number(row.cantidad || 0),
          precioVentaPactado: Number(row.precioVenta || 0),
          tipoPrecioVendido: row.tipoPrecioVendido ?? (row.nombrePromo ? 'MOSTRADOR+PROMO' : 'MOSTRADOR'),
          precioOriginal: getPrecioOriginal(row),
          descuentoAplicado: getDescuentoUnitario(row),
        })),
      };

      await invoke('registrar_venta', { venta: payload });
      try {
        const printerName = await imprimirTicketVenta(payload);
        showToast(`Venta registrada. Ticket enviado a ${printerName}.`, 'success');
      } catch (printError) {
        showToast(`Venta registrada, pero la impresora de tickets no está disponible. ${String(printError)}`, 'error');
      }
      setOpenCobrar(false);
      setOpenConfirmarVenta(false);
      clearVenta();
      fetchCajaActual().catch((error) => console.error('Error caja actual:', error));
      focusSearch();
    } catch (error) {
      console.error('Error al registrar venta:', error);
      showToast(`Error al registrar venta: ${error}`, 'error');
    } finally {
      setLoading(false);
    }
  };

  const esperaDisabled = !referenciaEspera.trim();
  const ventaRapidaDisabled =
    !ventaRapidaDescripcion.trim() || Number(ventaRapidaCantidad || 0) <= 0 || Number(ventaRapidaPrecio || 0) <= 0;
  useDialogHotkeys({
    open: openEspera,
    disabled: esperaDisabled,
    onConfirm: handleGuardarEnEspera,
    onCancel: () => setOpenEspera(false),
  });
  useDialogHotkeys({
    open: openVentaRapida,
    disabled: ventaRapidaDisabled,
    onConfirm: handleAgregarVentaRapida,
    onCancel: () => setOpenVentaRapida(false),
  });
  useDialogHotkeys({
    open: openCobrar,
    disabled: cobroBloqueado || openConfirmarVenta,
    cancelDisabled: loading,
    onConfirm: () => void confirmarCobro(),
    onCancel: () => setOpenCobrar(false),
  });
  useDialogHotkeys({
    open: openConfirmarVenta,
    disabled: loading,
    cancelDisabled: loading,
    onConfirm: () => {
      setOpenConfirmarVenta(false);
      void confirmarCobro(true);
    },
    onCancel: () => setOpenConfirmarVenta(false),
  });

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
        if (!loading && !checkingCaja && carrito.length > 0 && cajaAbierta) void prepararCobro();
        return;
      }

      if (key === 'F12') {
        event.preventDefault();
        if (!loading && !checkingCaja && carrito.length > 0 && cajaAbierta) void prepararCobro();
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
        if (openConfirmarVenta) {
          setOpenConfirmarVenta(false);
          void confirmarCobro(true);
          return;
        }
        if (openCobrar) {
          void confirmarCobro();
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
  }, [carrito, selectedRowIndex, busqueda, clienteId, efectivoRecibido, openConfirmarVenta, openCobrar, openEspera, openVentaRapida, metodoPago, cambio, total, loading, checkingCaja, cajaAbierta, canUseVentaRapida, ventaRapidaDescripcion, ventaRapidaCantidad, ventaRapidaPrecio, carritoInvalido, efectivoFormatoInvalido, creditoInsuficiente]);

  return (
    <Box sx={{ width: '100%', mt: 2, height: 'calc(100vh - 112px)', display: 'flex', flexDirection: 'column', minHeight: 0 }}>
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
        {checkingCaja && (
          <Alert severity="info" sx={{ mb: 2 }}>
            Verificando estado de caja...
          </Alert>
        )}
        {!checkingCaja && !cajaAbierta && (
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
                <TableCell sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {carritoPager.paginatedRows.map((row, pageIndex) => {
                const index = carritoPager.startIndex + pageIndex;
                return (
                <TableRow
                  key={`${row.productoId}-${index}`}
                  hover
                  selected={selectedRowIndex === index}
                  onClick={() => setSelectedRowIndex(index)}
                >
                  <TableCell>
                    <Box component="span">{row.descripcion}</Box>
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
                    {getDescuentoUnitario(row) > 0 ? (
                      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.25 }}>
                        <Stack direction="row" spacing={0.75} sx={{ alignItems: 'center', flexWrap: 'wrap' }}>
                          {row.nombrePromo && (
                            <Chip
                              label={row.nombrePromo}
                              size="small"
                              color="success"
                              sx={{ height: 22, borderRadius: '6px', fontWeight: 700 }}
                            />
                          )}
                          {isPrecioMayoreo(row) && (
                            <Chip
                              label="Mayoreo"
                              size="small"
                              color="primary"
                              sx={{ height: 22, borderRadius: '6px', fontWeight: 700 }}
                            />
                          )}
                        </Stack>
                        <Typography variant="caption" sx={{ color: 'text.secondary', textDecoration: 'line-through' }}>
                          {formatMoney(getPrecioOriginal(row))}
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
                    {getDescuentoUnitario(row) > 0 ? (
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
                  <TableCell>
                    <Button size="small" color="error" startIcon={<DeleteIcon />} onClick={() => removeRow(row.productoId)}>
                      Quitar
                    </Button>
                  </TableCell>
                </TableRow>
                );
              })}
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
        <TablePager
          page={carritoPager.page}
          pageSize={carritoPager.pageSize}
          totalPages={carritoPager.totalPages}
          totalRows={carritoPager.totalRows}
          fromRow={carritoPager.fromRow}
          toRow={carritoPager.toRow}
          canPreviousPage={carritoPager.canPreviousPage}
          canNextPage={carritoPager.canNextPage}
          onPreviousPage={carritoPager.previousPage}
          onNextPage={carritoPager.nextPage}
          onPageSizeChange={carritoPager.setPageSize}
          summary={`${carrito.length} ${carrito.length === 1 ? 'artículo' : 'artículos'} (${totalPiezasCarrito.toFixed(3).replace(/\.?0+$/, '')} piezas en total)`}
        />
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
                  label={`${productosConDescuento} ${productosConDescuento === 1 ? 'descuento' : 'descuentos'}`}
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
              disabled={loading || checkingCaja || carrito.length === 0 || !cajaAbierta || carritoInvalido}
            >
              <ButtonContentWithShortcut shortcut="F4">Cobrar</ButtonContentWithShortcut>
            </Button>
          </Box>
        </Box>
      </Paper>

      <Dialog open={openEspera} onClose={() => setOpenEspera(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Poner ticket en espera</DialogTitle>
        <DialogContent sx={dialogContentSx}>
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
        <DialogActions sx={dialogActionsSx}>
          <Button onClick={() => setOpenEspera(false)}>Cancelar</Button>
          <Button variant="contained" onClick={handleGuardarEnEspera} disabled={esperaDisabled}>
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
        <DialogContent sx={dialogContentSx}>
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
        <DialogActions sx={dialogActionsSx}>
          <Button onClick={() => setOpenVentaRapida(false)}>Cancelar</Button>
          <Button
            variant="contained"
            onClick={handleAgregarVentaRapida}
            disabled={ventaRapidaDisabled}
          >
            Agregar
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={openCobrar} onClose={loading ? undefined : () => setOpenCobrar(false)} maxWidth="xs" fullWidth>
        <DialogTitle sx={{ pb: 1 }}>Cobrar venta</DialogTitle>
        <DialogContent sx={dialogContentSx}>
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
          <FormControlLabel
            control={<Checkbox checked={requiereFactura} onChange={(e) => setRequiereFactura(e.target.checked)} />}
            label="El cliente requiere factura"
          />
          {requiereFactura && metodoPago !== 'CREDITO' && (
            <Paper
              elevation={0}
              sx={{
                p: 1.5,
                borderRadius: 1.5,
                border: '1px solid',
                borderColor: 'divider',
                display: 'grid',
                gap: 1.5,
              }}
            >
              <Typography variant="body2" sx={{ fontWeight: 700 }}>
                Datos rápidos del cliente
              </Typography>
              <TextField
                label="Nombre"
                value={clienteRapidoNombre}
                onChange={(e) => setClienteRapidoNombre(e.target.value)}
                fullWidth
              />
              <TextField
                label="Teléfono"
                value={clienteRapidoTelefono}
                onChange={(e) => setClienteRapidoTelefono(e.target.value)}
                fullWidth
              />
              <TextField
                label="Domicilio"
                value={clienteRapidoDomicilio}
                onChange={(e) => setClienteRapidoDomicilio(e.target.value)}
                multiline
                minRows={2}
                fullWidth
              />
            </Paper>
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
        <DialogActions sx={dialogActionsSx}>
          <Button onClick={() => setOpenCobrar(false)} disabled={loading}>Cancelar</Button>
          <AsyncButton
            variant="contained"
            onClick={() => void confirmarCobro()}
            disabled={cobroBloqueado}
            loading={loading}
            loadingText="Cobrando..."
          >
            <ButtonContentWithShortcut shortcut="F10">Confirmar cobro</ButtonContentWithShortcut>
          </AsyncButton>
        </DialogActions>
      </Dialog>

      <Dialog open={openConfirmarVenta} onClose={loading ? undefined : () => setOpenConfirmarVenta(false)} maxWidth="xs" fullWidth>
        <DialogTitle sx={{ display: 'flex', alignItems: 'center', gap: 1.25, pb: 1 }}>
          <ReceiptLongIcon color="primary" />
          Confirmar venta
        </DialogTitle>
        <DialogContent sx={dialogContentSx}>
          <DialogContentText>
            Antes de registrar la venta, confirma que el método de pago y el total sean correctos.
          </DialogContentText>
          <Paper
            elevation={0}
            sx={{
              p: 1.5,
              borderRadius: 1.5,
              border: '1px solid',
              borderColor: 'divider',
              bgcolor: 'background.default',
            }}
          >
            <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 2, mb: 0.75 }}>
              <Typography variant="body2" color="text.secondary">Método</Typography>
              <Typography variant="body2" sx={{ fontWeight: 700 }}>{metodoPago}</Typography>
            </Box>
            <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 2 }}>
              <Typography variant="body2" color="text.secondary">Total</Typography>
              <Typography variant="h6" sx={{ fontWeight: 800 }}>{formatMoney(total)}</Typography>
            </Box>
          </Paper>
          <FormControlLabel
            control={
              <Checkbox
                checked={recordarConfirmacionVenta}
                onChange={(event) => setRecordarConfirmacionVenta(event.target.checked)}
              />
            }
            label="Recordar mi elección durante esta sesión"
          />
        </DialogContent>
        <DialogActions sx={dialogActionsSx}>
          <Button onClick={() => setOpenConfirmarVenta(false)} disabled={loading}>Revisar</Button>
          <AsyncButton
            variant="contained"
            onClick={() => {
              setOpenConfirmarVenta(false);
              void confirmarCobro(true);
            }}
            loading={loading}
            loadingText="Registrando..."
          >
            <ButtonContentWithShortcut shortcut="F10">Proceder con la venta</ButtonContentWithShortcut>
          </AsyncButton>
        </DialogActions>
      </Dialog>

      <FeedbackSnackbar message={snackbar} severity={snackbarSeverity} onClose={() => setSnackbar('')} />
    </Box>
  );
}

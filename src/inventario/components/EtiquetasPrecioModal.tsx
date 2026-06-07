import { useMemo, useState } from 'react';
import {
  Alert,
  Box,
  Button,
  Checkbox,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  FormControlLabel,
  Paper,
  Stack,
  TextField,
  Typography,
} from '@mui/material';
import { Print as PrintIcon } from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/core';
import { useConfig } from '../../config/context/ConfigContext';
import { AsyncButton } from '../../shared/components/AsyncButton';
import { useDialogHotkeys } from '../../shared/hooks/useDialogHotkeys';
import { dialogActionsSx, dialogContentSx } from '../../shared/ui/patterns';
import { ProductoInventario } from '../types';

interface EtiquetasPrecioModalProps {
  open: boolean;
  productos: ProductoInventario[];
  onClose: () => void;
}

interface LabelOptions {
  mostrarNegocio: boolean;
  mostrarDescripcion: boolean;
  mostrarCodigoBarras: boolean;
  mostrarClave: boolean;
  mostrarPrecio: boolean;
}

interface PerifericosConfig {
  impresoraTickets: string;
  impresoraEtiquetas: string;
  updatedAt: string;
}

const labelFont = "'Arial Narrow', Arial, sans-serif";
const defaultOptions: LabelOptions = {
  mostrarNegocio: true,
  mostrarDescripcion: true,
  mostrarCodigoBarras: true,
  mostrarClave: true,
  mostrarPrecio: true,
};

const formatMoney = (value: number) => `$${(Number.isFinite(value) ? value : 0).toFixed(2)}`;

function BarcodeSvg({ value }: { value: string }) {
  const bars = useMemo(() => {
    const source = value.trim() || 'SIN-CODIGO';
    return Array.from(source).flatMap((char, index) => {
      const code = char.charCodeAt(0);
      return Array.from({ length: 7 }, (_, bit) => ({
        black: ((code >> bit) & 1) === 1,
        width: bit % 3 === 0 ? 2 : 1,
        key: `${index}-${bit}`,
      }));
    });
  }, [value]);
  const totalWidth = bars.reduce((sum, bar) => sum + bar.width + 1, 0);
  let cursor = 0;

  return (
    <Box
      component="svg"
      viewBox={`0 0 ${Math.max(totalWidth, 120)} 42`}
      preserveAspectRatio="none"
      sx={{ width: '100%', height: 42, display: 'block' }}
    >
      <rect x="0" y="0" width={Math.max(totalWidth, 120)} height="42" fill="#fff" />
      {bars.map((bar) => {
        const x = cursor;
        cursor += bar.width + 1;
        return bar.black ? <rect key={bar.key} x={x} y="2" width={bar.width} height="38" fill="#111" /> : null;
      })}
    </Box>
  );
}

function EtiquetaPreview({
  producto,
  negocio,
  options,
}: {
  producto: ProductoInventario;
  negocio: string;
  options: LabelOptions;
}) {
  const codigo = producto.codigoBarras || producto.codigoProveedor || producto.claveProducto || producto.id;

  return (
    <Paper
      elevation={4}
      sx={{
        width: 290,
        height: 160,
        p: 1.25,
        bgcolor: '#fff',
        color: '#111',
        border: '1px solid #d0d5dd',
        borderRadius: 1,
        display: 'flex',
        flexDirection: 'column',
        gap: 0.4,
        fontFamily: labelFont,
      }}
    >
      {options.mostrarNegocio && (
        <Typography sx={{ fontFamily: labelFont, fontSize: 12, fontWeight: 800, textAlign: 'center', lineHeight: 1 }}>
          {negocio}
        </Typography>
      )}

      {options.mostrarDescripcion && (
        <Typography
          sx={{
            fontFamily: labelFont,
            fontSize: 14,
            fontWeight: 800,
            lineHeight: 1.05,
            minHeight: options.mostrarNegocio ? 30 : 42,
            overflow: 'hidden',
            textAlign: 'center',
          }}
        >
          {producto.descripcion || 'Producto sin descripción'}
        </Typography>
      )}

      <Box sx={{ flex: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 1 }}>
        {(options.mostrarCodigoBarras || options.mostrarClave) && (
          <Box sx={{ flex: options.mostrarPrecio ? 1.35 : 1, minWidth: 0 }}>
            {options.mostrarCodigoBarras && <BarcodeSvg value={codigo} />}
            {options.mostrarClave && (
              <Typography sx={{ fontFamily: 'monospace', fontSize: 10, textAlign: 'center', lineHeight: 1, mt: 0.25 }}>
                {producto.claveProducto || producto.codigoProveedor || codigo}
              </Typography>
            )}
          </Box>
        )}

        {options.mostrarPrecio && (
          <Box sx={{ flex: options.mostrarCodigoBarras || options.mostrarClave ? 1 : 1.7, textAlign: 'right' }}>
            <Typography sx={{ fontFamily: labelFont, fontSize: 30, fontWeight: 900, lineHeight: 1 }}>
              {formatMoney(producto.precioVenta)}
            </Typography>
          </Box>
        )}
      </Box>
    </Paper>
  );
}

export function EtiquetasPrecioModal({ open, productos, onClose }: EtiquetasPrecioModalProps) {
  const { systemName } = useConfig();
  const [options, setOptions] = useState<LabelOptions>(defaultOptions);
  const [cantidad, setCantidad] = useState(1);
  const [printing, setPrinting] = useState(false);
  const [printError, setPrintError] = useState('');

  const productoPreview = productos[0] ?? null;
  const negocio = systemName?.trim() || 'Ferre-Materiales La Mixteca';
  const totalEtiquetas = Math.max(1, Math.min(500, Math.floor(cantidad || 1))) * productos.length;

  const updateOption = (key: keyof LabelOptions) => {
    setOptions((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const buildSilentLabelText = (producto: ProductoInventario) => {
    const codigo = producto.codigoBarras || producto.codigoProveedor || producto.claveProducto || producto.id;
    const lines = [
      options.mostrarNegocio ? negocio : '',
      options.mostrarDescripcion ? producto.descripcion : '',
      options.mostrarCodigoBarras ? `CODIGO: ${codigo}` : '',
      options.mostrarClave ? `CLAVE: ${producto.claveProducto || producto.codigoProveedor || codigo}` : '',
      options.mostrarPrecio ? `PRECIO: ${formatMoney(producto.precioVenta)}` : '',
    ].filter(Boolean);
    return `${lines.join('\n')}\n\n`;
  };

  const handlePrint = async () => {
    if (productos.length === 0) return;
    setPrinting(true);
    setPrintError('');
    const copies = Math.max(1, Math.min(500, Math.floor(cantidad || 1)));
    let printerName = '';
    try {
      const config = await invoke<PerifericosConfig>('get_perifericos_config');
      printerName = config.impresoraEtiquetas?.trim() ?? '';
      if (!printerName) {
        throw new Error('No hay impresora de etiquetas configurada.');
      }
      const contenido = productos
        .flatMap((producto) => Array.from({ length: copies }, () => buildSilentLabelText(producto)))
        .join('');
      await invoke('imprimir_silencioso', { input: { printerName, contenido } });
      onClose();
    } catch (error) {
      setPrintError(`La impresora ${printerName || 'Etiquetas'} no está conectada o no se encuentra disponible. ${String(error)}`);
    } finally {
      setPrinting(false);
    }
  };

  const printDisabled = printing || productos.length === 0;
  useDialogHotkeys({
    open,
    disabled: printDisabled,
    cancelDisabled: printing,
    onConfirm: handlePrint,
    onCancel: onClose,
  });

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle sx={{ fontWeight: 700 }}>Etiquetas de precio</DialogTitle>
      <Divider />
      <DialogContent sx={dialogContentSx}>
        {printError && <Alert severity="error" sx={{ mb: 2 }}>{printError}</Alert>}
        {productoPreview && (
          <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '280px 1fr' }, gap: 3, alignItems: 'start' }}>
            <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
              <Typography variant="subtitle2" sx={{ fontWeight: 800, mb: 1.5 }}>
                Contenido de la etiqueta
              </Typography>
              <Stack spacing={0.5}>
                <FormControlLabel control={<Checkbox checked={options.mostrarNegocio} onChange={() => updateOption('mostrarNegocio')} />} label="Negocio" />
                <FormControlLabel control={<Checkbox checked={options.mostrarDescripcion} onChange={() => updateOption('mostrarDescripcion')} />} label="Descripción" />
                <FormControlLabel control={<Checkbox checked={options.mostrarCodigoBarras} onChange={() => updateOption('mostrarCodigoBarras')} />} label="Código de barras" />
                <FormControlLabel control={<Checkbox checked={options.mostrarClave} onChange={() => updateOption('mostrarClave')} />} label="Clave interna" />
                <FormControlLabel control={<Checkbox checked={options.mostrarPrecio} onChange={() => updateOption('mostrarPrecio')} />} label="Precio" />
              </Stack>
              <TextField
                label="Cantidad por producto"
                type="number"
                value={cantidad}
                onChange={(event) => setCantidad(Math.max(1, Number(event.target.value || 1)))}
                fullWidth
                sx={{ mt: 2 }}
                helperText={`${totalEtiquetas} etiqueta${totalEtiquetas === 1 ? '' : 's'} en total`}
                slotProps={{ htmlInput: { min: 1, max: 500, step: 1 } }}
              />
            </Paper>

            <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'center', py: 2, gap: 1.5 }}>
              <EtiquetaPreview producto={productoPreview} negocio={negocio} options={options} />
              <Typography variant="caption" color="text.secondary">
                Vista previa con: {productoPreview.descripcion}
              </Typography>
            </Box>
          </Box>
        )}
      </DialogContent>
      <DialogActions sx={{ ...dialogActionsSx, p: 3, pt: 1 }}>
        <Button onClick={onClose} disabled={printing}>Cancelar</Button>
        <AsyncButton
          variant="contained"
          startIcon={<PrintIcon />}
          onClick={handlePrint}
          disabled={printDisabled}
          loading={printing}
          loadingText="Imprimiendo..."
        >
          Imprimir
        </AsyncButton>
      </DialogActions>
    </Dialog>
  );
}

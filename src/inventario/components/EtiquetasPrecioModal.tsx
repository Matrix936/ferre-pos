import { useMemo, useState } from 'react';
import {
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
import { useConfig } from '../../config/context/ConfigContext';
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

  const productoPreview = productos[0] ?? null;
  const negocio = systemName?.trim() || 'Ferre-Materiales La Mixteca';
  const totalEtiquetas = Math.max(1, Math.min(500, Math.floor(cantidad || 1))) * productos.length;

  const updateOption = (key: keyof LabelOptions) => {
    setOptions((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  const buildBarcodeHtml = (codigo: string) =>
    Array.from(codigo || 'SIN-CODIGO')
      .flatMap((char) =>
        Array.from({ length: 7 }, (_, bit) => {
          const black = ((char.charCodeAt(0) >> bit) & 1) === 1;
          const width = bit % 3 === 0 ? 2 : 1;
          return `<span style="display:inline-block;width:${width}px;height:38px;background:${black ? '#111' : '#fff'}"></span>`;
        }),
      )
      .join('');

  const handlePrint = () => {
    if (productos.length === 0) return;
    const copies = Math.max(1, Math.min(500, Math.floor(cantidad || 1)));
    const printWindow = window.open('', '_blank', 'width=640,height=720');
    if (!printWindow) return;

    const labelHtml = productos
      .flatMap((producto) => {
        const codigo = producto.codigoBarras || producto.codigoProveedor || producto.claveProducto || producto.id;
        const barcode = options.mostrarCodigoBarras ? `<div class="barcode">${buildBarcodeHtml(codigo)}</div>` : '';
        const key = producto.claveProducto || producto.codigoProveedor || codigo;
        return Array.from({ length: copies }, () => `
          <section class="label">
            ${options.mostrarNegocio ? `<div class="business">${negocio}</div>` : ''}
            ${options.mostrarDescripcion ? `<div class="description">${producto.descripcion}</div>` : ''}
            <div class="bottom">
              ${(options.mostrarCodigoBarras || options.mostrarClave) ? `<div class="code">${barcode}${options.mostrarClave ? `<div class="key">${key}</div>` : ''}</div>` : ''}
              ${options.mostrarPrecio ? `<div class="price">${formatMoney(producto.precioVenta)}</div>` : ''}
            </div>
          </section>
        `);
      })
      .join('');

    printWindow.document.write(`
      <html>
        <head>
          <title>Etiquetas de inventario</title>
          <style>
            @page { size: auto; margin: 6mm; }
            body { margin: 0; font-family: ${labelFont}; background: #fff; }
            .sheet { display: flex; flex-wrap: wrap; gap: 6px; }
            .label { box-sizing: border-box; width: 290px; height: 160px; padding: 10px; border: 1px solid #ddd; display: flex; flex-direction: column; gap: 4px; page-break-inside: avoid; color: #111; }
            .business { font-size: 12px; font-weight: 800; text-align: center; line-height: 1; }
            .description { font-size: 14px; font-weight: 800; text-align: center; line-height: 1.05; min-height: 30px; overflow: hidden; }
            .bottom { flex: 1; display: flex; align-items: center; justify-content: center; gap: 8px; }
            .code { flex: 1.35; min-width: 0; text-align: center; }
            .barcode { height: 42px; white-space: nowrap; overflow: hidden; }
            .key { font-family: monospace; font-size: 10px; line-height: 1; }
            .price { flex: 1; text-align: right; font-size: 30px; font-weight: 900; line-height: 1; }
          </style>
        </head>
        <body><main class="sheet">${labelHtml}</main></body>
      </html>
    `);
    printWindow.document.close();
    printWindow.focus();
    printWindow.print();
  };

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle sx={{ fontWeight: 700 }}>Etiquetas de precio</DialogTitle>
      <Divider />
      <DialogContent sx={{ pt: 3 }}>
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
      <DialogActions sx={{ p: 3, pt: 1 }}>
        <Button onClick={onClose}>Cancelar</Button>
        <Button variant="contained" startIcon={<PrintIcon />} onClick={handlePrint} disabled={productos.length === 0}>
          Imprimir
        </Button>
      </DialogActions>
    </Dialog>
  );
}

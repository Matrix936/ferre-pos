import { Box, Divider, Paper, Typography } from '@mui/material';
import logoDefecto from '../../images/logoDefecto.png';

interface TicketVenta {
  id: string;
  fecha: string;
  total: number;
  metodoPago: string;
  efectivoRecibido?: number | null;
  cambioEntregado?: number | null;
  estado: string;
  sucursalNombre: string;
  usuarioNombre: string;
}

interface TicketDetalle {
  id: string;
  descripcion: string;
  marca: string;
  cantidad: number;
  precioVentaPactado: number;
}

interface TicketPreviewProps {
  venta: TicketVenta;
  detalles: TicketDetalle[];
  empresaNombre?: string;
  rfc?: string;
  regimenFiscal?: string;
  codigoPostal?: string;
  logoSrc?: string;
}

const money = (value: number) => `$${(Number.isFinite(value) ? value : 0).toFixed(2)}`;
const ticketFont = "'Courier New', Courier, monospace";
const CREDIT_NOTICE =
  'AVISO DE CRÉDITO: Este comprobante expira a los 30 días naturales de su expedición. Pasada la fecha límite de pago, se aplicará una penalización de interés moratorio mensual sobre el saldo pendiente. ';

function DashedDivider() {
  return <Divider sx={{ my: 1.25, borderStyle: 'dashed', borderColor: 'grey.500' }} />;
}

function TicketRow({ left, right, bold = false }: { left: string; right: string; bold?: boolean }) {
  return (
    <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 1.5 }}>
      <Typography component="span" sx={{ fontFamily: ticketFont, fontSize: '13px', fontWeight: bold ? 700 : 400, minWidth: 0 }}>
        {left}
      </Typography>
      <Typography
        component="span"
        sx={{ fontFamily: ticketFont, fontSize: '13px', fontWeight: bold ? 700 : 400, textAlign: 'right', ml: 'auto', flexShrink: 0 }}
      >
        {right}
      </Typography>
    </Box>
  );
}

export function TicketPreview({
  venta,
  detalles,
  empresaNombre = 'FERRETERIA',
  rfc = '',
  regimenFiscal = '',
  codigoPostal = '',
  logoSrc = logoDefecto,
}: TicketPreviewProps) {
  const total = Number((Number.isFinite(venta.total) ? venta.total : 0).toFixed(2));
  const subtotal = Number((total / 1.16).toFixed(2));
  const iva = Number((total - subtotal).toFixed(2));
  const descuento = 0;
  const isCredito = venta.metodoPago.toUpperCase() === 'CREDITO';
  const isEfectivo = venta.metodoPago.toUpperCase() === 'EFECTIVO';
  const efectivoRecibido = typeof venta.efectivoRecibido === 'number' ? venta.efectivoRecibido : total;
  const cambioEntregado = typeof venta.cambioEntregado === 'number' ? venta.cambioEntregado : Math.max(efectivoRecibido - total, 0);

  return (
    <Paper
      elevation={8}
      sx={{
        width: '100%',
        maxWidth: 380,
        mx: 'auto',
        p: 2.25,
        bgcolor: '#fbfbfb',
        color: '#1f2933',
        border: '1px solid',
        borderColor: 'grey.300',
        boxShadow: '0 16px 35px rgba(15, 23, 42, 0.16)',
        fontFamily: ticketFont,
      }}
    >
      <Box sx={{ display: 'flex', justifyContent: 'center', mb: 1 }}>
        <Box
          component="img"
          src={logoSrc}
          alt="Logo"
          sx={{ maxWidth: 92, maxHeight: 64, objectFit: 'contain' }}
        />
      </Box>

      <Box sx={{ textAlign: 'center' }}>
        <Typography sx={{ fontFamily: ticketFont, fontSize: '15px', fontWeight: 700 }}>
          {empresaNombre}
        </Typography>
        <Typography sx={{ fontFamily: ticketFont, fontSize: '12px' }}>RFC: {rfc || 'SIN RFC CONFIGURADO'}</Typography>
        <Typography sx={{ fontFamily: ticketFont, fontSize: '12px' }}>
          Régimen Fiscal: {regimenFiscal || 'N/D'}
        </Typography>
        <Typography sx={{ fontFamily: ticketFont, fontSize: '12px' }}>
          C.P.: {codigoPostal || 'N/D'}
        </Typography>
        <Typography sx={{ fontFamily: ticketFont, fontSize: '12px' }}>{venta.sucursalNombre}</Typography>
        <Typography sx={{ fontFamily: ticketFont, fontSize: '12px' }}>FOLIO: {venta.id.slice(0, 8).toUpperCase()}</Typography>
      </Box>

      <DashedDivider />

      <TicketRow left="Fecha" right={new Date(venta.fecha).toLocaleString()} />
      <TicketRow left="Cajero" right={venta.usuarioNombre} />
      <TicketRow left="Pago" right={venta.metodoPago} />
      <TicketRow left="Estado" right={venta.estado} />

      <DashedDivider />

      {detalles.map((item) => {
        const importe = item.cantidad * item.precioVentaPactado;
        return (
          <Box key={item.id} sx={{ mb: 1 }}>
            <Typography sx={{ fontFamily: ticketFont, fontSize: '13px', fontWeight: 700, lineHeight: 1.25 }}>
              {item.descripcion}
            </Typography>
            {item.marca && (
              <Typography sx={{ fontFamily: ticketFont, fontSize: '11px', color: '#667085' }}>
                {item.marca}
              </Typography>
            )}
            <TicketRow left={`${item.cantidad.toFixed(3)} x ${money(item.precioVentaPactado)}`} right={money(importe)} />
          </Box>
        );
      })}

      <DashedDivider />

      <TicketRow left="Subtotal" right={money(subtotal)} />
      <TicketRow left="Descuento" right={`-${money(descuento)}`} />
      <TicketRow left="IVA 16%" right={money(iva)} />
      <Box sx={{ mt: 1 }}>
        <TicketRow left="TOTAL" right={money(total)} bold />
      </Box>
      {isEfectivo && (
        <Box sx={{ mt: 1 }}>
          <TicketRow left="Efectivo recibido" right={money(efectivoRecibido)} />
          <TicketRow left="Cambio" right={money(cambioEntregado)} bold />
        </Box>
      )}

      <DashedDivider />

      {isCredito && (
        <>
          <Box sx={{ my: 2.5, p: 1, border: '1px dashed #000' }}>
            <Typography sx={{ fontSize: '11px', textAlign: 'justify', fontFamily: 'monospace', lineHeight: 1.35 }}>
              {CREDIT_NOTICE}
            </Typography>
          </Box>
          <Box sx={{ mt: 4, mb: 2, textAlign: 'center' }}>
            <Typography sx={{ fontFamily: ticketFont, fontSize: '13px', whiteSpace: 'pre-line', lineHeight: 1.25 }}>
              {'__________________________________\n       Firma de Conformidad'}
            </Typography>
          </Box>
        </>
      )}

      <Typography sx={{ fontFamily: ticketFont, fontSize: '11px', textAlign: 'center', mb: 1 }}>
        Este ticket forma parte de la venta global del día.
      </Typography>

      <Typography sx={{ fontFamily: ticketFont, fontSize: '13px', textAlign: 'center', fontWeight: 700 }}>
        ¡Gracias por su compra!
      </Typography>
    </Paper>
  );
}

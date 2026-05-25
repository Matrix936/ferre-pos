import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Paper,
  Snackbar,
  TextField,
  Typography,
} from '@mui/material';
import { Add as AddIcon, Remove as RemoveIcon, PointOfSale as CorteIcon, Save as SaveIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';

interface CajaSesion {
  id: string;
  usuarioId: string;
  sucursalId: string;
  fechaApertura: string;
  montoInicial: number;
  fechaCierre: string | null;
  montoFinalReal: number | null;
  montoEsperado: number;
  estado: 'ABIERTA' | 'CERRADA';
}

interface CajaEstado {
  sesion: CajaSesion;
  ventasEfectivo: number;
  ingresos: number;
  egresos: number;
  montoEsperadoActual: number;
}

const MONEY_PATTERN = /^\d+(\.\d{0,2})?$/;

const parseMoneyInput = (value: string, fieldName: string, allowZero = true) => {
  const normalized = value.trim();
  if (!normalized || !MONEY_PATTERN.test(normalized)) {
    throw new Error(`${fieldName} debe ser un importe válido con máximo 2 decimales.`);
  }

  const amount = Math.round(Number(normalized) * 100) / 100;
  if (!Number.isFinite(amount) || amount < 0 || (!allowZero && amount <= 0)) {
    throw new Error(allowZero ? `${fieldName} no puede ser negativo.` : `${fieldName} debe ser mayor que cero.`);
  }
  return amount;
};

const toCents = (value: number) => Math.round(value * 100);

export function CajaView() {
  const { user } = useAuth();
  const [cajaActual, setCajaActual] = useState<CajaEstado | null>(null);
  const [montoInicial, setMontoInicial] = useState('0');
  const [loading, setLoading] = useState(false);
  const [snackbar, setSnackbar] = useState('');

  const [openMovimiento, setOpenMovimiento] = useState(false);
  const [tipoMovimiento, setTipoMovimiento] = useState<'INGRESO' | 'EGRESO'>('INGRESO');
  const [montoMovimiento, setMontoMovimiento] = useState('');
  const [motivoMovimiento, setMotivoMovimiento] = useState('');

  const [openCorte, setOpenCorte] = useState(false);
  const [montoFinalReal, setMontoFinalReal] = useState('');

  const diferencia = useMemo(() => {
    if (!cajaActual) return 0;
    return Number(montoFinalReal || 0) - cajaActual.montoEsperadoActual;
  }, [montoFinalReal, cajaActual]);

  const montoInicialValido = MONEY_PATTERN.test(montoInicial.trim() || '0');
  const montoMovimientoValido = MONEY_PATTERN.test(montoMovimiento.trim()) && Number(montoMovimiento) > 0;
  const montoFinalRealValido = MONEY_PATTERN.test(montoFinalReal.trim() || '0');
  const egresoSuperaCaja =
    Boolean(cajaActual) &&
    tipoMovimiento === 'EGRESO' &&
    montoMovimientoValido &&
    toCents(Number(montoMovimiento)) > toCents(cajaActual!.montoEsperadoActual);
  const cierreEnCeroInvalido =
    Boolean(cajaActual) &&
    cajaActual!.montoEsperadoActual > 0 &&
    montoFinalRealValido &&
    Number(montoFinalReal || 0) <= 0;

  const fetchCajaActual = async () => {
    if (!user?.id || !user?.sucursalId) return;
    const data = await invoke<CajaEstado | null>('get_caja_actual', {
      usuarioId: user.id,
      sucursalId: user.sucursalId,
    });
    setCajaActual(data);
  };

  useEffect(() => {
    fetchCajaActual().catch((error) => console.error('Error al consultar caja actual:', error));
  }, [user?.id, user?.sucursalId]);

  const handleAbrirCaja = async () => {
    if (!user?.id || !user?.sucursalId) return;
    setLoading(true);
    try {
      const fondoInicial = parseMoneyInput(montoInicial || '0', 'El fondo inicial');
      const data = await invoke<CajaEstado>('abrir_caja', {
        apertura: {
          id: crypto.randomUUID(),
          usuarioId: user.id,
          sucursalId: user.sucursalId,
          fechaApertura: new Date().toISOString(),
          montoInicial: fondoInicial,
        },
      });
      setCajaActual(data);
      setSnackbar('Caja abierta correctamente.');
    } catch (error) {
      setSnackbar(`Error al abrir caja: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleGuardarMovimiento = async () => {
    if (!cajaActual) return;
    setLoading(true);
    try {
      const monto = parseMoneyInput(montoMovimiento, 'El monto del movimiento', false);
      if (
        tipoMovimiento === 'EGRESO' &&
        cajaActual &&
        toCents(monto) > toCents(cajaActual.montoEsperadoActual)
      ) {
        throw new Error(
          `No puedes registrar un egreso mayor al efectivo esperado en caja ($${cajaActual.montoEsperadoActual.toFixed(2)}).`
        );
      }
      const data = await invoke<CajaEstado>('registrar_movimiento_caja', {
        movimiento: {
          id: crypto.randomUUID(),
          sesionId: cajaActual.sesion.id,
          tipo: tipoMovimiento,
          monto,
          motivo: motivoMovimiento.trim(),
        },
      });
      setCajaActual(data);
      setOpenMovimiento(false);
      setMontoMovimiento('');
      setMotivoMovimiento('');
      setSnackbar('Movimiento registrado correctamente.');
    } catch (error) {
      setSnackbar(`Error al registrar movimiento: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  const handleCerrarCaja = async () => {
    if (!cajaActual) return;
    setLoading(true);
    try {
      const montoContado = parseMoneyInput(montoFinalReal || '0', 'El monto contado físicamente');
      if (cajaActual.montoEsperadoActual > 0 && montoContado <= 0) {
        throw new Error('No puedes cerrar la caja en $0.00 cuando el monto esperado es mayor a cero.');
      }
      const diferenciaCierre = Math.round((montoContado - cajaActual.montoEsperadoActual) * 100) / 100;
      if (
        diferenciaCierre !== 0 &&
        !window.confirm(
          `La caja cerrará con una diferencia de ${diferenciaCierre >= 0 ? '+' : ''}$${diferenciaCierre.toFixed(2)}. ¿Confirmas el corte?`
        )
      ) {
        return;
      }

      await invoke('cerrar_caja', {
        cierre: {
          sesionId: cajaActual.sesion.id,
          fechaCierre: new Date().toISOString(),
          montoFinalReal: montoContado,
        },
      });
      setOpenCorte(false);
      setMontoFinalReal('');
      setCajaActual(null);
      setSnackbar('Caja cerrada correctamente.');
    } catch (error) {
      setSnackbar(`Error al cerrar caja: ${error}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <Box sx={{ maxWidth: 920, mx: 'auto', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3 }}>
        Caja
      </Typography>

      {!cajaActual ? (
        <Paper elevation={0} sx={{ p: 3, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
          <Typography variant="h6" sx={{ mb: 2, fontWeight: 600 }}>
            Apertura de caja
          </Typography>
          <TextField
            label="Fondo inicial"
            type="number"
            value={montoInicial}
            onChange={(e) => setMontoInicial(e.target.value)}
            fullWidth
            error={!montoInicialValido}
            helperText={!montoInicialValido ? 'Usa un importe válido con máximo 2 decimales.' : ' '}
            sx={{ mb: 2 }}
            slotProps={{ htmlInput: { min: 0, step: '0.01', inputMode: 'decimal' } }}
          />
          <Button
            variant="contained"
            onClick={handleAbrirCaja}
            disabled={loading || !montoInicialValido}
            startIcon={loading ? <CircularProgress size={18} color="inherit" /> : undefined}
          >
            {loading ? 'Abriendo...' : 'Abrir caja'}
          </Button>
        </Paper>
      ) : (
        <>
          <Paper elevation={0} sx={{ p: 3, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
            <Typography variant="h6" sx={{ mb: 2, fontWeight: 600 }}>
              Caja abierta
            </Typography>
            <Typography>Monto inicial: ${cajaActual.sesion.montoInicial.toFixed(2)}</Typography>
            <Typography>Ventas en efectivo: ${cajaActual.ventasEfectivo.toFixed(2)}</Typography>
            <Typography>Ingresos: ${cajaActual.ingresos.toFixed(2)}</Typography>
            <Typography>Egresos: ${cajaActual.egresos.toFixed(2)}</Typography>
            <Typography sx={{ mt: 1.5, fontWeight: 700, color: 'primary.main' }}>
              Monto esperado: ${cajaActual.montoEsperadoActual.toFixed(2)}
            </Typography>
          </Paper>

          <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
            <Button
              variant="outlined"
              startIcon={<AddIcon />}
              onClick={() => {
                setTipoMovimiento('INGRESO');
                setOpenMovimiento(true);
              }}
            >
              Registrar ingreso
            </Button>
            <Button
              variant="outlined"
              color="warning"
              startIcon={<RemoveIcon />}
              onClick={() => {
                setTipoMovimiento('EGRESO');
                setOpenMovimiento(true);
              }}
            >
              Registrar egreso
            </Button>
            <Button variant="contained" color="error" size="large" startIcon={<CorteIcon />} onClick={() => setOpenCorte(true)}>
              Corte de caja
            </Button>
          </Box>
        </>
      )}

      <Dialog open={openMovimiento} onClose={loading ? undefined : () => setOpenMovimiento(false)} maxWidth="xs" fullWidth>
        <DialogTitle>{tipoMovimiento === 'INGRESO' ? 'Registrar ingreso' : 'Registrar egreso'}</DialogTitle>
        <DialogContent sx={{ pt: 2, display: 'flex', flexDirection: 'column', gap: 2 }}>
          <TextField
            label="Monto"
            type="number"
            value={montoMovimiento}
            onChange={(e) => setMontoMovimiento(e.target.value)}
            error={(Boolean(montoMovimiento) && !montoMovimientoValido) || egresoSuperaCaja}
            helperText={
              egresoSuperaCaja
                ? `El egreso no puede superar el efectivo esperado ($${cajaActual?.montoEsperadoActual.toFixed(2) || '0.00'}).`
                : Boolean(montoMovimiento) && !montoMovimientoValido
                  ? 'Debe ser mayor a 0 y tener máximo 2 decimales.'
                  : ' '
            }
            slotProps={{ htmlInput: { min: 0.01, step: '0.01', inputMode: 'decimal' } }}
          />
          <TextField label="Motivo" value={motivoMovimiento} onChange={(e) => setMotivoMovimiento(e.target.value)} />
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <Button onClick={() => setOpenMovimiento(false)} disabled={loading}>Cancelar</Button>
          <Button
            variant="contained"
            onClick={handleGuardarMovimiento}
            disabled={loading || !motivoMovimiento.trim() || !montoMovimientoValido || egresoSuperaCaja}
            startIcon={loading ? <CircularProgress size={18} color="inherit" /> : <SaveIcon />}
          >
            {loading ? 'Guardando...' : 'Guardar'}
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={openCorte} onClose={loading ? undefined : () => setOpenCorte(false)} maxWidth="xs" fullWidth>
        <DialogTitle sx={{ pb: 1 }}>Corte de caja</DialogTitle>
        <DialogContent sx={{ '&&': { pt: 2.5 }, display: 'flex', flexDirection: 'column', gap: 2 }}>
          <TextField label="Monto esperado" value={`$${cajaActual?.montoEsperadoActual.toFixed(2) || '0.00'}`} disabled />
          <TextField
            label="Monto contado físicamente"
            type="number"
            value={montoFinalReal}
            onChange={(e) => setMontoFinalReal(e.target.value)}
            error={(Boolean(montoFinalReal) && !montoFinalRealValido) || cierreEnCeroInvalido}
            helperText={
              cierreEnCeroInvalido
                ? 'No puedes cerrar en $0.00 si el monto esperado es mayor a cero.'
                : Boolean(montoFinalReal) && !montoFinalRealValido
                  ? 'Usa un importe válido con máximo 2 decimales.'
                  : ' '
            }
            slotProps={{ htmlInput: { min: 0, step: '0.01', inputMode: 'decimal' } }}
          />
          <TextField
            label="Diferencia"
            value={`${diferencia >= 0 ? '+' : ''}$${diferencia.toFixed(2)}`}
            disabled
          />
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <Button onClick={() => setOpenCorte(false)} disabled={loading}>Cancelar</Button>
          <Button
            variant="contained"
            color="error"
            onClick={handleCerrarCaja}
            disabled={loading || !montoFinalRealValido || cierreEnCeroInvalido}
            startIcon={loading ? <CircularProgress size={18} color="inherit" /> : <CorteIcon />}
          >
            {loading ? 'Cerrando...' : 'Cerrar turno'}
          </Button>
        </DialogActions>
      </Dialog>

      <Snackbar open={Boolean(snackbar)} autoHideDuration={3500} onClose={() => setSnackbar('')}>
        <Alert onClose={() => setSnackbar('')} severity={snackbar.startsWith('Error') ? 'error' : 'success'} variant="filled">
          {snackbar}
        </Alert>
      </Snackbar>
    </Box>
  );
}

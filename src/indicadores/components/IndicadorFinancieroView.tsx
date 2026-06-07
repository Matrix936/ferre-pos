import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Alert, Box, Button } from '@mui/material';
import DownloadIcon from '@mui/icons-material/Download';
import { IndicadorFiltro, IndicadorFiltros, IndicadorLoadingBanner, IndicadorPage, MetricCard, MetricGrid, comparisonText, emptyIndicadorFiltro, money, previousPeriodFiltro, toBackendFiltro } from './IndicadoresCommon';
import { downloadCsv } from '../../shared/utils/csv';

interface IndicadorFinancieroResumen {
  ingresosCaja: number;
  egresosCaja: number;
  ventasEfectivo: number;
  ventasTarjeta: number;
  ventasTransferencia: number;
  ventasCredito: number;
  compras: number;
  cuentasPorCobrar: number;
  flujoNetoEstimado: number;
}

export function IndicadorFinancieroView() {
  const [filtro, setFiltro] = useState<IndicadorFiltro>(() => emptyIndicadorFiltro());
  const [data, setData] = useState<IndicadorFinancieroResumen | null>(null);
  const [previousData, setPreviousData] = useState<IndicadorFinancieroResumen | null>(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let active = true;
    const previousFiltro = previousPeriodFiltro(filtro);
    setLoading(true);
    Promise.all([
      invoke<IndicadorFinancieroResumen>('get_indicador_financiero', { filtro: toBackendFiltro(filtro) }),
      previousFiltro
        ? invoke<IndicadorFinancieroResumen>('get_indicador_financiero', { filtro: toBackendFiltro(previousFiltro) })
        : Promise.resolve(null),
    ])
      .then(([result, previous]) => {
        if (!active) return;
        setData(result);
        setPreviousData(previous);
        setError('');
      })
      .catch((err) => {
        if (active) setError(String(err));
      })
      .finally(() => {
        if (active) setLoading(false);
      });
    return () => {
      active = false;
    };
  }, [filtro]);

  return (
    <IndicadorPage title="Indicador financiero" subtitle="Flujo por caja, métodos de pago, compras y cuentas por cobrar.">
      <IndicadorFiltros filtro={filtro} onChange={setFiltro} showPaymentFilter showUserFilter />
      <IndicadorLoadingBanner visible={loading} />
      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}
      <MetricGrid>
        <MetricCard label="Flujo neto estimado" value={money(data?.flujoNetoEstimado ?? 0)} helper={comparisonText(data?.flujoNetoEstimado ?? 0, previousData?.flujoNetoEstimado)} />
        <MetricCard label="Ventas efectivo" value={money(data?.ventasEfectivo ?? 0)} helper={comparisonText(data?.ventasEfectivo ?? 0, previousData?.ventasEfectivo)} />
        <MetricCard label="Ventas tarjeta" value={money(data?.ventasTarjeta ?? 0)} helper={comparisonText(data?.ventasTarjeta ?? 0, previousData?.ventasTarjeta)} />
        <MetricCard label="Transferencias" value={money(data?.ventasTransferencia ?? 0)} helper={comparisonText(data?.ventasTransferencia ?? 0, previousData?.ventasTransferencia)} />
        <MetricCard label="Ventas a crédito" value={money(data?.ventasCredito ?? 0)} helper={comparisonText(data?.ventasCredito ?? 0, previousData?.ventasCredito)} />
        <MetricCard label="Ingresos caja" value={money(data?.ingresosCaja ?? 0)} helper={comparisonText(data?.ingresosCaja ?? 0, previousData?.ingresosCaja)} />
        <MetricCard label="Egresos caja" value={money(data?.egresosCaja ?? 0)} helper={comparisonText(data?.egresosCaja ?? 0, previousData?.egresosCaja)} />
        <MetricCard label="Compras" value={money(data?.compras ?? 0)} helper={comparisonText(data?.compras ?? 0, previousData?.compras)} />
        <MetricCard label="Cuentas por cobrar" value={money(data?.cuentasPorCobrar ?? 0)} />
      </MetricGrid>
      <Box sx={{ display: 'flex', justifyContent: 'flex-end' }}>
        <Button
          size="small"
          variant="outlined"
          startIcon={<DownloadIcon />}
          disabled={loading || !data}
          onClick={() => data && downloadCsv('indicador-financiero.csv', [
            { concepto: 'Flujo neto estimado', importe: data.flujoNetoEstimado },
            { concepto: 'Ventas efectivo', importe: data.ventasEfectivo },
            { concepto: 'Ventas tarjeta', importe: data.ventasTarjeta },
            { concepto: 'Transferencias', importe: data.ventasTransferencia },
            { concepto: 'Ventas a credito', importe: data.ventasCredito },
            { concepto: 'Ingresos caja', importe: data.ingresosCaja },
            { concepto: 'Egresos caja', importe: data.egresosCaja },
            { concepto: 'Compras', importe: data.compras },
            { concepto: 'Cuentas por cobrar', importe: data.cuentasPorCobrar },
          ])}
        >
          Exportar CSV
        </Button>
      </Box>
    </IndicadorPage>
  );
}

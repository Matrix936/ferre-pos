import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Alert, Box, Button, Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow, Typography } from '@mui/material';
import DownloadIcon from '@mui/icons-material/Download';
import { IndicadorFiltro, IndicadorFiltros, IndicadorLoadingBanner, IndicadorPage, MetricCard, MetricGrid, comparisonText, emptyIndicadorFiltro, money, numberText, previousPeriodFiltro, toBackendFiltro } from './IndicadoresCommon';
import { TablePager } from '../../shared/components/TablePager';
import { useLocalPagination } from '../../shared/hooks/useLocalPagination';
import { downloadCsv } from '../../shared/utils/csv';

interface MetodoPagoResumen {
  metodoPago: string;
  total: number;
  transacciones: number;
}

interface ProductoMasVendido {
  productoId: string;
  descripcion: string;
  marca: string;
  unidadesVendidas: number;
}

interface IndicadorVentasResumen {
  totalVendido: number;
  transacciones: number;
  ticketPromedio: number;
  ventasCanceladas: number;
  ventasCredito: number;
  ventasContado: number;
  metodos: MetodoPagoResumen[];
  productosMasVendidos: ProductoMasVendido[];
}

export function IndicadorVentasView() {
  const [filtro, setFiltro] = useState<IndicadorFiltro>(() => emptyIndicadorFiltro());
  const [data, setData] = useState<IndicadorVentasResumen | null>(null);
  const [previousData, setPreviousData] = useState<IndicadorVentasResumen | null>(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const productosPager = useLocalPagination(data?.productosMasVendidos ?? []);

  useEffect(() => {
    let active = true;
    const previousFiltro = previousPeriodFiltro(filtro);
    setLoading(true);
    Promise.all([
      invoke<IndicadorVentasResumen>('get_indicador_ventas', { filtro: toBackendFiltro(filtro) }),
      previousFiltro
        ? invoke<IndicadorVentasResumen>('get_indicador_ventas', { filtro: toBackendFiltro(previousFiltro) })
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
    <IndicadorPage title="Indicador de ventas" subtitle="Comportamiento de ventas por periodo, método de pago y productos.">
      <IndicadorFiltros filtro={filtro} onChange={setFiltro} showCatalogFilters showPaymentFilter showUserFilter />
      <IndicadorLoadingBanner visible={loading} />
      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}
      <MetricGrid>
        <MetricCard label="Total vendido" value={money(data?.totalVendido ?? 0)} helper={comparisonText(data?.totalVendido ?? 0, previousData?.totalVendido)} />
        <MetricCard label="Transacciones" value={numberText(data?.transacciones ?? 0)} helper={comparisonText(data?.transacciones ?? 0, previousData?.transacciones)} />
        <MetricCard label="Ticket promedio" value={money(data?.ticketPromedio ?? 0)} helper={comparisonText(data?.ticketPromedio ?? 0, previousData?.ticketPromedio)} />
        <MetricCard label="Canceladas" value={numberText(data?.ventasCanceladas ?? 0)} helper={comparisonText(data?.ventasCanceladas ?? 0, previousData?.ventasCanceladas)} />
        <MetricCard label="Contado" value={money(data?.ventasContado ?? 0)} helper={comparisonText(data?.ventasContado ?? 0, previousData?.ventasContado)} />
        <MetricCard label="Crédito" value={money(data?.ventasCredito ?? 0)} helper={comparisonText(data?.ventasCredito ?? 0, previousData?.ventasCredito)} />
      </MetricGrid>
      <Box sx={{ display: 'flex', justifyContent: 'flex-end', mb: 1.5 }}>
        <Button
          size="small"
          variant="outlined"
          startIcon={<DownloadIcon />}
          disabled={loading || (!data?.productosMasVendidos.length && !data?.metodos.length)}
          onClick={() => downloadCsv('indicador-ventas.csv', [
            ...(data?.metodos ?? []).map((item) => ({
              seccion: 'Metodo de pago',
              concepto: item.metodoPago,
              total: item.total,
              tickets: item.transacciones,
              unidades: '',
            })),
            ...(data?.productosMasVendidos ?? []).map((item) => ({
              seccion: 'Producto vendido',
              concepto: item.descripcion,
              total: '',
              tickets: '',
              unidades: item.unidadesVendidas,
            })),
          ])}
        >
          Exportar CSV
        </Button>
      </Box>
      <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', lg: '0.8fr 1.2fr' }, gap: 2 }}>
        <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
          <Typography variant="subtitle2" sx={{ fontWeight: 800, mb: 1.5 }}>Métodos de pago</Typography>
          <Table size="small">
            <TableHead>
              <TableRow>
                <TableCell>Método</TableCell>
                <TableCell>Total</TableCell>
                <TableCell>Tickets</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {(data?.metodos ?? []).map((item) => (
                <TableRow key={item.metodoPago}>
                  <TableCell>{item.metodoPago}</TableCell>
                  <TableCell>{money(item.total)}</TableCell>
                  <TableCell>{item.transacciones}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </Paper>
        <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
          <TableContainer>
            <Table>
              <TableHead sx={{ bgcolor: 'background.default' }}>
                <TableRow>
                  <TableCell sx={{ fontWeight: 700 }}>Producto</TableCell>
                  <TableCell sx={{ fontWeight: 700 }}>Marca</TableCell>
                  <TableCell sx={{ fontWeight: 700 }}>Unidades</TableCell>
                </TableRow>
              </TableHead>
              <TableBody>
                {productosPager.paginatedRows.map((item) => (
                  <TableRow key={item.productoId} hover>
                    <TableCell>{item.descripcion}</TableCell>
                    <TableCell>{item.marca || '-'}</TableCell>
                    <TableCell>{item.unidadesVendidas}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </TableContainer>
          <TablePager {...pagerProps(productosPager, 'productos')} />
        </Paper>
      </Box>
    </IndicadorPage>
  );
}

function pagerProps(pager: ReturnType<typeof useLocalPagination<ProductoMasVendido>>, rowLabel: string) {
  return { page: pager.page, pageSize: pager.pageSize, totalPages: pager.totalPages, totalRows: pager.totalRows, fromRow: pager.fromRow, toRow: pager.toRow, canPreviousPage: pager.canPreviousPage, canNextPage: pager.canNextPage, onPreviousPage: pager.previousPage, onNextPage: pager.nextPage, onPageSizeChange: pager.setPageSize, rowLabel };
}

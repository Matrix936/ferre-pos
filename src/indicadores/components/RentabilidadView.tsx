import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Alert, Box, Button, Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow } from '@mui/material';
import DownloadIcon from '@mui/icons-material/Download';
import { IndicadorFiltro, IndicadorFiltros, IndicadorLoadingBanner, IndicadorPage, MetricCard, MetricGrid, comparisonText, emptyIndicadorFiltro, money, numberText, percent, previousPeriodFiltro, toBackendFiltro } from './IndicadoresCommon';
import { TablePager } from '../../shared/components/TablePager';
import { useLocalPagination } from '../../shared/hooks/useLocalPagination';
import { downloadCsv } from '../../shared/utils/csv';

interface ProductoRentabilidad {
  productoId: string;
  descripcion: string;
  marca: string;
  unidades: number;
  ventaTotal: number;
  costoTotal: number;
  utilidad: number;
  margenPorcentaje: number;
}

interface RentabilidadResumen {
  ventaTotal: number;
  costoTotal: number;
  utilidadBruta: number;
  margenPorcentaje: number;
  productos: ProductoRentabilidad[];
}

export function RentabilidadView() {
  const [filtro, setFiltro] = useState<IndicadorFiltro>(() => emptyIndicadorFiltro());
  const [data, setData] = useState<RentabilidadResumen | null>(null);
  const [previousData, setPreviousData] = useState<RentabilidadResumen | null>(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const pager = useLocalPagination(data?.productos ?? []);

  useEffect(() => {
    let active = true;
    const previousFiltro = previousPeriodFiltro(filtro);
    setLoading(true);
    Promise.all([
      invoke<RentabilidadResumen>('get_rentabilidad', { filtro: toBackendFiltro(filtro) }),
      previousFiltro
        ? invoke<RentabilidadResumen>('get_rentabilidad', { filtro: toBackendFiltro(previousFiltro) })
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
    <IndicadorPage title="Rentabilidad" subtitle="Utilidad bruta, margen y productos que más aportan al negocio.">
      <IndicadorFiltros filtro={filtro} onChange={setFiltro} showCatalogFilters showPaymentFilter showUserFilter />
      <IndicadorLoadingBanner visible={loading} />
      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}
      <MetricGrid>
        <MetricCard label="Venta neta" value={money(data?.ventaTotal ?? 0)} helper={comparisonText(data?.ventaTotal ?? 0, previousData?.ventaTotal)} />
        <MetricCard label="Costo vendido" value={money(data?.costoTotal ?? 0)} helper={comparisonText(data?.costoTotal ?? 0, previousData?.costoTotal)} />
        <MetricCard label="Utilidad bruta" value={money(data?.utilidadBruta ?? 0)} helper={comparisonText(data?.utilidadBruta ?? 0, previousData?.utilidadBruta)} />
        <MetricCard label="Margen" value={percent(data?.margenPorcentaje ?? 0)} helper={comparisonText(data?.margenPorcentaje ?? 0, previousData?.margenPorcentaje)} />
        <MetricCard label="Productos evaluados" value={numberText(data?.productos.length ?? 0)} />
      </MetricGrid>
      <Box sx={{ display: 'flex', justifyContent: 'flex-end', mb: 1.5 }}>
        <Button
          size="small"
          variant="outlined"
          startIcon={<DownloadIcon />}
          disabled={loading || !data?.productos.length}
          onClick={() => downloadCsv('rentabilidad-productos.csv', (data?.productos ?? []).map((item) => ({
            producto: item.descripcion,
            marca: item.marca,
            unidades: item.unidades,
            venta: item.ventaTotal,
            costo: item.costoTotal,
            utilidad: item.utilidad,
            margen: item.margenPorcentaje,
          })))}
        >
          Exportar CSV
        </Button>
      </Box>
      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 700 }}>Producto</TableCell>
                <TableCell sx={{ fontWeight: 700 }}>Marca</TableCell>
                <TableCell sx={{ fontWeight: 700 }}>Unidades</TableCell>
                <TableCell sx={{ fontWeight: 700 }}>Venta</TableCell>
                <TableCell sx={{ fontWeight: 700 }}>Costo</TableCell>
                <TableCell sx={{ fontWeight: 700 }}>Utilidad</TableCell>
                <TableCell sx={{ fontWeight: 700 }}>Margen</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {pager.paginatedRows.map((item) => (
                <TableRow key={item.productoId} hover>
                  <TableCell>{item.descripcion}</TableCell>
                  <TableCell>{item.marca || '-'}</TableCell>
                  <TableCell>{item.unidades}</TableCell>
                  <TableCell>{money(item.ventaTotal)}</TableCell>
                  <TableCell>{money(item.costoTotal)}</TableCell>
                  <TableCell>{money(item.utilidad)}</TableCell>
                  <TableCell>{percent(item.margenPorcentaje)}</TableCell>
                </TableRow>
              ))}
              {pager.totalRows === 0 && (
                <TableRow><TableCell colSpan={7} sx={{ py: 4, color: 'text.secondary' }}>Sin ventas para calcular rentabilidad.</TableCell></TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
        <TablePager {...pagerProps(pager, 'productos')} />
      </Paper>
    </IndicadorPage>
  );
}

function pagerProps(pager: ReturnType<typeof useLocalPagination<ProductoRentabilidad>>, rowLabel: string) {
  return { page: pager.page, pageSize: pager.pageSize, totalPages: pager.totalPages, totalRows: pager.totalRows, fromRow: pager.fromRow, toRow: pager.toRow, canPreviousPage: pager.canPreviousPage, canNextPage: pager.canNextPage, onPreviousPage: pager.previousPage, onNextPage: pager.nextPage, onPageSizeChange: pager.setPageSize, rowLabel };
}

import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Alert, Box, Button, Paper, Table, TableBody, TableCell, TableContainer, TableHead, TableRow } from '@mui/material';
import DownloadIcon from '@mui/icons-material/Download';
import { useAuth } from '../../auth/context/AuthContext';
import { IndicadorFiltro, IndicadorFiltros, IndicadorLoadingBanner, IndicadorPage, MetricCard, MetricGrid, emptyIndicadorFiltro, money, numberText, toBackendFiltro } from './IndicadoresCommon';
import { TablePager } from '../../shared/components/TablePager';
import { useLocalPagination } from '../../shared/hooks/useLocalPagination';
import { downloadCsv } from '../../shared/utils/csv';

interface ProductoBajoStock {
  productoId: string;
  descripcion: string;
  marca: string;
  sucursalNombre: string;
  stock: number;
  stockMinimo: number;
}

interface IndicadorInventarioResumen {
  productosEnInventario: number;
  valorInventario: number;
  stockTotal: number;
  stockBajo: number;
  sinStock: number;
  sobreStock: number;
  bajoStock: ProductoBajoStock[];
}

export function IndicadorInventarioView() {
  const { user } = useAuth();
  const [filtro, setFiltro] = useState<IndicadorFiltro>(() => emptyIndicadorFiltro());
  const [data, setData] = useState<IndicadorInventarioResumen | null>(null);
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);
  const pager = useLocalPagination(data?.bajoStock ?? []);
  const backendFiltro = useMemo(
    () => user?.role === 'SUPERADMIN'
      ? toBackendFiltro(filtro)
      : { ...toBackendFiltro(filtro), sucursalId: user?.sucursalId },
    [filtro, user?.role, user?.sucursalId],
  );

  useEffect(() => {
    let active = true;
    setLoading(true);
    invoke<IndicadorInventarioResumen>('get_indicador_inventario', { filtro: backendFiltro })
      .then((result) => {
        if (!active) return;
        setData(result);
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
  }, [backendFiltro]);

  return (
    <IndicadorPage title="Indicador de inventario" subtitle="Valor del inventario, riesgo de agotados y mercancía con exceso.">
      <IndicadorFiltros filtro={filtro} onChange={setFiltro} showDates={false} showCatalogFilters />
      <IndicadorLoadingBanner visible={loading} />
      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}
      <MetricGrid>
        <MetricCard label="Valor inventario" value={money(data?.valorInventario ?? 0)} />
        <MetricCard label="Productos" value={numberText(data?.productosEnInventario ?? 0)} />
        <MetricCard label="Piezas totales" value={numberText(data?.stockTotal ?? 0)} />
        <MetricCard label="Stock bajo" value={numberText(data?.stockBajo ?? 0)} />
        <MetricCard label="Sin stock" value={numberText(data?.sinStock ?? 0)} />
        <MetricCard label="Sobreinventario" value={numberText(data?.sobreStock ?? 0)} />
      </MetricGrid>
      <Box sx={{ display: 'flex', justifyContent: 'flex-end', mb: 1.5 }}>
        <Button
          size="small"
          variant="outlined"
          startIcon={<DownloadIcon />}
          disabled={loading || !data?.bajoStock.length}
          onClick={() => downloadCsv('inventario-bajo-stock.csv', (data?.bajoStock ?? []).map((item) => ({
            producto: item.descripcion,
            marca: item.marca,
            sucursal: item.sucursalNombre,
            stock: item.stock,
            minimo: item.stockMinimo,
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
                <TableCell sx={{ fontWeight: 700 }}>Sucursal</TableCell>
                <TableCell sx={{ fontWeight: 700 }}>Stock</TableCell>
                <TableCell sx={{ fontWeight: 700 }}>Mínimo</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {pager.paginatedRows.map((item) => (
                <TableRow key={`${item.productoId}-${item.sucursalNombre}`} hover>
                  <TableCell>{item.descripcion}</TableCell>
                  <TableCell>{item.marca || '-'}</TableCell>
                  <TableCell>{item.sucursalNombre}</TableCell>
                  <TableCell>{item.stock}</TableCell>
                  <TableCell>{item.stockMinimo}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </TableContainer>
        <TablePager {...pagerProps(pager, 'alertas')} />
      </Paper>
    </IndicadorPage>
  );
}

function pagerProps(pager: ReturnType<typeof useLocalPagination<ProductoBajoStock>>, rowLabel: string) {
  return { page: pager.page, pageSize: pager.pageSize, totalPages: pager.totalPages, totalRows: pager.totalRows, fromRow: pager.fromRow, toRow: pager.toRow, canPreviousPage: pager.canPreviousPage, canNextPage: pager.canNextPage, onPreviousPage: pager.previousPage, onNextPage: pager.nextPage, onPageSizeChange: pager.setPageSize, rowLabel };
}

import { Box, IconButton, MenuItem, Select, Typography } from '@mui/material';
import { ChevronLeft, ChevronRight } from '@mui/icons-material';
import { LOCAL_TABLE_PAGE_SIZE_OPTIONS } from '../hooks/useLocalPagination';

interface TablePagerProps {
  page: number;
  pageSize: number;
  totalPages: number;
  totalRows: number;
  fromRow: number;
  toRow: number;
  canPreviousPage: boolean;
  canNextPage: boolean;
  onPreviousPage: () => void;
  onNextPage: () => void;
  onPageSizeChange: (pageSize: number) => void;
  rowLabel?: string;
  summary?: string;
}

export function TablePager({
  page,
  pageSize,
  totalPages,
  totalRows,
  fromRow,
  toRow,
  canPreviousPage,
  canNextPage,
  onPreviousPage,
  onNextPage,
  onPageSizeChange,
  rowLabel = 'registros',
  summary,
}: TablePagerProps) {
  return (
    <Box
      sx={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: 2,
        px: 2,
        py: 1.5,
        borderTop: '1px solid',
        borderColor: 'divider',
        flexWrap: 'wrap',
      }}
    >
      <Typography variant="body2" color="text.secondary" sx={{ fontWeight: 600 }}>
        {summary ?? (totalRows === 0 ? `Total: 0 ${rowLabel}` : `Mostrando ${fromRow}-${toRow} de ${totalRows} ${rowLabel}`)}
      </Typography>

      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5, ml: 'auto' }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <Typography variant="body2" color="text.secondary">
            Mostrar:
          </Typography>
          <Select
            size="small"
            value={pageSize}
            onChange={(event) => onPageSizeChange(Number(event.target.value))}
            sx={{ minWidth: 76, '& .MuiSelect-select': { py: 0.75 } }}
          >
            {LOCAL_TABLE_PAGE_SIZE_OPTIONS.map((option) => (
              <MenuItem key={option} value={option}>
                {option}
              </MenuItem>
            ))}
          </Select>
        </Box>
        <IconButton size="small" onClick={onPreviousPage} disabled={!canPreviousPage} aria-label="Página anterior">
          <ChevronLeft fontSize="small" />
        </IconButton>
        <Typography variant="body2" color="text.secondary" sx={{ minWidth: 118, textAlign: 'center', fontWeight: 600 }}>
          Página {page + 1} de {totalPages}
        </Typography>
        <IconButton size="small" onClick={onNextPage} disabled={!canNextPage} aria-label="Página siguiente">
          <ChevronRight fontSize="small" />
        </IconButton>
      </Box>
    </Box>
  );
}

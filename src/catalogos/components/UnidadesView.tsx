import { useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Box,
  Button,
  CircularProgress,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  IconButton,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from '@mui/material';
import { Add as AddIcon, Delete as DeleteIcon, Edit as EditIcon, Save as SaveIcon } from '@mui/icons-material';
import { useCatalogos } from '../context/CatalogosContext';
import { UnidadMedida } from '../../inventario/types';
import { AsyncButton } from '../../shared/components/AsyncButton';
import { ConfirmActionDialog } from '../../shared/components/ConfirmActionDialog';
import { FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { TablePager } from '../../shared/components/TablePager';
import { TableActions } from '../../shared/components/TableActions';
import { useDialogHotkeys } from '../../shared/hooks/useDialogHotkeys';
import { useFeedback } from '../../shared/hooks/useFeedback';
import { useLocalPagination } from '../../shared/hooks/useLocalPagination';
import { dialogActionsSx, dialogContentSx } from '../../shared/ui/patterns';

export function UnidadesView() {
  const { unidades, refreshCatalogos } = useCatalogos();
  const [search, setSearch] = useState('');
  const [open, setOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [currentId, setCurrentId] = useState('');
  const [nombre, setNombre] = useState('');
  const [claveSat, setClaveSat] = useState('');
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState('');
  const [deleteTarget, setDeleteTarget] = useState<UnidadMedida | null>(null);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return q
      ? unidades.filter((unidad) => unidad.nombre.toLowerCase().includes(q) || unidad.claveSat.toLowerCase().includes(q))
      : unidades;
  }, [unidades, search]);
  const unidadesPager = useLocalPagination(filtered);

  const handleOpen = (unidad?: UnidadMedida) => {
    setEditMode(Boolean(unidad));
    setCurrentId(unidad?.id ?? crypto.randomUUID());
    setNombre(unidad?.nombre ?? '');
    setClaveSat(unidad?.claveSat ?? '');
    setOpen(true);
  };

  const handleSave = async () => {
    if (saving) return;
    const unidad: UnidadMedida = { id: currentId, nombre: nombre.trim(), claveSat: claveSat.trim().toUpperCase() };
    setSaving(true);
    try {
      if (editMode) {
        await invoke('update_unidad', { id: currentId, unidad });
      } else {
        await invoke('create_unidad', { unidad });
      }
      setOpen(false);
      await refreshCatalogos();
      showFeedback(editMode ? 'Unidad actualizada.' : 'Unidad creada.', 'success');
    } catch (error) {
      showFeedback(`Error al guardar: ${error}`, 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    setDeletingId(id);
    try {
      await invoke('delete_unidad', { id });
      await refreshCatalogos();
      setDeleteTarget(null);
      showFeedback('Unidad eliminada.', 'success');
    } catch (error) {
      showFeedback(`Error al eliminar: ${error}`, 'error');
    } finally {
      setDeletingId('');
    }
  };

  const claveSatLimpia = claveSat.trim();
  const claveSatInvalida = claveSatLimpia.length > 0 && claveSatLimpia.length !== 3;
  const saveDisabled = saving || !nombre.trim() || claveSatInvalida;

  useDialogHotkeys({
    open,
    disabled: saveDisabled,
    cancelDisabled: saving,
    onConfirm: handleSave,
    onCancel: () => setOpen(false),
  });

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3, gap: 2, flexWrap: 'wrap' }}>
        <Typography variant="h5" sx={{ fontWeight: 700 }}>Unidades</Typography>
        <Button variant="contained" startIcon={<AddIcon />} onClick={() => handleOpen()} disableElevation>
          Nueva unidad
        </Button>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
          <TextField label="Buscar unidad o clave SAT" value={search} onChange={(event) => setSearch(event.target.value)} fullWidth />
          <TableActions
            filename="unidades"
            rows={filtered.map((unidad) => ({ nombre: unidad.nombre, claveSat: unidad.claveSat }))}
            columns={[
              { key: 'nombre', label: 'Nombre' },
              { key: 'claveSat', label: 'Clave SAT' },
            ]}
          />
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Nombre</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Clave SAT</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {unidadesPager.paginatedRows.map((unidad) => (
                <TableRow key={unidad.id} hover>
                  <TableCell>{unidad.nombre}</TableCell>
                  <TableCell>{unidad.claveSat || '-'}</TableCell>
                  <TableCell>
                    <IconButton color="primary" size="small" onClick={() => handleOpen(unidad)} sx={{ mr: 1 }} disabled={Boolean(deletingId)}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="error" size="small" onClick={() => setDeleteTarget(unidad)} disabled={Boolean(deletingId)}>
                      {deletingId === unidad.id ? <CircularProgress size={18} /> : <DeleteIcon fontSize="small" />}
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))}
              {filtered.length === 0 && (
                <TableRow>
                  <TableCell colSpan={3} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay unidades registradas.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
        <TablePager
          page={unidadesPager.page}
          pageSize={unidadesPager.pageSize}
          totalPages={unidadesPager.totalPages}
          totalRows={unidadesPager.totalRows}
          fromRow={unidadesPager.fromRow}
          toRow={unidadesPager.toRow}
          canPreviousPage={unidadesPager.canPreviousPage}
          canNextPage={unidadesPager.canNextPage}
          onPreviousPage={unidadesPager.previousPage}
          onNextPage={unidadesPager.nextPage}
          onPageSizeChange={unidadesPager.setPageSize}
          rowLabel="unidades"
        />
      </Paper>

      <Dialog open={open} onClose={saving ? undefined : () => setOpen(false)} maxWidth="xs" fullWidth>
        <DialogTitle sx={{ fontWeight: 600 }}>{editMode ? 'Editar unidad' : 'Nueva unidad'}</DialogTitle>
        <Divider />
        <DialogContent sx={dialogContentSx}>
          <TextField label="Nombre" value={nombre} onChange={(event) => setNombre(event.target.value)} fullWidth required />
          <TextField
            label="Clave Unidad SAT"
            value={claveSat}
            onChange={(event) => setClaveSat(event.target.value.toUpperCase())}
            fullWidth
            error={claveSatInvalida}
            helperText={claveSatInvalida ? 'Si capturas clave SAT debe tener exactamente 3 caracteres.' : 'Opcional. Ej. H87, KGM'}
            slotProps={{ htmlInput: { maxLength: 3 } }}
          />
        </DialogContent>
        <DialogActions sx={{ ...dialogActionsSx, p: 3, pt: 1 }}>
          <Button onClick={() => setOpen(false)} disabled={saving}>Cancelar</Button>
          <AsyncButton
            variant="contained"
            startIcon={<SaveIcon />}
            onClick={handleSave}
            disabled={saveDisabled}
            loading={saving}
            loadingText="Guardando..."
          >
            Guardar
          </AsyncButton>
        </DialogActions>
      </Dialog>
      <ConfirmActionDialog
        open={Boolean(deleteTarget)}
        title="Eliminar unidad"
        message={`¿Eliminar la unidad "${deleteTarget?.nombre ?? ''}"?`}
        confirmText="Eliminar"
        confirmColor="error"
        loading={Boolean(deletingId)}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={() => {
          if (deleteTarget) return handleDelete(deleteTarget.id);
        }}
      />
      <FeedbackSnackbar message={feedbackMessage} severity={feedbackSeverity} onClose={closeFeedback} />
    </Box>
  );
}

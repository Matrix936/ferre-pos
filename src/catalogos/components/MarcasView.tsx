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
import { Marca } from '../../inventario/types';
import { AsyncButton } from '../../shared/components/AsyncButton';
import { ConfirmActionDialog } from '../../shared/components/ConfirmActionDialog';
import { FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { TablePager } from '../../shared/components/TablePager';
import { TableActions } from '../../shared/components/TableActions';
import { useDialogHotkeys } from '../../shared/hooks/useDialogHotkeys';
import { useFeedback } from '../../shared/hooks/useFeedback';
import { useLocalPagination } from '../../shared/hooks/useLocalPagination';
import { dialogActionsSx, dialogContentSx } from '../../shared/ui/patterns';

export function MarcasView() {
  const { marcas, refreshCatalogos } = useCatalogos();
  const [search, setSearch] = useState('');
  const [open, setOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [currentId, setCurrentId] = useState('');
  const [nombre, setNombre] = useState('');
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState('');
  const [deleteTarget, setDeleteTarget] = useState<Marca | null>(null);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return q ? marcas.filter((marca) => marca.nombre.toLowerCase().includes(q)) : marcas;
  }, [marcas, search]);
  const marcasPager = useLocalPagination(filtered);

  const handleOpen = (marca?: Marca) => {
    setEditMode(Boolean(marca));
    setCurrentId(marca?.id ?? crypto.randomUUID());
    setNombre(marca?.nombre ?? '');
    setOpen(true);
  };

  const handleSave = async () => {
    if (saving) return;
    const marca: Marca = { id: currentId, nombre: nombre.trim() };
    setSaving(true);
    try {
      if (editMode) {
        await invoke('update_marca', { id: currentId, marca });
      } else {
        await invoke('create_marca', { marca });
      }
      setOpen(false);
      await refreshCatalogos();
      showFeedback(editMode ? 'Marca actualizada.' : 'Marca creada.', 'success');
    } catch (error) {
      showFeedback(`Error al guardar: ${error}`, 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    setDeletingId(id);
    try {
      await invoke('delete_marca', { id });
      await refreshCatalogos();
      setDeleteTarget(null);
      showFeedback('Marca eliminada.', 'success');
    } catch (error) {
      showFeedback(`Error al eliminar: ${error}`, 'error');
    } finally {
      setDeletingId('');
    }
  };

  const saveDisabled = saving || !nombre.trim();

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
        <Typography variant="h5" sx={{ fontWeight: 700 }}>Marcas</Typography>
        <Button variant="contained" startIcon={<AddIcon />} onClick={() => handleOpen()} disableElevation>
          Nueva marca
        </Button>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
          <TextField label="Buscar marca" value={search} onChange={(event) => setSearch(event.target.value)} fullWidth />
          <TableActions
            filename="marcas"
            rows={filtered.map((marca) => ({ nombre: marca.nombre }))}
            columns={[{ key: 'nombre', label: 'Nombre' }]}
          />
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Nombre</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {marcasPager.paginatedRows.map((marca) => (
                <TableRow key={marca.id} hover>
                  <TableCell>{marca.nombre}</TableCell>
                  <TableCell>
                    <IconButton color="primary" size="small" onClick={() => handleOpen(marca)} sx={{ mr: 1 }} disabled={Boolean(deletingId)}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="error" size="small" onClick={() => setDeleteTarget(marca)} disabled={Boolean(deletingId)}>
                      {deletingId === marca.id ? <CircularProgress size={18} /> : <DeleteIcon fontSize="small" />}
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))}
              {filtered.length === 0 && (
                <TableRow>
                  <TableCell colSpan={2} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay marcas registradas.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
        <TablePager
          page={marcasPager.page}
          pageSize={marcasPager.pageSize}
          totalPages={marcasPager.totalPages}
          totalRows={marcasPager.totalRows}
          fromRow={marcasPager.fromRow}
          toRow={marcasPager.toRow}
          canPreviousPage={marcasPager.canPreviousPage}
          canNextPage={marcasPager.canNextPage}
          onPreviousPage={marcasPager.previousPage}
          onNextPage={marcasPager.nextPage}
          onPageSizeChange={marcasPager.setPageSize}
          rowLabel="marcas"
        />
      </Paper>

      <Dialog open={open} onClose={saving ? undefined : () => setOpen(false)} maxWidth="xs" fullWidth>
        <DialogTitle sx={{ fontWeight: 600 }}>{editMode ? 'Editar marca' : 'Nueva marca'}</DialogTitle>
        <Divider />
        <DialogContent sx={dialogContentSx}>
          <TextField label="Nombre" value={nombre} onChange={(event) => setNombre(event.target.value)} fullWidth required />
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
        title="Eliminar marca"
        message={`¿Eliminar la marca "${deleteTarget?.nombre ?? ''}"?`}
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

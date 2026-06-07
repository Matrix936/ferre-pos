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
import { Categoria } from '../../inventario/types';
import { AsyncButton } from '../../shared/components/AsyncButton';
import { ConfirmActionDialog } from '../../shared/components/ConfirmActionDialog';
import { FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { TablePager } from '../../shared/components/TablePager';
import { TableActions } from '../../shared/components/TableActions';
import { useDialogHotkeys } from '../../shared/hooks/useDialogHotkeys';
import { useFeedback } from '../../shared/hooks/useFeedback';
import { useLocalPagination } from '../../shared/hooks/useLocalPagination';
import { dialogActionsSx, dialogContentSx } from '../../shared/ui/patterns';

export function CategoriasView() {
  const { categorias, refreshCatalogos } = useCatalogos();
  const [search, setSearch] = useState('');
  const [open, setOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [currentId, setCurrentId] = useState('');
  const [nombre, setNombre] = useState('');
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState('');
  const [deleteTarget, setDeleteTarget] = useState<Categoria | null>(null);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return q ? categorias.filter((categoria) => categoria.nombre.toLowerCase().includes(q)) : categorias;
  }, [categorias, search]);
  const categoriasPager = useLocalPagination(filtered);

  const handleOpen = (categoria?: Categoria) => {
    setEditMode(Boolean(categoria));
    setCurrentId(categoria?.id ?? crypto.randomUUID());
    setNombre(categoria?.nombre ?? '');
    setOpen(true);
  };

  const handleSave = async () => {
    if (saving) return;
    const categoria: Categoria = { id: currentId, nombre: nombre.trim() };
    setSaving(true);
    try {
      if (editMode) {
        await invoke('update_categoria', { id: currentId, categoria });
      } else {
        await invoke('create_categoria', { categoria });
      }
      setOpen(false);
      await refreshCatalogos();
      showFeedback(editMode ? 'Categoría actualizada.' : 'Categoría creada.', 'success');
    } catch (error) {
      showFeedback(`Error al guardar: ${error}`, 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    setDeletingId(id);
    try {
      await invoke('delete_categoria', { id });
      await refreshCatalogos();
      setDeleteTarget(null);
      showFeedback('Categoría eliminada.', 'success');
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
        <Typography variant="h5" sx={{ fontWeight: 700 }}>Categorías</Typography>
        <Button variant="contained" startIcon={<AddIcon />} onClick={() => handleOpen()} disableElevation>
          Nueva categoría
        </Button>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
          <TextField label="Buscar categoría" value={search} onChange={(event) => setSearch(event.target.value)} fullWidth />
          <TableActions
            filename="categorias"
            rows={filtered.map((categoria) => ({ nombre: categoria.nombre }))}
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
              {categoriasPager.paginatedRows.map((categoria) => (
                <TableRow key={categoria.id} hover>
                  <TableCell>{categoria.nombre}</TableCell>
                  <TableCell>
                    <IconButton color="primary" size="small" onClick={() => handleOpen(categoria)} sx={{ mr: 1 }} disabled={Boolean(deletingId)}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="error" size="small" onClick={() => setDeleteTarget(categoria)} disabled={Boolean(deletingId)}>
                      {deletingId === categoria.id ? <CircularProgress size={18} /> : <DeleteIcon fontSize="small" />}
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))}
              {filtered.length === 0 && (
                <TableRow>
                  <TableCell colSpan={2} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay categorías registradas.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
        <TablePager
          page={categoriasPager.page}
          pageSize={categoriasPager.pageSize}
          totalPages={categoriasPager.totalPages}
          totalRows={categoriasPager.totalRows}
          fromRow={categoriasPager.fromRow}
          toRow={categoriasPager.toRow}
          canPreviousPage={categoriasPager.canPreviousPage}
          canNextPage={categoriasPager.canNextPage}
          onPreviousPage={categoriasPager.previousPage}
          onNextPage={categoriasPager.nextPage}
          onPageSizeChange={categoriasPager.setPageSize}
          rowLabel="categorías"
        />
      </Paper>

      <Dialog open={open} onClose={saving ? undefined : () => setOpen(false)} maxWidth="xs" fullWidth>
        <DialogTitle sx={{ fontWeight: 600 }}>{editMode ? 'Editar categoría' : 'Nueva categoría'}</DialogTitle>
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
        title="Eliminar categoría"
        message={`¿Eliminar la categoría "${deleteTarget?.nombre ?? ''}"?`}
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

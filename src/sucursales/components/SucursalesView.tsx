import { useState } from 'react';
import { 
  Box, 
  Typography, 
  Paper, 
  Table, 
  TableBody, 
  TableCell, 
  TableContainer, 
  TableHead, 
  TableRow, 
  Button, 
  CircularProgress,
  IconButton, 
  Dialog, 
  DialogTitle, 
  DialogContent, 
  DialogActions, 
  TextField, 
  Divider
} from '@mui/material';
import { Add as AddIcon, Edit as EditIcon, Delete as DeleteIcon, Save as SaveIcon } from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/core';
import { Sucursal } from '../types';
import { AsyncButton } from '../../shared/components/AsyncButton';
import { ConfirmActionDialog } from '../../shared/components/ConfirmActionDialog';
import { FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { TablePager } from '../../shared/components/TablePager';
import { TableActions } from '../../shared/components/TableActions';
import { useDialogHotkeys } from '../../shared/hooks/useDialogHotkeys';
import { useFeedback } from '../../shared/hooks/useFeedback';
import { useLocalPagination } from '../../shared/hooks/useLocalPagination';
import { dialogActionsSx, dialogContentSx } from '../../shared/ui/patterns';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';

export function SucursalesView() {
  const { sucursales, refreshCatalogos } = useCatalogos();
  const [open, setOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  
  const [currentId, setCurrentId] = useState('');
  const [nombre, setNombre] = useState('');
  const [direccion, setDireccion] = useState('');
  const [telefono, setTelefono] = useState('');
  const [codigoPostal, setCodigoPostal] = useState('');
  const [search, setSearch] = useState('');
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState('');
  const [deleteTarget, setDeleteTarget] = useState<Sucursal | null>(null);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();

  const handleOpen = (sucursal?: Sucursal) => {
    if (sucursal) {
      setEditMode(true);
      setCurrentId(sucursal.id);
      setNombre(sucursal.nombre);
      setDireccion(sucursal.direccion);
      setTelefono(sucursal.telefono);
      setCodigoPostal(sucursal.codigoPostal || '');
    } else {
      setEditMode(false);
      setCurrentId(crypto.randomUUID());
      setNombre('');
      setDireccion('');
      setTelefono('');
      setCodigoPostal('');
    }
    setOpen(true);
  };

  const handleClose = () => {
    setOpen(false);
  };

  const handleSave = async () => {
    if (saving) return;
    const sucursal: Sucursal = {
      id: currentId,
      nombre,
      direccion,
      telefono,
      codigoPostal,
    };

    setSaving(true);
    try {
      if (editMode) {
        await invoke('update_sucursal', { id: currentId, sucursal });
      } else {
        await invoke('create_sucursal', { sucursal });
      }
      handleClose();
      await refreshCatalogos();
      showFeedback(editMode ? 'Sucursal actualizada correctamente.' : 'Sucursal creada correctamente.');
    } catch (error) {
      console.error('Error al guardar sucursal:', error);
      showFeedback(`Error al guardar: ${error}`, 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    setDeletingId(id);
    try {
      await invoke('delete_sucursal', { id });
      await refreshCatalogos();
      setDeleteTarget(null);
      showFeedback('Sucursal eliminada correctamente.');
    } catch (error) {
      console.error('Error al eliminar sucursal:', error);
      showFeedback(`Error al eliminar: ${error}`, 'error');
    } finally {
      setDeletingId('');
    }
  };

  const filteredSucursales = sucursales.filter((sucursal) => {
    const query = search.trim().toLowerCase();
    if (!query) return true;
    return (
      sucursal.nombre.toLowerCase().includes(query) ||
      sucursal.direccion.toLowerCase().includes(query) ||
      sucursal.telefono.toLowerCase().includes(query) ||
      sucursal.codigoPostal.toLowerCase().includes(query)
    );
  });
  const sucursalesPager = useLocalPagination(filteredSucursales);

  const saveDisabled = saving || !nombre || !direccion || !codigoPostal || !currentId;

  useDialogHotkeys({
    open,
    disabled: saveDisabled,
    cancelDisabled: saving,
    onConfirm: handleSave,
    onCancel: handleClose,
  });

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5" sx={{ fontWeight: 700, color: 'text.primary' }}>
          Gestión de sucursales
        </Typography>
        <Button 
          variant="contained" 
          startIcon={<AddIcon />} 
          onClick={() => handleOpen()}
          disableElevation
          sx={{ borderRadius: '8px', px: 3 }}
        >
          Nueva sucursal
        </Button>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap', alignItems: 'center' }}>
          <TextField
            label="Buscar sucursal por nombre, dirección o teléfono"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            fullWidth
          />
          <TableActions
            filename="sucursales"
            rows={filteredSucursales.map((sucursal) => ({
              nombre: sucursal.nombre,
              direccion: sucursal.direccion,
              telefono: sucursal.telefono,
              codigoPostal: sucursal.codigoPostal,
            }))}
            columns={[
              { key: 'nombre', label: 'Nombre' },
              { key: 'direccion', label: 'Dirección' },
              { key: 'telefono', label: 'Teléfono' },
              { key: 'codigoPostal', label: 'Código postal' },
            ]}
          />
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table sx={{ minWidth: 650 }}>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Nombre</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Dirección</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Teléfono</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Código postal</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {sucursalesPager.paginatedRows.map((sucursal) => (
                <TableRow key={sucursal.id} hover sx={{ '&:last-child td, &:last-child th': { border: 0 } }}>
                  <TableCell>{sucursal.nombre}</TableCell>
                  <TableCell>{sucursal.direccion}</TableCell>
                  <TableCell>{sucursal.telefono}</TableCell>
                  <TableCell>{sucursal.codigoPostal || '-'}</TableCell>
                  <TableCell>
                    <IconButton color="primary" onClick={() => handleOpen(sucursal)} size="small" sx={{ mr: 1 }} disabled={Boolean(deletingId)}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="error" onClick={() => setDeleteTarget(sucursal)} size="small" disabled={Boolean(deletingId)}>
                      {deletingId === sucursal.id ? <CircularProgress size={18} /> : <DeleteIcon fontSize="small" />}
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))}
              {filteredSucursales.length === 0 && (
                <TableRow>
                  <TableCell colSpan={5} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay sucursales registradas.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
        <TablePager
          page={sucursalesPager.page}
          pageSize={sucursalesPager.pageSize}
          totalPages={sucursalesPager.totalPages}
          totalRows={sucursalesPager.totalRows}
          fromRow={sucursalesPager.fromRow}
          toRow={sucursalesPager.toRow}
          canPreviousPage={sucursalesPager.canPreviousPage}
          canNextPage={sucursalesPager.canNextPage}
          onPreviousPage={sucursalesPager.previousPage}
          onNextPage={sucursalesPager.nextPage}
          onPageSizeChange={sucursalesPager.setPageSize}
          rowLabel="sucursales"
        />
      </Paper>

      <Dialog open={open} onClose={saving ? undefined : handleClose} maxWidth="sm" fullWidth slotProps={{ paper: { sx: { borderRadius: 2 } } }}>
        <DialogTitle sx={{ fontWeight: 600, pb: 1 }}>
          {editMode ? 'Editar sucursal' : 'Nueva sucursal'}
        </DialogTitle>
        <Divider />
        <DialogContent sx={dialogContentSx}>
          <Box component="form" sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
            <TextField 
              label="Nombre" 
              value={nombre} 
              onChange={(e) => setNombre(e.target.value)} 
              fullWidth 
              required 
            />
            <TextField 
              label="Dirección" 
              value={direccion} 
              onChange={(e) => setDireccion(e.target.value)} 
              fullWidth 
              required 
            />
            <TextField 
              label="Teléfono" 
              value={telefono} 
              onChange={(e) => setTelefono(e.target.value)} 
              fullWidth 
            />
            <TextField
              label="Código postal"
              value={codigoPostal}
              onChange={(e) => setCodigoPostal(e.target.value)}
              fullWidth
              required
              slotProps={{ htmlInput: { maxLength: 5 } }}
            />
          </Box>
        </DialogContent>
        <DialogActions sx={{ ...dialogActionsSx, p: 3, pt: 1 }}>
          <Button onClick={handleClose} disabled={saving} sx={{ borderRadius: '8px' }}>
            Cancelar
          </Button>
          <AsyncButton
            onClick={handleSave}
            variant="contained" 
            disableElevation
            startIcon={<SaveIcon />}
            disabled={saveDisabled}
            loading={saving}
            loadingText="Guardando..."
            sx={{ borderRadius: '8px', px: 3 }}
          >
            Guardar
          </AsyncButton>
        </DialogActions>
      </Dialog>
      <ConfirmActionDialog
        open={Boolean(deleteTarget)}
        title="Eliminar sucursal"
        message={`¿Eliminar la sucursal "${deleteTarget?.nombre ?? ''}"?`}
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

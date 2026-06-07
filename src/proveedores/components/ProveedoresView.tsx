import { useState } from 'react';
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
import { invoke } from '@tauri-apps/api/core';
import { Proveedor } from '../../inventario/types';
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

export function ProveedoresView() {
  const { proveedores, refreshCatalogos } = useCatalogos();
  const [search, setSearch] = useState('');
  const [open, setOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [currentId, setCurrentId] = useState('');
  const [nombre, setNombre] = useState('');
  const [contactoNombre, setContactoNombre] = useState('');
  const [telefono, setTelefono] = useState('');
  const [email, setEmail] = useState('');
  const [direccion, setDireccion] = useState('');
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState('');
  const [deleteTarget, setDeleteTarget] = useState<Proveedor | null>(null);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();

  const handleOpen = (proveedor?: Proveedor) => {
    if (proveedor) {
      setEditMode(true);
      setCurrentId(proveedor.id);
      setNombre(proveedor.nombre);
      setContactoNombre(proveedor.contactoNombre);
      setTelefono(proveedor.telefono);
      setEmail(proveedor.email);
      setDireccion(proveedor.direccion);
    } else {
      setEditMode(false);
      setCurrentId(crypto.randomUUID());
      setNombre('');
      setContactoNombre('');
      setTelefono('');
      setEmail('');
      setDireccion('');
    }
    setOpen(true);
  };

  const handleClose = () => setOpen(false);

  const handleSave = async () => {
    if (saving) return;
    const proveedor: Proveedor = {
      id: currentId,
      nombre: nombre.trim(),
      contactoNombre: contactoNombre.trim(),
      telefono: telefono.trim(),
      email: email.trim(),
      direccion: direccion.trim(),
    };

    setSaving(true);
    try {
      if (editMode) {
        await invoke('update_proveedor', { id: currentId, proveedor });
      } else {
        await invoke('create_proveedor', { proveedor });
      }
      handleClose();
      await refreshCatalogos();
      showFeedback(editMode ? 'Proveedor actualizado correctamente.' : 'Proveedor creado correctamente.');
    } catch (error) {
      console.error('Error al guardar proveedor:', error);
      showFeedback(`Error al guardar: ${error}`, 'error');
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    setDeletingId(id);
    try {
      await invoke('delete_provider', { id });
      await refreshCatalogos();
      setDeleteTarget(null);
      showFeedback('Proveedor eliminado correctamente.');
    } catch (error) {
      console.error('Error al eliminar proveedor:', error);
      showFeedback(`Error al eliminar: ${error}`, 'error');
    } finally {
      setDeletingId('');
    }
  };

  const filtered = proveedores.filter((proveedor) => {
    const q = search.trim().toLowerCase();
    if (!q) return true;
    return (
      proveedor.nombre.toLowerCase().includes(q) ||
      proveedor.contactoNombre.toLowerCase().includes(q) ||
      proveedor.telefono.toLowerCase().includes(q) ||
      proveedor.email.toLowerCase().includes(q)
    );
  });
  const proveedoresPager = useLocalPagination(filtered);

  const saveDisabled = saving || !nombre.trim();

  useDialogHotkeys({
    open,
    disabled: saveDisabled,
    cancelDisabled: saving,
    onConfirm: handleSave,
    onCancel: handleClose,
  });

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3, gap: 2, flexWrap: 'wrap' }}>
        <Typography variant="h5" sx={{ fontWeight: 700, color: 'text.primary' }}>
          Proveedores
        </Typography>
        <Button variant="contained" startIcon={<AddIcon />} onClick={() => handleOpen()} disableElevation sx={{ borderRadius: '8px', px: 3 }}>
          Nuevo proveedor
        </Button>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap', alignItems: 'center' }}>
          <TextField
            label="Buscar proveedor por nombre, contacto, teléfono o correo"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            fullWidth
          />
          <TableActions
            filename="proveedores"
            rows={filtered.map((proveedor) => ({
              nombre: proveedor.nombre,
              contacto: proveedor.contactoNombre,
              telefono: proveedor.telefono,
              email: proveedor.email,
              direccion: proveedor.direccion,
            }))}
            columns={[
              { key: 'nombre', label: 'Nombre' },
              { key: 'contacto', label: 'Contacto' },
              { key: 'telefono', label: 'Teléfono' },
              { key: 'email', label: 'Email' },
              { key: 'direccion', label: 'Dirección' },
            ]}
          />
        </Box>
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table sx={{ minWidth: 860 }}>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Nombre</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Contacto</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Teléfono</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Email</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Dirección</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {proveedoresPager.paginatedRows.map((proveedor) => (
                <TableRow key={proveedor.id} hover>
                  <TableCell>{proveedor.nombre}</TableCell>
                  <TableCell>{proveedor.contactoNombre || '-'}</TableCell>
                  <TableCell>{proveedor.telefono || '-'}</TableCell>
                  <TableCell>{proveedor.email || '-'}</TableCell>
                  <TableCell>{proveedor.direccion || '-'}</TableCell>
                  <TableCell>
                    <IconButton color="primary" size="small" sx={{ mr: 1 }} onClick={() => handleOpen(proveedor)} disabled={Boolean(deletingId)}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="error" size="small" onClick={() => setDeleteTarget(proveedor)} disabled={Boolean(deletingId)}>
                      {deletingId === proveedor.id ? <CircularProgress size={18} /> : <DeleteIcon fontSize="small" />}
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))}
              {filtered.length === 0 && (
                <TableRow>
                  <TableCell colSpan={6} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay proveedores registrados.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
        <TablePager
          page={proveedoresPager.page}
          pageSize={proveedoresPager.pageSize}
          totalPages={proveedoresPager.totalPages}
          totalRows={proveedoresPager.totalRows}
          fromRow={proveedoresPager.fromRow}
          toRow={proveedoresPager.toRow}
          canPreviousPage={proveedoresPager.canPreviousPage}
          canNextPage={proveedoresPager.canNextPage}
          onPreviousPage={proveedoresPager.previousPage}
          onNextPage={proveedoresPager.nextPage}
          onPageSizeChange={proveedoresPager.setPageSize}
          rowLabel="proveedores"
        />
      </Paper>

      <Dialog open={open} onClose={saving ? undefined : handleClose} maxWidth="sm" fullWidth slotProps={{ paper: { sx: { borderRadius: 2 } } }}>
        <DialogTitle sx={{ fontWeight: 600, pb: 1 }}>{editMode ? 'Editar proveedor' : 'Nuevo proveedor'}</DialogTitle>
        <Divider />
        <DialogContent sx={dialogContentSx}>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2.5 }}>
            <TextField label="Nombre de la empresa" value={nombre} onChange={(e) => setNombre(e.target.value)} required fullWidth />
            <TextField label="Nombre del responsable" value={contactoNombre} onChange={(e) => setContactoNombre(e.target.value)} fullWidth />
            <TextField label="Teléfono" value={telefono} onChange={(e) => setTelefono(e.target.value)} fullWidth />
            <TextField label="Email" type="email" value={email} onChange={(e) => setEmail(e.target.value)} fullWidth />
            <TextField label="Dirección" value={direccion} onChange={(e) => setDireccion(e.target.value)} fullWidth multiline minRows={2} />
          </Box>
        </DialogContent>
        <DialogActions sx={{ ...dialogActionsSx, p: 3, pt: 1 }}>
          <Button onClick={handleClose} disabled={saving}>Cancelar</Button>
          <AsyncButton
            onClick={handleSave}
            variant="contained"
            startIcon={<SaveIcon />}
            disableElevation
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
        title="Eliminar proveedor"
        message={`¿Eliminar el proveedor "${deleteTarget?.nombre ?? ''}"?`}
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

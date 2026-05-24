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
import { TableActions } from '../../shared/components/TableActions';
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
    } catch (error) {
      console.error('Error al guardar proveedor:', error);
      alert(`Error al guardar: ${error}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('¿Está seguro de que desea eliminar este proveedor?')) return;
    setDeletingId(id);
    try {
      await invoke('delete_provider', { id });
      await refreshCatalogos();
    } catch (error) {
      console.error('Error al eliminar proveedor:', error);
      alert(`Error al eliminar: ${error}`);
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

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
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
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filtered.map((proveedor) => (
                <TableRow key={proveedor.id} hover>
                  <TableCell>{proveedor.nombre}</TableCell>
                  <TableCell>{proveedor.contactoNombre || '-'}</TableCell>
                  <TableCell>{proveedor.telefono || '-'}</TableCell>
                  <TableCell>{proveedor.email || '-'}</TableCell>
                  <TableCell>{proveedor.direccion || '-'}</TableCell>
                  <TableCell align="right">
                    <IconButton color="primary" size="small" sx={{ mr: 1 }} onClick={() => handleOpen(proveedor)} disabled={Boolean(deletingId)}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="error" size="small" onClick={() => handleDelete(proveedor.id)} disabled={Boolean(deletingId)}>
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
      </Paper>

      <Dialog open={open} onClose={saving ? undefined : handleClose} maxWidth="sm" fullWidth slotProps={{ paper: { sx: { borderRadius: 2 } } }}>
        <DialogTitle sx={{ fontWeight: 600, pb: 1 }}>{editMode ? 'Editar proveedor' : 'Nuevo proveedor'}</DialogTitle>
        <Divider />
        <DialogContent sx={{ pt: 3 }}>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2.5 }}>
            <TextField label="Nombre de la empresa" value={nombre} onChange={(e) => setNombre(e.target.value)} required fullWidth />
            <TextField label="Nombre del responsable" value={contactoNombre} onChange={(e) => setContactoNombre(e.target.value)} fullWidth />
            <TextField label="Teléfono" value={telefono} onChange={(e) => setTelefono(e.target.value)} fullWidth />
            <TextField label="Email" type="email" value={email} onChange={(e) => setEmail(e.target.value)} fullWidth />
            <TextField label="Dirección" value={direccion} onChange={(e) => setDireccion(e.target.value)} fullWidth multiline minRows={2} />
          </Box>
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={handleClose} disabled={saving}>Cancelar</Button>
          <Button
            onClick={handleSave}
            variant="contained"
            startIcon={saving ? <CircularProgress size={18} color="inherit" /> : <SaveIcon />}
            disableElevation
            disabled={saving || !nombre.trim()}
          >
            {saving ? 'Guardando...' : 'Guardar'}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

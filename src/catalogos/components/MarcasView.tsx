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
import { TableActions } from '../../shared/components/TableActions';

export function MarcasView() {
  const { marcas, refreshCatalogos } = useCatalogos();
  const [search, setSearch] = useState('');
  const [open, setOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [currentId, setCurrentId] = useState('');
  const [nombre, setNombre] = useState('');
  const [saving, setSaving] = useState(false);
  const [deletingId, setDeletingId] = useState('');

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return q ? marcas.filter((marca) => marca.nombre.toLowerCase().includes(q)) : marcas;
  }, [marcas, search]);

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
    } catch (error) {
      alert(`Error al guardar: ${error}`);
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('¿Eliminar esta marca?')) return;
    setDeletingId(id);
    try {
      await invoke('delete_marca', { id });
      await refreshCatalogos();
    } catch (error) {
      alert(`Error al eliminar: ${error}`);
    } finally {
      setDeletingId('');
    }
  };

  return (
    <Box sx={{ maxWidth: 960, mx: 'auto', mt: 2 }}>
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
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filtered.map((marca) => (
                <TableRow key={marca.id} hover>
                  <TableCell>{marca.nombre}</TableCell>
                  <TableCell align="right">
                    <IconButton color="primary" size="small" onClick={() => handleOpen(marca)} sx={{ mr: 1 }} disabled={Boolean(deletingId)}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="error" size="small" onClick={() => handleDelete(marca.id)} disabled={Boolean(deletingId)}>
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
      </Paper>

      <Dialog open={open} onClose={saving ? undefined : () => setOpen(false)} maxWidth="xs" fullWidth>
        <DialogTitle sx={{ fontWeight: 600 }}>{editMode ? 'Editar marca' : 'Nueva marca'}</DialogTitle>
        <Divider />
        <DialogContent sx={{ pt: 3 }}>
          <TextField label="Nombre" value={nombre} onChange={(event) => setNombre(event.target.value)} fullWidth required />
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={() => setOpen(false)} disabled={saving}>Cancelar</Button>
          <Button
            variant="contained"
            startIcon={saving ? <CircularProgress size={18} color="inherit" /> : <SaveIcon />}
            onClick={handleSave}
            disabled={saving || !nombre.trim()}
          >
            {saving ? 'Guardando...' : 'Guardar'}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

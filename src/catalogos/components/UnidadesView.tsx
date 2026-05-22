import { useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Box,
  Button,
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
import { TableActions } from '../../shared/components/TableActions';

export function UnidadesView() {
  const { unidades, refreshCatalogos } = useCatalogos();
  const [search, setSearch] = useState('');
  const [open, setOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [currentId, setCurrentId] = useState('');
  const [nombre, setNombre] = useState('');
  const [claveSat, setClaveSat] = useState('');

  const filtered = useMemo(() => {
    const q = search.trim().toLowerCase();
    return q
      ? unidades.filter((unidad) => unidad.nombre.toLowerCase().includes(q) || unidad.claveSat.toLowerCase().includes(q))
      : unidades;
  }, [unidades, search]);

  const handleOpen = (unidad?: UnidadMedida) => {
    setEditMode(Boolean(unidad));
    setCurrentId(unidad?.id ?? crypto.randomUUID());
    setNombre(unidad?.nombre ?? '');
    setClaveSat(unidad?.claveSat ?? '');
    setOpen(true);
  };

  const handleSave = async () => {
    const unidad: UnidadMedida = { id: currentId, nombre: nombre.trim(), claveSat: claveSat.trim().toUpperCase() };
    try {
      if (editMode) {
        await invoke('update_unidad', { id: currentId, unidad });
      } else {
        await invoke('create_unidad', { unidad });
      }
      setOpen(false);
      await refreshCatalogos();
    } catch (error) {
      alert(`Error al guardar: ${error}`);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm('¿Eliminar esta unidad?')) return;
    try {
      await invoke('delete_unidad', { id });
      await refreshCatalogos();
    } catch (error) {
      alert(`Error al eliminar: ${error}`);
    }
  };

  return (
    <Box sx={{ maxWidth: 960, mx: 'auto', mt: 2 }}>
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
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filtered.map((unidad) => (
                <TableRow key={unidad.id} hover>
                  <TableCell>{unidad.nombre}</TableCell>
                  <TableCell>{unidad.claveSat || '-'}</TableCell>
                  <TableCell align="right">
                    <IconButton color="primary" size="small" onClick={() => handleOpen(unidad)} sx={{ mr: 1 }}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="error" size="small" onClick={() => handleDelete(unidad.id)}>
                      <DeleteIcon fontSize="small" />
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
      </Paper>

      <Dialog open={open} onClose={() => setOpen(false)} maxWidth="xs" fullWidth>
        <DialogTitle sx={{ fontWeight: 600 }}>{editMode ? 'Editar unidad' : 'Nueva unidad'}</DialogTitle>
        <Divider />
        <DialogContent sx={{ pt: 3, display: 'flex', flexDirection: 'column', gap: 2 }}>
          <TextField label="Nombre" value={nombre} onChange={(event) => setNombre(event.target.value)} fullWidth required />
          <TextField
            label="Clave Unidad SAT"
            value={claveSat}
            onChange={(event) => setClaveSat(event.target.value.toUpperCase())}
            fullWidth
            required
            helperText="Ej. H87, KGM"
            slotProps={{ htmlInput: { maxLength: 3 } }}
          />
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={() => setOpen(false)}>Cancelar</Button>
          <Button variant="contained" startIcon={<SaveIcon />} onClick={handleSave} disabled={!nombre.trim() || !claveSat.trim()}>
            Guardar
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

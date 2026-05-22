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
import { TableActions } from '../../shared/components/TableActions';
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
    const sucursal: Sucursal = {
      id: currentId,
      nombre,
      direccion,
      telefono,
      codigoPostal,
    };

    try {
      if (editMode) {
        await invoke('update_sucursal', { id: currentId, sucursal });
      } else {
        await invoke('create_sucursal', { sucursal });
      }
      handleClose();
      await refreshCatalogos();
    } catch (error) {
      console.error('Error al guardar sucursal:', error);
      alert(`Error al guardar: ${error}`);
    }
  };

  const handleDelete = async (id: string) => {
    if (confirm('¿Está seguro de que desea eliminar esta sucursal?')) {
      try {
        await invoke('delete_sucursal', { id });
        await refreshCatalogos();
      } catch (error) {
        console.error('Error al eliminar sucursal:', error);
        alert(`Error al eliminar: ${error}`);
      }
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

  return (
    <Box sx={{ maxWidth: 1200, mx: 'auto', mt: 2 }}>
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
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filteredSucursales.map((sucursal) => (
                <TableRow key={sucursal.id} hover sx={{ '&:last-child td, &:last-child th': { border: 0 } }}>
                  <TableCell>{sucursal.nombre}</TableCell>
                  <TableCell>{sucursal.direccion}</TableCell>
                  <TableCell>{sucursal.telefono}</TableCell>
                  <TableCell>{sucursal.codigoPostal || '-'}</TableCell>
                  <TableCell align="right">
                    <IconButton color="primary" onClick={() => handleOpen(sucursal)} size="small" sx={{ mr: 1 }}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="error" onClick={() => handleDelete(sucursal.id)} size="small">
                      <DeleteIcon fontSize="small" />
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
      </Paper>

      <Dialog open={open} onClose={handleClose} maxWidth="sm" fullWidth slotProps={{ paper: { sx: { borderRadius: 2 } } }}>
        <DialogTitle sx={{ fontWeight: 600, pb: 1 }}>
          {editMode ? 'Editar sucursal' : 'Nueva sucursal'}
        </DialogTitle>
        <Divider />
        <DialogContent sx={{ pt: 3 }}>
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
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={handleClose} sx={{ borderRadius: '8px' }}>
            Cancelar
          </Button>
          <Button 
            onClick={handleSave} 
            variant="contained" 
            disableElevation
            startIcon={<SaveIcon />}
            disabled={!nombre || !direccion || !codigoPostal || !currentId}
            sx={{ borderRadius: '8px', px: 3 }}
          >
            Guardar
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

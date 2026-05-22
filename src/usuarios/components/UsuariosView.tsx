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
  Chip, 
  IconButton, 
  Dialog, 
  DialogTitle, 
  DialogContent, 
  DialogActions, 
  TextField, 
  MenuItem,
  Divider
} from '@mui/material';
import { Add as AddIcon, Edit as EditIcon, Delete as DeleteIcon, Save as SaveIcon } from '@mui/icons-material';
import { invoke } from '@tauri-apps/api/core';
import { Usuario } from '../types';
import { Role } from '../../auth/types';
import { TableActions } from '../../shared/components/TableActions';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { useAuth } from '../../auth/context/AuthContext';

function splitNombreCompleto(nombreCompleto: string) {
  const parts = nombreCompleto.trim().split(/\s+/).filter(Boolean);

  if (parts.length === 0) {
    return { nombres: '', apellidoPaterno: '', apellidoMaterno: '' };
  }

  if (parts.length === 1) {
    return { nombres: parts[0], apellidoPaterno: '', apellidoMaterno: '' };
  }

  if (parts.length === 2) {
    return { nombres: parts[0], apellidoPaterno: parts[1], apellidoMaterno: '' };
  }

  return {
    nombres: parts.slice(0, -2).join(' '),
    apellidoPaterno: parts[parts.length - 2],
    apellidoMaterno: parts[parts.length - 1],
  };
}

function buildNombreCompleto(nombres: string, apellidoPaterno: string, apellidoMaterno: string) {
  return [nombres, apellidoPaterno, apellidoMaterno]
    .map((value) => value.trim())
    .filter(Boolean)
    .join(' ');
}

export function UsuariosView() {
  const { user: usuarioSesion } = useAuth();
  const { usuarios, sucursales, refreshCatalogos } = useCatalogos();
  const isAdmin = usuarioSesion?.role === 'ADMIN';
  const [open, setOpen] = useState(false);
  const [editMode, setEditMode] = useState(false);
  
  const [currentId, setCurrentId] = useState('');
  const [nombres, setNombres] = useState('');
  const [apellidoPaterno, setApellidoPaterno] = useState('');
  const [apellidoMaterno, setApellidoMaterno] = useState('');
  const [email, setEmail] = useState('');
  const [role, setRole] = useState<Role>('USUARIO');
  const [sucursalId, setSucursalId] = useState('');
  const [password, setPassword] = useState('');
  const [search, setSearch] = useState('');

  const canManageUsuario = (usuario: Usuario) => {
    if (!usuarioSesion || usuario.id === usuarioSesion.id) return false;
    if (usuarioSesion.role === 'SUPERADMIN') return true;
    return usuario.role === 'USUARIO' && usuario.sucursalId === usuarioSesion.sucursalId;
  };

  const handleOpen = (user?: Usuario) => {
    if (user && !canManageUsuario(user)) {
      alert('No puedes modificar esta cuenta con tu rol actual.');
      return;
    }

    if (user) {
      const nombreSeparado = splitNombreCompleto(user.nombre);
      setEditMode(true);
      setCurrentId(user.id);
      setNombres(nombreSeparado.nombres);
      setApellidoPaterno(nombreSeparado.apellidoPaterno);
      setApellidoMaterno(nombreSeparado.apellidoMaterno);
      setEmail(user.email);
      setRole(user.role);
      setSucursalId(user.sucursalId);
      setPassword(''); // Password isn't fetched, left blank unless changed
    } else {
      setEditMode(false);
      setCurrentId(crypto.randomUUID());
      setNombres('');
      setApellidoPaterno('');
      setApellidoMaterno('');
      setEmail('');
      setRole('USUARIO');
      if (isAdmin && usuarioSesion?.sucursalId) {
        setSucursalId(usuarioSesion.sucursalId);
      } else if (sucursales.length > 0) {
        setSucursalId(sucursales[0].id);
      } else {
        setSucursalId('');
      }
      setPassword('');
    }
    setOpen(true);
  };

  const handleClose = () => {
    setOpen(false);
  };

  const handleSave = async () => {
    const effectiveRole: Role = isAdmin ? 'USUARIO' : role;
    const effectiveSucursalId = isAdmin ? usuarioSesion?.sucursalId || '' : sucursalId;

    if (isAdmin && !effectiveSucursalId) {
      alert('Tu cuenta no tiene sucursal asignada.');
      return;
    }

    const usuario: Usuario = {
      id: currentId,
      nombre: buildNombreCompleto(nombres, apellidoPaterno, apellidoMaterno),
      email: email.trim(),
      role: effectiveRole,
      sucursalId: effectiveSucursalId,
    };

    try {
      if (editMode) {
        await invoke('update_usuario', { id: currentId, usuario });
      } else {
        await invoke('create_usuario', { usuario, password });
      }
      handleClose();
      await refreshCatalogos();
    } catch (error) {
      console.error('Error al guardar usuario:', error);
      alert(`Error al guardar: ${error}`);
    }
  };

  const handleDelete = async (id: string) => {
    const usuario = usuarios.find((item) => item.id === id);
    if (usuario && !canManageUsuario(usuario)) {
      alert('No puedes eliminar esta cuenta con tu rol actual.');
      return;
    }

    if (confirm('¿Está seguro de que desea eliminar este usuario?')) {
      try {
        await invoke('delete_usuario', { id });
        await refreshCatalogos();
      } catch (error) {
        console.error('Error al eliminar usuario:', error);
        alert(`Error al eliminar: ${error}`);
      }
    }
  };

  const getSucursalName = (id: string) => {
    const sucursal = sucursales.find(s => s.id === id);
    return sucursal ? sucursal.nombre : id;
  };

  const filteredUsuarios = usuarios.filter((usuario) => {
    const query = search.trim().toLowerCase();
    if (!query) return true;
    return (
      usuario.nombre.toLowerCase().includes(query) ||
      usuario.email.toLowerCase().includes(query) ||
      usuario.role.toLowerCase().includes(query) ||
      getSucursalName(usuario.sucursalId).toLowerCase().includes(query)
    );
  });

  return (
    <Box sx={{ maxWidth: 1200, mx: 'auto', mt: 2 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
        <Typography variant="h5" sx={{ fontWeight: 700, color: 'text.primary' }}>
          Gestión de usuarios
        </Typography>
        <Button 
          variant="contained" 
          startIcon={<AddIcon />} 
          onClick={() => handleOpen()}
          disableElevation
          sx={{ borderRadius: '8px', px: 3 }}
        >
          Nuevo usuario
        </Button>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap', alignItems: 'center' }}>
          <TextField
            label="Buscar usuario por nombre, correo, rol o sucursal"
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            fullWidth
          />
          <TableActions
            filename="usuarios"
            rows={filteredUsuarios.map((usuario) => ({
              nombre: usuario.nombre,
              email: usuario.email,
              rol: usuario.role,
              sucursal: getSucursalName(usuario.sucursalId),
            }))}
            columns={[
              { key: 'nombre', label: 'Nombre' },
              { key: 'email', label: 'Email' },
              { key: 'rol', label: 'Rol' },
              { key: 'sucursal', label: 'Sucursal' },
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
                <TableCell sx={{ fontWeight: 600 }}>Email</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Rol</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Sucursal</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filteredUsuarios.map((usuario) => (
                <TableRow key={usuario.id} hover sx={{ '&:last-child td, &:last-child th': { border: 0 } }}>
                  <TableCell>{usuario.nombre}</TableCell>
                  <TableCell>{usuario.email}</TableCell>
                  <TableCell>
                    <Chip 
                      label={usuario.role} 
                      color={usuario.role === 'SUPERADMIN' ? 'error' : usuario.role === 'ADMIN' ? 'warning' : 'primary'}
                      size="small"
                      sx={{ borderRadius: '6px', fontWeight: 500 }}
                    />
                  </TableCell>
                  <TableCell>{getSucursalName(usuario.sucursalId)}</TableCell>
                  <TableCell align="right">
                    <IconButton
                      color="primary"
                      onClick={() => handleOpen(usuario)}
                      size="small"
                      disabled={!canManageUsuario(usuario)}
                      sx={{ mr: 1 }}
                    >
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton
                      color="error"
                      onClick={() => handleDelete(usuario.id)}
                      size="small"
                      disabled={!canManageUsuario(usuario)}
                    >
                      <DeleteIcon fontSize="small" />
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))}
              {filteredUsuarios.length === 0 && (
                <TableRow>
                  <TableCell colSpan={5} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay usuarios registrados.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Dialog open={open} onClose={handleClose} maxWidth="sm" fullWidth slotProps={{ paper: { sx: { borderRadius: 2 } } }}>
        <DialogTitle sx={{ fontWeight: 600, pb: 1 }}>
          {editMode ? 'Editar usuario' : 'Nuevo usuario'}
        </DialogTitle>
        <Divider />
        <DialogContent sx={{ pt: 3 }}>
          <Box component="form" sx={{ display: 'flex', flexDirection: 'column', gap: 3 }}>
            <TextField 
              label="Nombres" 
              value={nombres} 
              onChange={(e) => setNombres(e.target.value)} 
              fullWidth 
              required 
            />
            <TextField 
              label="Apellido paterno" 
              value={apellidoPaterno} 
              onChange={(e) => setApellidoPaterno(e.target.value)} 
              fullWidth 
              required 
            />
            <TextField 
              label="Apellido materno" 
              value={apellidoMaterno} 
              onChange={(e) => setApellidoMaterno(e.target.value)} 
              fullWidth 
            />
            <TextField 
              label="Email" 
              type="email" 
              value={email} 
              onChange={(e) => setEmail(e.target.value)} 
              fullWidth 
              required 
            />
            {!editMode && (
              <TextField 
                label="Contraseña" 
                type="password" 
                value={password} 
                onChange={(e) => setPassword(e.target.value)} 
                fullWidth 
                required 
              />
            )}
            <TextField 
              select 
              label="Rol" 
              value={role} 
              onChange={(e) => setRole(e.target.value as Role)} 
              fullWidth 
              required
              disabled={isAdmin}
            >
              {!isAdmin && <MenuItem value="SUPERADMIN">Super Administrador</MenuItem>}
              {!isAdmin && <MenuItem value="ADMIN">Administrador</MenuItem>}
              <MenuItem value="USUARIO">Usuario</MenuItem>
            </TextField>
            <TextField 
              select 
              label="Sucursal" 
              value={isAdmin ? usuarioSesion?.sucursalId || '' : sucursalId} 
              onChange={(e) => setSucursalId(e.target.value)} 
              fullWidth 
              required
              disabled={isAdmin}
            >
              {sucursales.map((sucursal) => (
                <MenuItem key={sucursal.id} value={sucursal.id}>
                  {sucursal.nombre}
                </MenuItem>
              ))}
              {sucursales.length === 0 && (
                <MenuItem value="" disabled>No hay sucursales registradas.</MenuItem>
              )}
            </TextField>
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
            disabled={!nombres || !apellidoPaterno || !email || (!editMode && !password) || !(isAdmin ? usuarioSesion?.sucursalId : sucursalId)}
            sx={{ borderRadius: '8px', px: 3 }}
          >
            Guardar
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}


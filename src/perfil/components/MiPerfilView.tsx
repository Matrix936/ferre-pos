import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Box,
  Button,
  Divider,
  Paper,
  TextField,
  Typography,
} from '@mui/material';
import { Save as SaveIcon } from '@mui/icons-material';
import { User } from '../../auth/types';
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

export function MiPerfilView() {
  const { user, updateUser } = useAuth();
  const [nombres, setNombres] = useState('');
  const [apellidoPaterno, setApellidoPaterno] = useState('');
  const [apellidoMaterno, setApellidoMaterno] = useState('');
  const [email, setEmail] = useState('');
  const [passwordActual, setPasswordActual] = useState('');
  const [nuevaPassword, setNuevaPassword] = useState('');
  const [confirmarPassword, setConfirmarPassword] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  useEffect(() => {
    if (!user) return;

    const nombreSeparado = splitNombreCompleto(user.nombre);
    setNombres(nombreSeparado.nombres);
    setApellidoPaterno(nombreSeparado.apellidoPaterno);
    setApellidoMaterno(nombreSeparado.apellidoMaterno);
    setEmail(user.email);
  }, [user]);

  const handleSave = async () => {
    setError('');
    setSuccess('');

    if (nuevaPassword && nuevaPassword !== confirmarPassword) {
      setError('La nueva contraseña y su confirmación no coinciden.');
      return;
    }

    setSaving(true);
    try {
      const usuario = await invoke<User>('update_mi_perfil', {
        perfil: {
          nombres,
          apellidoPaterno,
          apellidoMaterno,
          email,
          passwordActual,
          nuevaPassword: nuevaPassword || null,
        },
      });

      updateUser(usuario);
      setPasswordActual('');
      setNuevaPassword('');
      setConfirmarPassword('');
      setSuccess('Perfil actualizado correctamente.');
    } catch (err) {
      setError(err as string);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Box sx={{ maxWidth: 900, mx: 'auto', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3, color: 'text.primary' }}>
        Mi perfil
      </Typography>

      <Paper elevation={0} sx={{ p: 4, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
        <Typography variant="h6" sx={{ fontWeight: 600, mb: 1 }}>
          Información personal
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
          Para guardar cualquier cambio se requiere confirmar tu contraseña actual.
        </Typography>

        {error && (
          <Alert severity="error" sx={{ mb: 3 }}>
            {error}
          </Alert>
        )}

        {success && (
          <Alert severity="success" sx={{ mb: 3 }}>
            {success}
          </Alert>
        )}

        <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' }, gap: 3 }}>
          <TextField
            label="Nombres"
            value={nombres}
            onChange={(event) => setNombres(event.target.value)}
            required
            fullWidth
          />
          <TextField
            label="Apellido paterno"
            value={apellidoPaterno}
            onChange={(event) => setApellidoPaterno(event.target.value)}
            required
            fullWidth
          />
          <TextField
            label="Apellido materno"
            value={apellidoMaterno}
            onChange={(event) => setApellidoMaterno(event.target.value)}
            fullWidth
          />
          <TextField
            label="Correo electrónico"
            type="email"
            value={email}
            onChange={(event) => setEmail(event.target.value)}
            required
            fullWidth
          />
        </Box>

        <Divider sx={{ my: 4 }} />

        <Typography variant="h6" sx={{ fontWeight: 600, mb: 3 }}>
          Seguridad
        </Typography>

        <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' }, gap: 3 }}>
          <TextField
            label="Contraseña actual"
            type="password"
            value={passwordActual}
            onChange={(event) => setPasswordActual(event.target.value)}
            required
            fullWidth
          />
          <Box />
          <TextField
            label="Nueva contraseña"
            type="password"
            value={nuevaPassword}
            onChange={(event) => setNuevaPassword(event.target.value)}
            fullWidth
          />
          <TextField
            label="Confirmar nueva contraseña"
            type="password"
            value={confirmarPassword}
            onChange={(event) => setConfirmarPassword(event.target.value)}
            fullWidth
          />
        </Box>

        <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 4 }}>
          <Button
            variant="contained"
            startIcon={<SaveIcon />}
            onClick={handleSave}
            disabled={!nombres || !apellidoPaterno || !email || !passwordActual || saving}
            disableElevation
            sx={{ px: 4 }}
          >
            {saving ? 'Guardando...' : 'Guardar cambios'}
          </Button>
        </Box>
      </Paper>
    </Box>
  );
}

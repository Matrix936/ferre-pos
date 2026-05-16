import { useEffect, useState } from 'react';
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
import { Add as AddIcon, Delete as DeleteIcon, Edit as EditIcon, Payments as AbonoIcon, Save as SaveIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { Cliente } from '../../inventario/types';

export function ClientesView() {
  const { user } = useAuth();
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [search, setSearch] = useState('');

  const [openCliente, setOpenCliente] = useState(false);
  const [editMode, setEditMode] = useState(false);
  const [currentId, setCurrentId] = useState('');
  const [nombre, setNombre] = useState('');
  const [telefono, setTelefono] = useState('');
  const [direccion, setDireccion] = useState('');
  const [limiteCredito, setLimiteCredito] = useState('0');

  const [openAbono, setOpenAbono] = useState(false);
  const [clienteAbono, setClienteAbono] = useState<Cliente | null>(null);
  const [montoAbono, setMontoAbono] = useState('');

  const fetchClientes = async () => {
    try {
      const data = await invoke<Cliente[]>('get_clientes');
      setClientes(data);
    } catch (error) {
      console.error('Error al obtener clientes:', error);
    }
  };

  useEffect(() => {
    fetchClientes();
  }, []);

  const handleOpenCliente = (cliente?: Cliente) => {
    if (cliente) {
      setEditMode(true);
      setCurrentId(cliente.id);
      setNombre(cliente.nombre);
      setTelefono(cliente.telefono);
      setDireccion(cliente.direccion);
      setLimiteCredito(String(cliente.limiteCredito));
    } else {
      setEditMode(false);
      setCurrentId(crypto.randomUUID());
      setNombre('');
      setTelefono('');
      setDireccion('');
      setLimiteCredito('0');
    }
    setOpenCliente(true);
  };

  const handleSaveCliente = async () => {
    const cliente: Cliente = {
      id: currentId,
      nombre: nombre.trim(),
      telefono: telefono.trim(),
      direccion: direccion.trim(),
      limiteCredito: Number(limiteCredito || 0),
      saldoDeudor: 0,
    };
    try {
      if (editMode) {
        await invoke('update_cliente', { id: currentId, cliente });
      } else {
        await invoke('create_cliente', { cliente });
      }
      setOpenCliente(false);
      fetchClientes();
    } catch (error) {
      alert(`Error al guardar cliente: ${error}`);
    }
  };

  const handleDeleteCliente = async (id: string) => {
    if (!confirm('¿Eliminar cliente?')) return;
    try {
      await invoke('delete_cliente', { id });
      fetchClientes();
    } catch (error) {
      alert(`Error al eliminar cliente: ${error}`);
    }
  };

  const handleOpenAbono = (cliente: Cliente) => {
    setClienteAbono(cliente);
    setMontoAbono('');
    setOpenAbono(true);
  };

  const handleRegistrarAbono = async () => {
    if (!clienteAbono || !user?.id) return;
    try {
      await invoke('registrar_abono', {
        abono: {
          id: crypto.randomUUID(),
          clienteId: clienteAbono.id,
          monto: Number(montoAbono || 0),
          fecha: new Date().toISOString(),
          usuarioId: user.id,
        },
      });
      setOpenAbono(false);
      setClienteAbono(null);
      fetchClientes();
    } catch (error) {
      alert(`Error al registrar abono: ${error}`);
    }
  };

  const filtered = clientes.filter((cliente) => {
    const q = search.trim().toLowerCase();
    if (!q) return true;
    return (
      cliente.nombre.toLowerCase().includes(q) ||
      cliente.telefono.toLowerCase().includes(q) ||
      cliente.direccion.toLowerCase().includes(q)
    );
  });

  return (
    <Box sx={{ maxWidth: 1280, mx: 'auto', mt: 2 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3, gap: 2, flexWrap: 'wrap' }}>
        <Typography variant="h5" sx={{ fontWeight: 700 }}>Clientes y Crédito</Typography>
        <Button variant="contained" startIcon={<AddIcon />} onClick={() => handleOpenCliente()} disableElevation>
          Nuevo cliente
        </Button>
      </Box>

      <Paper elevation={0} sx={{ p: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 2 }}>
        <TextField
          label="Buscar cliente por nombre, teléfono o dirección"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          fullWidth
        />
      </Paper>

      <Paper elevation={0} sx={{ borderRadius: 2, border: '1px solid', borderColor: 'divider', overflow: 'hidden' }}>
        <TableContainer>
          <Table>
            <TableHead sx={{ bgcolor: 'background.default' }}>
              <TableRow>
                <TableCell sx={{ fontWeight: 600 }}>Cliente</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Teléfono</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Límite Crédito</TableCell>
                <TableCell sx={{ fontWeight: 600 }}>Saldo Deudor</TableCell>
                <TableCell align="right" sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filtered.map((cliente) => (
                <TableRow key={cliente.id} hover>
                  <TableCell>{cliente.nombre}</TableCell>
                  <TableCell>{cliente.telefono || '-'}</TableCell>
                  <TableCell>${cliente.limiteCredito.toFixed(2)}</TableCell>
                  <TableCell>${cliente.saldoDeudor.toFixed(2)}</TableCell>
                  <TableCell align="right">
                    <IconButton color="primary" size="small" sx={{ mr: 1 }} onClick={() => handleOpenCliente(cliente)}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="success" size="small" sx={{ mr: 1 }} onClick={() => handleOpenAbono(cliente)}>
                      <AbonoIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="error" size="small" onClick={() => handleDeleteCliente(cliente.id)}>
                      <DeleteIcon fontSize="small" />
                    </IconButton>
                  </TableCell>
                </TableRow>
              ))}
              {filtered.length === 0 && (
                <TableRow>
                  <TableCell colSpan={5} align="center" sx={{ py: 4, color: 'text.secondary' }}>
                    No hay clientes registrados.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </TableContainer>
      </Paper>

      <Dialog open={openCliente} onClose={() => setOpenCliente(false)} maxWidth="sm" fullWidth>
        <DialogTitle>{editMode ? 'Editar cliente' : 'Nuevo cliente'}</DialogTitle>
        <Divider />
        <DialogContent sx={{ pt: 3 }}>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2.5 }}>
            <TextField label="Nombre" value={nombre} onChange={(e) => setNombre(e.target.value)} required />
            <TextField label="Teléfono" value={telefono} onChange={(e) => setTelefono(e.target.value)} />
            <TextField label="Dirección" value={direccion} onChange={(e) => setDireccion(e.target.value)} />
            <TextField
              label="Límite de crédito"
              type="number"
              value={limiteCredito}
              onChange={(e) => setLimiteCredito(e.target.value)}
              slotProps={{ htmlInput: { min: 0, step: '0.01' } }}
            />
          </Box>
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={() => setOpenCliente(false)}>Cancelar</Button>
          <Button onClick={handleSaveCliente} variant="contained" startIcon={<SaveIcon />} disableElevation disabled={!nombre.trim()}>
            Guardar
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={openAbono} onClose={() => setOpenAbono(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Registrar abono</DialogTitle>
        <DialogContent sx={{ pt: 2 }}>
          <Typography variant="body2" sx={{ mb: 1.5 }}>
            Cliente: <strong>{clienteAbono?.nombre}</strong>
          </Typography>
          <TextField
            label="Monto del abono"
            type="number"
            fullWidth
            value={montoAbono}
            onChange={(e) => setMontoAbono(e.target.value)}
            slotProps={{ htmlInput: { min: 0.01, step: '0.01' } }}
          />
        </DialogContent>
        <DialogActions sx={{ p: 2 }}>
          <Button onClick={() => setOpenAbono(false)}>Cancelar</Button>
          <Button variant="contained" onClick={handleRegistrarAbono} disabled={!Number(montoAbono || 0)}>
            Registrar
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

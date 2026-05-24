import { useEffect, useState } from 'react';
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
  MenuItem,
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
import { Add as AddIcon, Delete as DeleteIcon, Edit as EditIcon, FactCheck as FiscalIcon, Payments as AbonoIcon, Save as SaveIcon } from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { Cliente } from '../../inventario/types';

interface ClienteDatosFiscales {
  clienteId: string;
  rfc: string;
  razonSocial: string;
  regimenFiscal: string;
  codigoPostal: string;
}

const regimenesFiscales = [
  { value: '601', label: '601 - General de Ley Personas Morales' },
  { value: '605', label: '605 - Sueldos y Salarios' },
  { value: '612', label: '612 - Personas Físicas con Actividades Empresariales' },
  { value: '626', label: '626 - Régimen Simplificado de Confianza' },
];

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
  const [openFiscal, setOpenFiscal] = useState(false);
  const [clienteFiscal, setClienteFiscal] = useState<Cliente | null>(null);
  const [rfc, setRfc] = useState('');
  const [razonSocial, setRazonSocial] = useState('');
  const [regimenFiscal, setRegimenFiscal] = useState('626');
  const [codigoPostalFiscal, setCodigoPostalFiscal] = useState('');
  const [savingCliente, setSavingCliente] = useState(false);
  const [deletingClienteId, setDeletingClienteId] = useState('');
  const [savingFiscal, setSavingFiscal] = useState(false);
  const [savingAbono, setSavingAbono] = useState(false);
  const [loadingFiscalId, setLoadingFiscalId] = useState('');

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
    if (savingCliente) return;
    const cliente: Cliente = {
      id: currentId,
      nombre: nombre.trim(),
      telefono: telefono.trim(),
      direccion: direccion.trim(),
      limiteCredito: Number(limiteCredito || 0),
      saldoDeudor: 0,
    };
    setSavingCliente(true);
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
    } finally {
      setSavingCliente(false);
    }
  };

  const handleDeleteCliente = async (id: string) => {
    if (!confirm('¿Eliminar cliente?')) return;
    setDeletingClienteId(id);
    try {
      await invoke('delete_cliente', { id });
      fetchClientes();
    } catch (error) {
      alert(`Error al eliminar cliente: ${error}`);
    } finally {
      setDeletingClienteId('');
    }
  };

  const handleOpenAbono = (cliente: Cliente) => {
    setClienteAbono(cliente);
    setMontoAbono('');
    setOpenAbono(true);
  };

  const handleOpenFiscal = async (cliente: Cliente) => {
    setLoadingFiscalId(cliente.id);
    setClienteFiscal(cliente);
    setRfc('');
    setRazonSocial(cliente.nombre);
    setRegimenFiscal('626');
    setCodigoPostalFiscal('');
    setOpenFiscal(true);
    try {
      const data = await invoke<ClienteDatosFiscales | null>('get_cliente_datos_fiscales', {
        clienteId: cliente.id,
      });
      if (data) {
        setRfc(data.rfc);
        setRazonSocial(data.razonSocial);
        setRegimenFiscal(data.regimenFiscal);
        setCodigoPostalFiscal(data.codigoPostal);
      }
    } catch (error) {
      alert(`Error al cargar datos fiscales: ${error}`);
    } finally {
      setLoadingFiscalId('');
    }
  };

  const handleGuardarFiscal = async () => {
    if (!clienteFiscal) return;
    setSavingFiscal(true);
    try {
      await invoke('guardar_cliente_datos_fiscales', {
        datos: {
          clienteId: clienteFiscal.id,
          rfc: rfc.trim().toUpperCase(),
          razonSocial: razonSocial.trim(),
          regimenFiscal,
          codigoPostal: codigoPostalFiscal.trim(),
        },
      });
      setOpenFiscal(false);
      setClienteFiscal(null);
    } catch (error) {
      alert(`Error al guardar datos fiscales: ${error}`);
    } finally {
      setSavingFiscal(false);
    }
  };

  const handleRegistrarAbono = async () => {
    if (!clienteAbono || !user?.id) return;
    setSavingAbono(true);
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
    } finally {
      setSavingAbono(false);
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
                    <IconButton color="primary" size="small" sx={{ mr: 1 }} onClick={() => handleOpenCliente(cliente)} disabled={Boolean(deletingClienteId)}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="success" size="small" sx={{ mr: 1 }} onClick={() => handleOpenAbono(cliente)} disabled={Boolean(deletingClienteId)}>
                      <AbonoIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="info" size="small" sx={{ mr: 1 }} onClick={() => handleOpenFiscal(cliente)} disabled={Boolean(deletingClienteId) || Boolean(loadingFiscalId)}>
                      {loadingFiscalId === cliente.id ? <CircularProgress size={18} /> : <FiscalIcon fontSize="small" />}
                    </IconButton>
                    <IconButton color="error" size="small" onClick={() => handleDeleteCliente(cliente.id)} disabled={Boolean(deletingClienteId)}>
                      {deletingClienteId === cliente.id ? <CircularProgress size={18} /> : <DeleteIcon fontSize="small" />}
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

      <Dialog open={openCliente} onClose={savingCliente ? undefined : () => setOpenCliente(false)} maxWidth="sm" fullWidth>
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
          <Button onClick={() => setOpenCliente(false)} disabled={savingCliente}>Cancelar</Button>
          <Button
            onClick={handleSaveCliente}
            variant="contained"
            startIcon={savingCliente ? <CircularProgress size={18} color="inherit" /> : <SaveIcon />}
            disableElevation
            disabled={savingCliente || !nombre.trim()}
          >
            {savingCliente ? 'Guardando...' : 'Guardar'}
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={openAbono} onClose={savingAbono ? undefined : () => setOpenAbono(false)} maxWidth="xs" fullWidth>
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
          <Button onClick={() => setOpenAbono(false)} disabled={savingAbono}>Cancelar</Button>
          <Button
            variant="contained"
            onClick={handleRegistrarAbono}
            startIcon={savingAbono ? <CircularProgress size={18} color="inherit" /> : undefined}
            disabled={savingAbono || !Number(montoAbono || 0)}
          >
            {savingAbono ? 'Registrando...' : 'Registrar'}
          </Button>
        </DialogActions>
      </Dialog>

      <Dialog open={openFiscal} onClose={savingFiscal ? undefined : () => setOpenFiscal(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Datos Fiscales (SAT)</DialogTitle>
        <Divider />
        <DialogContent sx={{ pt: 3 }}>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            Cliente: <strong>{clienteFiscal?.nombre}</strong>
          </Typography>
          <Box sx={{ display: 'grid', gap: 2.5, gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' } }}>
            <TextField
              label="RFC"
              value={rfc}
              onChange={(e) => setRfc(e.target.value.toUpperCase())}
              required
              slotProps={{ htmlInput: { maxLength: 13 } }}
            />
            <TextField
              select
              label="Régimen fiscal"
              value={regimenFiscal}
              onChange={(e) => setRegimenFiscal(e.target.value)}
              required
            >
              {regimenesFiscales.map((regimen) => (
                <MenuItem key={regimen.value} value={regimen.value}>
                  {regimen.label}
                </MenuItem>
              ))}
            </TextField>
            <TextField
              label="Razón social"
              value={razonSocial}
              onChange={(e) => setRazonSocial(e.target.value)}
              required
              sx={{ gridColumn: { xs: 'auto', md: '1 / -1' } }}
            />
            <TextField
              label="Código postal fiscal"
              value={codigoPostalFiscal}
              onChange={(e) => setCodigoPostalFiscal(e.target.value.replace(/\D/g, '').slice(0, 5))}
              required
              slotProps={{ htmlInput: { maxLength: 5 } }}
            />
          </Box>
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={() => setOpenFiscal(false)} disabled={savingFiscal}>Cancelar</Button>
          <Button
            variant="contained"
            startIcon={savingFiscal ? <CircularProgress size={18} color="inherit" /> : <SaveIcon />}
            onClick={handleGuardarFiscal}
            disabled={savingFiscal || rfc.trim().length < 12 || !razonSocial.trim() || !regimenFiscal || codigoPostalFiscal.trim().length !== 5}
            disableElevation
          >
            {savingFiscal ? 'Guardando...' : 'Guardar datos SAT'}
          </Button>
        </DialogActions>
      </Dialog>
    </Box>
  );
}

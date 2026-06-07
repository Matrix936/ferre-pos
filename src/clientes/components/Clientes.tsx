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
import { AsyncButton } from '../../shared/components/AsyncButton';
import { ConfirmActionDialog } from '../../shared/components/ConfirmActionDialog';
import { FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { TablePager } from '../../shared/components/TablePager';
import { useDialogHotkeys } from '../../shared/hooks/useDialogHotkeys';
import { useFeedback } from '../../shared/hooks/useFeedback';
import { useLocalPagination } from '../../shared/hooks/useLocalPagination';
import { dialogActionsSx, dialogContentSx } from '../../shared/ui/patterns';

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

const MONEY_PATTERN = /^\d+(\.\d{0,2})?$/;

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
  const [saldoEdicion, setSaldoEdicion] = useState(0);

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
  const [deleteTarget, setDeleteTarget] = useState<Cliente | null>(null);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();
  const montoAbonoValido =
    MONEY_PATTERN.test(montoAbono.trim()) &&
    Number(montoAbono) > 0 &&
    (!clienteAbono || Number(montoAbono) <= clienteAbono.saldoDeudor);
  const limiteCreditoValido =
    MONEY_PATTERN.test(limiteCredito.trim()) &&
    Number(limiteCredito) >= 0;
  const limiteCreditoMenorASaldo =
    editMode &&
    Number(limiteCredito || 0) > 0 &&
    Math.round(Number(limiteCredito || 0) * 100) < Math.round(saldoEdicion * 100);
  const clienteFormInvalido = !nombre.trim() || !limiteCreditoValido || limiteCreditoMenorASaldo;

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
      setSaldoEdicion(cliente.saldoDeudor);
    } else {
      setEditMode(false);
      setCurrentId(crypto.randomUUID());
      setNombre('');
      setTelefono('');
      setDireccion('');
      setLimiteCredito('0');
      setSaldoEdicion(0);
    }
    setOpenCliente(true);
  };

  const handleSaveCliente = async () => {
    if (savingCliente) return;
    if (!nombre.trim()) {
      showFeedback('Captura el nombre del cliente.', 'warning');
      return;
    }
    if (!limiteCreditoValido) {
      showFeedback('Captura un límite de crédito válido, sin negativos y máximo 2 decimales.', 'warning');
      return;
    }
    if (limiteCreditoMenorASaldo) {
      showFeedback(`El límite de crédito no puede ser menor al saldo deudor actual ($${saldoEdicion.toFixed(2)}).`, 'warning');
      return;
    }
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
      showFeedback(editMode ? 'Cliente actualizado correctamente.' : 'Cliente creado correctamente.');
    } catch (error) {
      showFeedback(`Error al guardar cliente: ${error}`, 'error');
    } finally {
      setSavingCliente(false);
    }
  };

  const handleDeleteCliente = async (id: string) => {
    setDeletingClienteId(id);
    try {
      await invoke('delete_cliente', { id });
      fetchClientes();
      setDeleteTarget(null);
      showFeedback('Cliente eliminado correctamente.');
    } catch (error) {
      showFeedback(`Error al eliminar cliente: ${error}`, 'error');
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
      showFeedback(`Error al cargar datos fiscales: ${error}`, 'error');
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
      showFeedback('Datos fiscales guardados correctamente.');
    } catch (error) {
      showFeedback(`Error al guardar datos fiscales: ${error}`, 'error');
    } finally {
      setSavingFiscal(false);
    }
  };

  const handleRegistrarAbono = async () => {
    if (!clienteAbono || !user?.id) return;
    if (!montoAbonoValido) {
      showFeedback('Captura un abono válido, mayor a cero y no superior al saldo deudor.', 'warning');
      return;
    }

    setSavingAbono(true);
    try {
      await invoke('registrar_abono', {
        abono: {
          id: crypto.randomUUID(),
          clienteId: clienteAbono.id,
          monto: Math.round(Number(montoAbono) * 100) / 100,
          fecha: new Date().toISOString(),
          usuarioId: user.id,
        },
      });
      setOpenAbono(false);
      setClienteAbono(null);
      fetchClientes();
      showFeedback('Abono registrado correctamente.');
    } catch (error) {
      showFeedback(`Error al registrar abono: ${error}`, 'error');
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
  const clientesPager = useLocalPagination(filtered);

  const clienteSaveDisabled = savingCliente || clienteFormInvalido;
  const abonoSaveDisabled = savingAbono || !montoAbonoValido;
  const fiscalSaveDisabled =
    savingFiscal || rfc.trim().length < 12 || !razonSocial.trim() || !regimenFiscal || codigoPostalFiscal.trim().length !== 5;

  useDialogHotkeys({
    open: openCliente,
    disabled: clienteSaveDisabled,
    cancelDisabled: savingCliente,
    onConfirm: handleSaveCliente,
    onCancel: () => setOpenCliente(false),
  });
  useDialogHotkeys({
    open: openAbono,
    disabled: abonoSaveDisabled,
    cancelDisabled: savingAbono,
    onConfirm: handleRegistrarAbono,
    onCancel: () => setOpenAbono(false),
  });
  useDialogHotkeys({
    open: openFiscal,
    disabled: fiscalSaveDisabled,
    cancelDisabled: savingFiscal,
    onConfirm: handleGuardarFiscal,
    onCancel: () => setOpenFiscal(false),
  });

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
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
                <TableCell sx={{ fontWeight: 600 }}>Acciones</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {clientesPager.paginatedRows.map((cliente) => (
                <TableRow key={cliente.id} hover>
                  <TableCell>{cliente.nombre}</TableCell>
                  <TableCell>{cliente.telefono || '-'}</TableCell>
                  <TableCell>${cliente.limiteCredito.toFixed(2)}</TableCell>
                  <TableCell>${cliente.saldoDeudor.toFixed(2)}</TableCell>
                  <TableCell>
                    <IconButton color="primary" size="small" sx={{ mr: 1 }} onClick={() => handleOpenCliente(cliente)} disabled={Boolean(deletingClienteId)}>
                      <EditIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="success" size="small" sx={{ mr: 1 }} onClick={() => handleOpenAbono(cliente)} disabled={Boolean(deletingClienteId)}>
                      <AbonoIcon fontSize="small" />
                    </IconButton>
                    <IconButton color="info" size="small" sx={{ mr: 1 }} onClick={() => handleOpenFiscal(cliente)} disabled={Boolean(deletingClienteId) || Boolean(loadingFiscalId)}>
                      {loadingFiscalId === cliente.id ? <CircularProgress size={18} /> : <FiscalIcon fontSize="small" />}
                    </IconButton>
                    <IconButton color="error" size="small" onClick={() => setDeleteTarget(cliente)} disabled={Boolean(deletingClienteId)}>
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
        <TablePager
          page={clientesPager.page}
          pageSize={clientesPager.pageSize}
          totalPages={clientesPager.totalPages}
          totalRows={clientesPager.totalRows}
          fromRow={clientesPager.fromRow}
          toRow={clientesPager.toRow}
          canPreviousPage={clientesPager.canPreviousPage}
          canNextPage={clientesPager.canNextPage}
          onPreviousPage={clientesPager.previousPage}
          onNextPage={clientesPager.nextPage}
          onPageSizeChange={clientesPager.setPageSize}
          rowLabel="clientes"
        />
      </Paper>

      <Dialog open={openCliente} onClose={savingCliente ? undefined : () => setOpenCliente(false)} maxWidth="sm" fullWidth>
        <DialogTitle>{editMode ? 'Editar cliente' : 'Nuevo cliente'}</DialogTitle>
        <Divider />
        <DialogContent sx={{ ...dialogContentSx, gap: 2.5 }}>
          <Box sx={{ display: 'flex', flexDirection: 'column', gap: 2.5 }}>
            <TextField label="Nombre" value={nombre} onChange={(e) => setNombre(e.target.value)} required />
            <TextField label="Teléfono" value={telefono} onChange={(e) => setTelefono(e.target.value)} />
            <TextField label="Dirección" value={direccion} onChange={(e) => setDireccion(e.target.value)} />
            <TextField
              label="Límite de crédito"
              type="number"
              value={limiteCredito}
              onChange={(e) => setLimiteCredito(e.target.value)}
              error={Boolean(limiteCredito) && !limiteCreditoValido}
              helperText={
                Boolean(limiteCredito) && !limiteCreditoValido
                  ? 'Debe ser mayor o igual a cero y tener máximo 2 decimales.'
                  : limiteCreditoMenorASaldo
                    ? `No puede ser menor al saldo deudor actual: $${saldoEdicion.toFixed(2)}.`
                  : 'Usa 0 para clientes sin crédito autorizado.'
              }
              slotProps={{ htmlInput: { min: 0, step: '0.01' } }}
            />
          </Box>
        </DialogContent>
        <DialogActions sx={{ ...dialogActionsSx, p: 3, pt: 1 }}>
          <Button onClick={() => setOpenCliente(false)} disabled={savingCliente}>Cancelar</Button>
          <AsyncButton
            onClick={handleSaveCliente}
            variant="contained"
            startIcon={<SaveIcon />}
            disableElevation
            disabled={clienteSaveDisabled}
            loading={savingCliente}
            loadingText="Guardando..."
          >
            Guardar
          </AsyncButton>
        </DialogActions>
      </Dialog>

      <Dialog open={openAbono} onClose={savingAbono ? undefined : () => setOpenAbono(false)} maxWidth="xs" fullWidth>
        <DialogTitle>Registrar abono</DialogTitle>
        <DialogContent sx={dialogContentSx}>
          <Typography variant="body2" sx={{ mb: 1.5 }}>
            Cliente: <strong>{clienteAbono?.nombre}</strong>
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            Saldo pendiente: ${clienteAbono?.saldoDeudor.toFixed(2) ?? '0.00'}
          </Typography>
          <TextField
            label="Monto del abono"
            type="number"
            fullWidth
            value={montoAbono}
            onChange={(e) => setMontoAbono(e.target.value)}
            error={Boolean(montoAbono) && !montoAbonoValido}
            helperText={
              Boolean(montoAbono) && !montoAbonoValido
                ? 'Debe ser mayor a cero, máximo 2 decimales y no superar el saldo.'
                : 'El abono se registrará como ingreso en la caja abierta.'
            }
            slotProps={{ htmlInput: { min: 0.01, step: '0.01', inputMode: 'decimal' } }}
          />
        </DialogContent>
        <DialogActions sx={dialogActionsSx}>
          <Button onClick={() => setOpenAbono(false)} disabled={savingAbono}>Cancelar</Button>
          <AsyncButton
            variant="contained"
            onClick={handleRegistrarAbono}
            disabled={abonoSaveDisabled}
            loading={savingAbono}
            loadingText="Registrando..."
          >
            Registrar
          </AsyncButton>
        </DialogActions>
      </Dialog>

      <Dialog open={openFiscal} onClose={savingFiscal ? undefined : () => setOpenFiscal(false)} maxWidth="sm" fullWidth>
        <DialogTitle>Datos Fiscales (SAT)</DialogTitle>
        <Divider />
        <DialogContent sx={dialogContentSx}>
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
        <DialogActions sx={{ ...dialogActionsSx, p: 3, pt: 1 }}>
          <Button onClick={() => setOpenFiscal(false)} disabled={savingFiscal}>Cancelar</Button>
          <AsyncButton
            variant="contained"
            startIcon={<SaveIcon />}
            onClick={handleGuardarFiscal}
            disabled={fiscalSaveDisabled}
            loading={savingFiscal}
            loadingText="Guardando..."
            disableElevation
          >
            Guardar datos SAT
          </AsyncButton>
        </DialogActions>
      </Dialog>
      <ConfirmActionDialog
        open={Boolean(deleteTarget)}
        title="Eliminar cliente"
        message={`¿Eliminar el cliente "${deleteTarget?.nombre ?? ''}"?`}
        confirmText="Eliminar"
        confirmColor="error"
        loading={Boolean(deletingClienteId)}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={() => {
          if (deleteTarget) return handleDeleteCliente(deleteTarget.id);
        }}
      />
      <FeedbackSnackbar message={feedbackMessage} severity={feedbackSeverity} onClose={closeFeedback} />
    </Box>
  );
}

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Alert, Box, Button, CircularProgress, Divider, Paper, TextField, Typography } from '@mui/material';
import { Business as BusinessIcon, Save as SaveIcon } from '@mui/icons-material';
import { configActionButtonSx, configIconBadgeSx, configPanelSx, configSectionHeaderSx } from '../components/configSectionStyles';

interface EmpresaConfigFiscal {
  rfc: string;
  razonSocial: string;
  regimenFiscal: string;
  registroPatronal?: string | null;
  actualizadoAt: string;
}

export function ConfiguracionFiscalView() {
  const [rfc, setRfc] = useState('');
  const [razonSocial, setRazonSocial] = useState('');
  const [regimenFiscal, setRegimenFiscal] = useState('');
  const [registroPatronal, setRegistroPatronal] = useState('');
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    invoke<EmpresaConfigFiscal | null>('get_empresa_config')
      .then((config) => {
        if (!config) return;
        setRfc(config.rfc || '');
        setRazonSocial(config.razonSocial || '');
        setRegimenFiscal(config.regimenFiscal || '');
        setRegistroPatronal(config.registroPatronal || '');
      })
      .catch((err) => {
        console.error('Error al cargar configuración fiscal:', err);
        setError(String(err));
      });
  }, []);

  const handleSave = async () => {
    setLoading(true);
    setError('');
    setMessage('');
    try {
      await invoke<EmpresaConfigFiscal>('guardar_empresa_config', {
        config: {
          rfc: rfc.trim().toUpperCase(),
          razonSocial: razonSocial.trim(),
          regimenFiscal: regimenFiscal.trim(),
          registroPatronal: registroPatronal.trim() || null,
          actualizadoAt: new Date().toISOString(),
        },
      });
      setMessage('Configuración fiscal guardada.');
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <Paper elevation={0} sx={configPanelSx}>
      <Box sx={configSectionHeaderSx}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
          <Box sx={configIconBadgeSx}>
            <BusinessIcon fontSize="small" />
          </Box>
          <Box>
            <Typography variant="h6" sx={{ fontWeight: 800 }}>
              Configuración fiscal del emisor
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Datos fiscales globales usados para pre-facturación CFDI.
            </Typography>
          </Box>
        </Box>
      </Box>
      <Divider sx={{ mb: 3 }} />

      <Box sx={{ display: 'grid', gap: 2.5, gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' } }}>
        <TextField
          label="RFC"
          value={rfc}
          onChange={(e) => setRfc(e.target.value.toUpperCase())}
          required
          slotProps={{ htmlInput: { maxLength: 13 } }}
        />
        <TextField
          label="Régimen fiscal"
          value={regimenFiscal}
          onChange={(e) => setRegimenFiscal(e.target.value.replace(/\D/g, '').slice(0, 3))}
          required
          slotProps={{ htmlInput: { maxLength: 3 } }}
        />
        <TextField
          label="Razón social"
          value={razonSocial}
          onChange={(e) => setRazonSocial(e.target.value)}
          required
          sx={{ gridColumn: { xs: 'auto', md: '1 / -1' } }}
        />
        <TextField
          label="Registro patronal"
          value={registroPatronal}
          onChange={(e) => setRegistroPatronal(e.target.value)}
          sx={{ gridColumn: { xs: 'auto', md: '1 / -1' } }}
        />
      </Box>

      {message && <Alert severity="success" sx={{ mt: 3 }}>{message}</Alert>}
      {error && <Alert severity="error" sx={{ mt: 3 }}>{error}</Alert>}

      <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 3 }}>
        <Button
          variant="contained"
          startIcon={loading ? <CircularProgress size={18} color="inherit" /> : <SaveIcon />}
          onClick={handleSave}
          disabled={loading || !rfc.trim() || !razonSocial.trim() || !regimenFiscal.trim()}
          disableElevation
          sx={configActionButtonSx}
        >
          {loading ? 'Guardando...' : 'Guardar configuración fiscal'}
        </Button>
      </Box>
    </Paper>
  );
}

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Divider,
  MenuItem,
  Paper,
  TextField,
  Typography,
} from '@mui/material';
import { Print as PrintIcon, Refresh as RefreshIcon, Save as SaveIcon } from '@mui/icons-material';
import { configActionButtonSx, configIconBadgeSx, configPanelSx, configSectionHeaderSx } from '../components/configSectionStyles';

interface PerifericosConfig {
  impresoraTickets: string;
  impresoraEtiquetas: string;
  updatedAt: string;
}

const emptyConfig: PerifericosConfig = {
  impresoraTickets: '',
  impresoraEtiquetas: '',
  updatedAt: '',
};

export function PerifericosConfigView() {
  const [printers, setPrinters] = useState<string[]>([]);
  const [config, setConfig] = useState<PerifericosConfig>(emptyConfig);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState('');
  const [error, setError] = useState('');

  const load = async () => {
    setLoading(true);
    setError('');
    try {
      const [systemPrinters, savedConfig] = await Promise.all([
        invoke<string[]>('get_system_printers'),
        invoke<PerifericosConfig>('get_perifericos_config'),
      ]);
      setPrinters(systemPrinters);
      setConfig(savedConfig ?? emptyConfig);
    } catch (loadError) {
      setError(String(loadError));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    load().catch((loadError) => setError(String(loadError)));
  }, []);

  const handleSave = async () => {
    setSaving(true);
    setError('');
    setMessage('');
    try {
      const saved = await invoke<PerifericosConfig>('guardar_perifericos_config', {
        config: {
          impresoraTickets: config.impresoraTickets,
          impresoraEtiquetas: config.impresoraEtiquetas,
        },
      });
      setConfig(saved);
      setMessage('Periféricos guardados correctamente.');
    } catch (saveError) {
      setError(String(saveError));
    } finally {
      setSaving(false);
    }
  };

  return (
    <Paper elevation={0} sx={{ ...configPanelSx, mb: 3, position: 'relative' }}>
      {loading && (
        <Box
          sx={{
            position: 'absolute',
            inset: 0,
            zIndex: 2,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            bgcolor: (theme) => theme.palette.mode === 'dark' ? 'rgba(18,18,18,0.58)' : 'rgba(255,255,255,0.72)',
            backdropFilter: 'blur(2px)',
          }}
        >
          <Paper
            elevation={0}
            sx={{
              px: 2.5,
              py: 1.5,
              borderRadius: 2,
              border: '1px solid',
              borderColor: 'divider',
              display: 'flex',
              alignItems: 'center',
              gap: 1.5,
            }}
          >
            <CircularProgress size={22} />
            <Typography variant="body2" sx={{ fontWeight: 700 }}>
              Buscando impresoras...
            </Typography>
          </Paper>
        </Box>
      )}
      <Box sx={configSectionHeaderSx}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
          <Box sx={configIconBadgeSx}>
            <PrintIcon fontSize="small" />
          </Box>
          <Box>
            <Typography variant="h6" sx={{ fontWeight: 800 }}>
              Periféricos e impresión silenciosa
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Define las impresoras que usará el POS sin abrir diálogos de Windows.
            </Typography>
          </Box>
        </Box>
        <Button
          startIcon={loading ? <CircularProgress size={16} color="inherit" /> : <RefreshIcon />}
          onClick={() => void load()}
          disabled={loading || saving}
          sx={configActionButtonSx}
        >
          {loading ? 'Buscando...' : 'Actualizar'}
        </Button>
      </Box>
      <Divider sx={{ mb: 3 }} />

      {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}
      {message && <Alert severity="success" sx={{ mb: 2 }}>{message}</Alert>}
      {printers.length === 0 && !loading && (
        <Alert severity="warning" sx={{ mb: 2 }}>
          No se encontraron impresoras instaladas. Revisa los drivers de Windows y vuelve a actualizar.
        </Alert>
      )}

      <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' }, gap: 2 }}>
        <TextField
          select
          label="Impresora de Tickets Predeterminada"
          value={config.impresoraTickets}
          onChange={(event) => setConfig((prev) => ({ ...prev, impresoraTickets: event.target.value }))}
          disabled={loading}
          helperText="Tickets térmicos y apertura de cajón."
        >
          <MenuItem value="">Sin configurar</MenuItem>
          {printers.map((printer) => (
            <MenuItem key={printer} value={printer}>
              {printer}
            </MenuItem>
          ))}
        </TextField>

        <TextField
          select
          label="Impresora de Etiquetas Predeterminada"
          value={config.impresoraEtiquetas}
          onChange={(event) => setConfig((prev) => ({ ...prev, impresoraEtiquetas: event.target.value }))}
          disabled={loading}
          helperText="Etiquetas de precio desde inventario."
        >
          <MenuItem value="">Sin configurar</MenuItem>
          {printers.map((printer) => (
            <MenuItem key={printer} value={printer}>
              {printer}
            </MenuItem>
          ))}
        </TextField>
      </Box>

      <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 3 }}>
        <Button
          variant="contained"
          startIcon={saving ? <CircularProgress size={18} color="inherit" /> : <SaveIcon />}
          onClick={handleSave}
          disabled={loading || saving}
          disableElevation
          sx={configActionButtonSx}
        >
          {saving ? 'Guardando...' : 'Guardar periféricos'}
        </Button>
      </Box>

      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mt: 2, color: 'text.secondary' }}>
        <PrintIcon fontSize="small" />
        <Typography variant="caption">
          La impresión se enviará directo al spooler del sistema usando el nombre del driver seleccionado.
        </Typography>
      </Box>
    </Paper>
  );
}

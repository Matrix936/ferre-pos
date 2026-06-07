import { useRef, useState } from 'react';
import {
  Alert,
  Box,
  Button,
  CircularProgress,
  Divider,
  FormControlLabel,
  Paper,
  Switch,
  TextField,
  Typography,
} from '@mui/material';
import { CloudUpload as CloudUploadIcon, Palette as PaletteIcon, Save as SaveIcon } from '@mui/icons-material';
import { useConfig } from '../../config/context/ConfigContext';
import { validateLogoFile } from '../../shared/utils/logoValidation';
import { configActionButtonSx, configIconBadgeSx, configPanelSx, configSectionHeaderSx } from './configSectionStyles';

export function SeccionApariencia() {
  const { systemName, setSystemName, logo, setLogo, logoAnimationEnabled, setLogoAnimationEnabled } = useConfig();
  const [tempName, setTempName] = useState(systemName);
  const [error, setError] = useState<string | null>(null);
  const [logoInfo, setLogoInfo] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleSave = () => {
    setSaving(true);
    setSystemName(tempName);
    window.setTimeout(() => setSaving(false), 450);
  };

  const handleLogoUpload = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      setError(null);
      setLogoInfo(null);
      try {
        const result = await validateLogoFile(file);
        setLogo(result.dataUrl);
        setLogoInfo(result.warning || `Logo cargado correctamente (${result.width}x${result.height}px).`);
      } catch (validationError) {
        setError(validationError instanceof Error ? validationError.message : 'No se pudo validar el logo.');
      }
    }
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  return (
    <Paper elevation={0} sx={configPanelSx}>
      <Box sx={configSectionHeaderSx}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
          <Box sx={configIconBadgeSx}>
            <PaletteIcon fontSize="small" />
          </Box>
          <Box>
            <Typography variant="h6" sx={{ fontWeight: 800 }}>
              Apariencia
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Ajusta identidad visual, logo y comportamiento del sidebar.
            </Typography>
          </Box>
        </Box>
      </Box>
      <Divider sx={{ mb: 3 }} />

      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 3, alignItems: 'flex-start' }}>
        <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: 2, width: '100%' }}>
          <Typography variant="subtitle2" color="text.secondary">
            Logotipo del sistema
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Acepta logos horizontales, cuadrados o verticales. Se adaptan al espacio sin recortarse.
          </Typography>

          {error && (
            <Alert severity="error" sx={{ width: '100%' }}>
              {error}
            </Alert>
          )}
          {logoInfo && (
            <Alert severity={logoInfo.startsWith('El logo') ? 'warning' : 'success'} sx={{ width: '100%' }}>
              {logoInfo}
            </Alert>
          )}

          <Box
            sx={{
              width: '100%',
              maxWidth: 520,
              minHeight: 172,
              bgcolor: 'background.default',
              border: '1px dashed',
              borderColor: 'divider',
              boxShadow: 'inset 0 0 10px rgba(0,0,0,0.05)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              overflow: 'hidden',
              borderRadius: 2,
            }}
          >
            {logo ? (
              <Box component="img" src={logo} alt="Logo" sx={{ maxWidth: '92%', maxHeight: 140, objectFit: 'contain' }} />
            ) : (
              <Typography color="text.secondary">Sin Logo</Typography>
            )}
          </Box>

          {logo && (
            <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', sm: '1fr 1fr' }, gap: 2, width: '100%', maxWidth: 520 }}>
              <Box sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 2, p: 1.5, bgcolor: 'background.default' }}>
                <Typography variant="caption" color="text.secondary">
                  Sidebar abierto
                </Typography>
                <Box sx={{ height: 54, mt: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', bgcolor: 'background.paper', borderRadius: 1.5 }}>
                  <Box component="img" src={logo} alt="Vista sidebar abierto" sx={{ maxWidth: 180, maxHeight: 42, objectFit: 'contain' }} />
                </Box>
              </Box>
              <Box sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 2, p: 1.5, bgcolor: 'background.default' }}>
                <Typography variant="caption" color="text.secondary">
                  Sidebar cerrado
                </Typography>
                <Box sx={{ height: 54, mt: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', bgcolor: 'background.paper', borderRadius: 1.5 }}>
                  <Box component="img" src={logo} alt="Vista sidebar cerrado" sx={{ maxWidth: 44, maxHeight: 44, objectFit: 'contain' }} />
                </Box>
              </Box>
            </Box>
          )}

          <input type="file" accept="image/*" hidden ref={fileInputRef} onChange={handleLogoUpload} />
          <Box sx={{ display: 'flex', gap: 2, flexWrap: 'wrap' }}>
            <Button variant="outlined" startIcon={<CloudUploadIcon />} onClick={() => fileInputRef.current?.click()} sx={configActionButtonSx}>
              Cargar logo
            </Button>
            {logo && (
              <Button
                variant="text"
                color="error"
                size="small"
                onClick={() => {
                  setLogo(null);
                  setLogoInfo(null);
                  setError(null);
                }}
              >
                Quitar Logo
              </Button>
            )}
          </Box>

          <FormControlLabel
            control={<Switch checked={logoAnimationEnabled} onChange={(event) => setLogoAnimationEnabled(event.target.checked)} />}
            label="Animar logo en el sistema"
          />
        </Box>

        <Divider sx={{ width: '100%' }} />

        <Box sx={{ flex: 1, width: '100%' }}>
          <TextField
            fullWidth
            label="Nombre de la empresa"
            value={tempName}
            onChange={(event) => setTempName(event.target.value)}
            helperText="Este nombre aparecerá en la barra superior y el menú lateral."
            sx={{ mb: 3 }}
          />

          <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 2 }}>
            <Button
              variant="contained"
              startIcon={saving ? <CircularProgress size={18} color="inherit" /> : <SaveIcon />}
              onClick={handleSave}
              disabled={saving}
          disableElevation
          sx={configActionButtonSx}
        >
              {saving ? 'Guardando...' : 'Guardar cambios'}
            </Button>
          </Box>
        </Box>
      </Box>
    </Paper>
  );
}

import { useState, useRef } from 'react';
import { 
  Box, 
  Typography, 
  Paper, 
  TextField, 
  Button, 
  CircularProgress,
  Divider,
  Alert,
  FormControlLabel,
  Switch
} from '@mui/material';
import { CloudUpload as CloudUploadIcon, Save as SaveIcon } from '@mui/icons-material';
import { useConfig } from '../../config/context/ConfigContext';
import { ConfiguracionFiscalView } from '../views/ConfiguracionFiscalView';
import { SincronizacionConfigView } from '../views/SincronizacionConfigView';
import { validateLogoFile } from '../../shared/utils/logoValidation';

export function ConfiguracionView() {
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
    // Clear input so the same file can be selected again
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  return (
    <Box sx={{ maxWidth: 900, mx: 'auto', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3, color: 'text.primary' }}>
        Configuración del sistema
      </Typography>

      <Paper elevation={0} sx={{ p: 4, borderRadius: 2, border: '1px solid', borderColor: 'divider', mb: 3 }}>
        <Typography variant="h6" sx={{ mb: 2, fontWeight: 600 }}>
          Apariencia
        </Typography>
        <Divider sx={{ mb: 4 }} />

        <Box sx={{ display: 'flex', flexDirection: { xs: 'column', md: 'column' }, gap: 4, alignItems: 'flex-start' }}>
          
          {/* Logo Section */}
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
                borderRadius: 1
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
                <Box sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 1, p: 1.5 }}>
                  <Typography variant="caption" color="text.secondary">Sidebar abierto</Typography>
                  <Box sx={{ height: 54, mt: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', bgcolor: 'background.default', borderRadius: 1 }}>
                    <Box component="img" src={logo} alt="Vista sidebar abierto" sx={{ maxWidth: 180, maxHeight: 42, objectFit: 'contain' }} />
                  </Box>
                </Box>
                <Box sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 1, p: 1.5 }}>
                  <Typography variant="caption" color="text.secondary">Sidebar cerrado</Typography>
                  <Box sx={{ height: 54, mt: 1, display: 'flex', alignItems: 'center', justifyContent: 'center', bgcolor: 'background.default', borderRadius: 1 }}>
                    <Box component="img" src={logo} alt="Vista sidebar cerrado" sx={{ maxWidth: 44, maxHeight: 44, objectFit: 'contain' }} />
                  </Box>
                </Box>
              </Box>
            )}
            
            <input
              type="file"
              accept="image/*"
              hidden
              ref={fileInputRef}
              onChange={handleLogoUpload}
            />
            <Box sx={{ display: 'flex', gap: 2 }}>
              <Button 
                variant="outlined" 
                startIcon={<CloudUploadIcon />}
                onClick={() => fileInputRef.current?.click()}
                sx={{ borderRadius: '8px' }}
              >
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
              control={
                <Switch
                  checked={logoAnimationEnabled}
                  onChange={(event) => setLogoAnimationEnabled(event.target.checked)}
                />
              }
              label="Animar logo en el sistema"
            />
          </Box>

          <Divider sx={{ width: '100%' }} />

          {/* Form Section */}
          <Box sx={{ flex: 1, width: '100%' }}>
            <TextField
              fullWidth
              label="Nombre de la empresa"
              value={tempName}
              onChange={(e) => setTempName(e.target.value)}
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
                sx={{ px: 4, py: 1, borderRadius: '8px' }}
              >
                {saving ? 'Guardando...' : 'Guardar cambios'}
              </Button>
            </Box>
          </Box>

        </Box>
      </Paper>
      <SincronizacionConfigView />
      <ConfiguracionFiscalView />
    </Box>
  );
}

import { useState, useRef } from 'react';
import { 
  Box, 
  Typography, 
  Paper, 
  TextField, 
  Button, 
  Divider,
  Alert
} from '@mui/material';
import { CloudUpload as CloudUploadIcon, Save as SaveIcon } from '@mui/icons-material';
import { useConfig } from '../../config/context/ConfigContext';

export function ConfiguracionView() {
  const { systemName, setSystemName, logo, setLogo } = useConfig();
  const [tempName, setTempName] = useState(systemName);
  const [error, setError] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleSave = () => {
    setSystemName(tempName);
    // Podríamos añadir una notificación de éxito aquí en el futuro
  };

  const handleLogoUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      setError(null);
      const reader = new FileReader();
      reader.onloadend = () => {
        const img = new Image();
        img.onload = () => {
          if (img.width === 1024 && img.height === 330) {
            setLogo(reader.result as string);
          } else {
            setError(`Las dimensiones de la imagen deben ser exactamente 1024x330 píxeles. La imagen actual es de ${img.width}x${img.height} píxeles.`);
          }
        };
        img.src = reader.result as string;
      };
      reader.readAsDataURL(file);
    }
    // Clear input so the same file can be selected again
    if (fileInputRef.current) {
      fileInputRef.current.value = '';
    }
  };

  return (
    <Box sx={{ maxWidth: 800, mx: 'auto', mt: 2 }}>
      <Typography variant="h5" sx={{ fontWeight: 700, mb: 3, color: 'text.primary' }}>
        Configuración del sistema
      </Typography>

      <Paper elevation={0} sx={{ p: 4, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
        <Typography variant="h6" sx={{ mb: 2, fontWeight: 600 }}>
          Apariencia
        </Typography>
        <Divider sx={{ mb: 4 }} />

        <Box sx={{ display: 'flex', flexDirection: { xs: 'column', md: 'column' }, gap: 4, alignItems: 'flex-start' }}>
          
          {/* Logo Section */}
          <Box sx={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-start', gap: 2, width: '100%' }}>
            <Typography variant="subtitle2" color="text.secondary">
              Logotipo del sistema (Se requiere una imagen de exactamente 1024x330 píxeles)
            </Typography>
            
            {error && (
              <Alert severity="error" sx={{ width: '100%' }}>
                {error}
              </Alert>
            )}

            <Box 
              sx={{ 
                width: '100%', 
                maxWidth: 512, // Half of 1024 for reasonable display
                height: 165, // Half of 330
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
                <Box component="img" src={logo} alt="Logo" sx={{ width: '100%', height: '100%', objectFit: 'contain' }} />
              ) : (
                <Typography color="text.secondary">Sin Logo</Typography>
              )}
            </Box>
            
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
                Cargar Logo (1024x330)
              </Button>
              {logo && (
                <Button 
                  variant="text" 
                  color="error" 
                  size="small"
                  onClick={() => setLogo(null)}
                >
                  Quitar Logo
                </Button>
              )}
            </Box>
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
                startIcon={<SaveIcon />}
                onClick={handleSave}
                disableElevation
                sx={{ px: 4, py: 1, borderRadius: '8px' }}
              >
                Guardar cambios
              </Button>
            </Box>
          </Box>

        </Box>
      </Paper>
    </Box>
  );
}

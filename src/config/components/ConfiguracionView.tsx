import { ReactNode, SyntheticEvent, useState } from 'react';
import { Box, Paper, Tab, Tabs, Typography } from '@mui/material';
import {
  Business as BusinessIcon,
  CloudSync as CloudSyncIcon,
  CloudUpload as CloudUploadIcon,
  Palette as PaletteIcon,
  Print as PrintIcon,
} from '@mui/icons-material';
import { SeccionApariencia } from './SeccionApariencia';
import { SeccionFiscal } from './SeccionFiscal';
import { SeccionMigracionLegacy } from './SeccionMigracionLegacy';
import { SeccionPerifericos } from './SeccionPerifericos';
import { SeccionSincronizacion } from './SeccionSincronizacion';

interface CustomTabPanelProps {
  children: ReactNode;
  index: number;
  value: number;
}

function CustomTabPanel({ children, index, value }: CustomTabPanelProps) {
  if (value !== index) return null;

  return (
    <Box role="tabpanel" id={`config-tabpanel-${index}`} aria-labelledby={`config-tab-${index}`} sx={{ mt: 2.5 }}>
      {children}
    </Box>
  );
}

function a11yProps(index: number) {
  return {
    id: `config-tab-${index}`,
    'aria-controls': `config-tabpanel-${index}`,
  };
}

export function ConfiguracionView() {
  const [value, setValue] = useState(0);

  const handleChange = (_event: SyntheticEvent, newValue: number) => {
    setValue(newValue);
  };

  return (
    <Box sx={{ width: '100%', mt: 2 }}>
      <Box sx={{ mb: 2.5 }}>
        <Typography variant="h5" sx={{ fontWeight: 800, color: 'text.primary' }}>
          Configuración del sistema
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 0.5 }}>
          Administra apariencia, periféricos, sincronización y datos fiscales desde secciones independientes.
        </Typography>
      </Box>

      <Paper
        elevation={0}
        sx={{
          borderRadius: 2,
          border: '1px solid',
          borderColor: 'divider',
          overflow: 'hidden',
          bgcolor: 'background.paper',
          position: 'sticky',
          top: 0,
          zIndex: 3,
        }}
      >
        <Tabs
          value={value}
          onChange={handleChange}
          variant="scrollable"
          scrollButtons="auto"
          allowScrollButtonsMobile
          sx={{
            minHeight: 54,
            px: 1,
            '& .MuiTab-root': {
              minHeight: 54,
              textTransform: 'none',
              fontWeight: 700,
              gap: 1,
              borderRadius: '10px',
              mx: 0.25,
              my: 0.75,
              color: 'text.secondary',
              '&.Mui-selected': {
                bgcolor: 'action.hover',
                color: 'primary.main',
              },
            },
            '& .MuiTabs-indicator': {
              height: 3,
              borderRadius: 999,
            },
          }}
        >
          <Tab icon={<PaletteIcon fontSize="small" />} iconPosition="start" label="Apariencia" {...a11yProps(0)} />
          <Tab icon={<PrintIcon fontSize="small" />} iconPosition="start" label="Periféricos e Impresión" {...a11yProps(1)} />
          <Tab icon={<CloudSyncIcon fontSize="small" />} iconPosition="start" label="Sincronización y Respaldos" {...a11yProps(2)} />
          <Tab icon={<BusinessIcon fontSize="small" />} iconPosition="start" label="Configuración Fiscal" {...a11yProps(3)} />
          <Tab icon={<CloudUploadIcon fontSize="small" />} iconPosition="start" label="Migración Legacy" {...a11yProps(4)} />
        </Tabs>
      </Paper>

      <CustomTabPanel value={value} index={0}>
        <SeccionApariencia />
      </CustomTabPanel>
      <CustomTabPanel value={value} index={1}>
        <SeccionPerifericos />
      </CustomTabPanel>
      <CustomTabPanel value={value} index={2}>
        <SeccionSincronizacion />
      </CustomTabPanel>
      <CustomTabPanel value={value} index={3}>
        <SeccionFiscal />
      </CustomTabPanel>
      <CustomTabPanel value={value} index={4}>
        <SeccionMigracionLegacy />
      </CustomTabPanel>
    </Box>
  );
}

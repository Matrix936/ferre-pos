import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  CircularProgress,
  Divider,
  TextField,
  Typography,
  useTheme,
} from '@mui/material';
import {
  ArrowForward as ArrowForwardIcon,
  Check as CheckIcon,
  CloudDone as CloudDoneIcon,
  CloudUpload as CloudUploadSyncIcon,
  CloudDownload as CloudDownloadIcon,
  Download as DownloadIcon,
  Info as InfoIcon,
  PowerSettingsNew as PowerSettingsNewIcon,
  Security as SecurityIcon,
  Storage as StorageIcon,
  Upload as UploadIcon,
  WarningAmber as WarningIcon,
} from '@mui/icons-material';

interface SupabaseConfig {
  url: string;
  anonKey: string;
  isConnected: boolean;
}

interface SyncUploadResult {
  totalRegistros: number;
  porTabla: Record<string, number>;
}

export function SincronizacionConfigView() {
  const theme = useTheme();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [inputUrl, setInputUrl] = useState('');
  const [inputKey, setInputKey] = useState('');
  const [currentUrl, setCurrentUrl] = useState('');
  const [isConnected, setIsConnected] = useState(false);
  const [isBusy, setIsBusy] = useState(false);
  const [isBusyBackup, setIsBusyBackup] = useState(false);
  const [backupAction, setBackupAction] = useState<'download' | 'apply' | 'upload' | 'uploadFull' | 'restore' | ''>('');
  const [errorMessage, setErrorMessage] = useState('');
  const [backupMessage, setBackupMessage] = useState('');

  const isDark = theme.palette.mode === 'dark';

  const loadStatus = async () => {
    try {
      const config = await invoke<SupabaseConfig>('get_supabase_config');
      setIsConnected(config.isConnected);
      setCurrentUrl(config.url || '');
      setInputUrl(config.isConnected ? '' : config.url || '');
      setInputKey('');
    } catch (error) {
      setErrorMessage(String(error));
    }
  };

  useEffect(() => {
    loadStatus();
  }, []);

  const handleConnect = async () => {
    if (!inputUrl.trim() || !inputKey.trim()) {
      setErrorMessage('Ambos campos son obligatorios.');
      return;
    }

    setIsBusy(true);
    setErrorMessage('');
    try {
      const config = await invoke<SupabaseConfig>('test_and_save_supabase_connect', {
        url: inputUrl.trim(),
        anonKey: inputKey.trim(),
      });
      setIsConnected(config.isConnected);
      setCurrentUrl(config.url);
      setInputUrl('');
      setInputKey('');
      window.alert('Conexión exitosa. El sistema ahora está listo para sincronizar.');
    } catch (error) {
      setErrorMessage(String(error));
    } finally {
      setIsBusy(false);
    }
  };

  const handleDisconnect = async () => {
    const confirmed = window.confirm('¿Seguro que deseas desvincular? La sincronización se detendrá.');
    if (!confirmed) return;

    setIsBusy(true);
    setErrorMessage('');
    try {
      await invoke('disconnect_supabase');
      setIsConnected(false);
      setCurrentUrl('');
      setInputUrl('');
      setInputKey('');
    } catch (error) {
      setErrorMessage(String(error));
    } finally {
      setIsBusy(false);
    }
  };

  const handleDownloadBackup = async () => {
    setIsBusyBackup(true);
    setBackupAction('download');
    setBackupMessage('');
    try {
      const path = await invoke<string>('crear_respaldo_local');
      setBackupMessage(`Respaldo guardado en: ${path}`);
      window.alert('Respaldo guardado correctamente.');
    } catch (error) {
      const message = `Error al generar respaldo: ${String(error)}`;
      setBackupMessage(message);
      window.alert(message);
    } finally {
      setIsBusyBackup(false);
      setBackupAction('');
    }
  };

  const handleApplyBackup = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = '';
    if (!file) {
      setBackupMessage('Selección cancelada.');
      return;
    }

    const confirmed = window.confirm('Aplicar un respaldo puede modificar datos locales. ¿Deseas continuar?');
    if (!confirmed) return;

    setIsBusyBackup(true);
    setBackupAction('apply');
    setBackupMessage('');
    try {
      const backupJson = await file.text();
      await invoke('aplicar_respaldo_local', { backupJson });
      setBackupMessage('Respaldo local aplicado correctamente.');
      window.alert('Respaldo aplicado correctamente.');
    } catch (error) {
      const message = `Error al aplicar respaldo: ${String(error)}`;
      setBackupMessage(message);
      window.alert(message);
    } finally {
      setIsBusyBackup(false);
      setBackupAction('');
    }
  };

  const handleRestoreFromCloud = async () => {
    const confirmed = window.confirm(
      'ADVERTENCIA:\n\nEsto descargará toda la información de la nube y sobrescribirá los datos locales que coincidan.\n\n¿Estás seguro de que quieres continuar?'
    );
    if (!confirmed) return;

    setIsBusyBackup(true);
    setBackupAction('restore');
    setBackupMessage('Descargando datos de la nube, por favor espera...');
    try {
      await invoke('sincronizar_desde_nube');
      setBackupMessage('Base de datos actualizada desde la nube correctamente.');
      window.alert('Restauración exitosa. Tus datos locales han sido actualizados.');
    } catch (error) {
      const message = `Error en la restauración: ${String(error)}`;
      setBackupMessage(message);
      window.alert(message);
    } finally {
      setIsBusyBackup(false);
      setBackupAction('');
    }
  };

  const handleUploadToCloud = async () => {
    setIsBusyBackup(true);
    setBackupAction('upload');
    setBackupMessage('Subiendo cambios locales a Supabase...');
    try {
      const result = await invoke<SyncUploadResult>('sincronizar_hacia_nube');
      const tableSummary = Object.entries(result.porTabla)
        .map(([table, count]) => `${table}: ${count}`)
        .join(', ');
      setBackupMessage(
        result.totalRegistros > 0
          ? `Sincronización completada. ${result.totalRegistros} registros subidos (${tableSummary}).`
          : 'No hay cambios locales pendientes por subir.'
      );
      window.alert('Sincronización hacia la nube completada.');
    } catch (error) {
      const message = `Error al subir cambios: ${String(error)}`;
      setBackupMessage(message);
      window.alert(message);
    } finally {
      setIsBusyBackup(false);
      setBackupAction('');
    }
  };

  const handleUploadFullToCloud = async () => {
    const confirmed = window.confirm(
      'ADVERTENCIA:\n\nEsto tomará la base de datos local como fuente principal y subirá todos sus registros a Supabase, sobrescribiendo los datos que coincidan en la nube.\n\nNo elimina registros que existan solo en Supabase. Para un espejo exacto, primero limpia la nube y después ejecuta esta acción.\n\n¿Estás seguro de que quieres continuar?'
    );
    if (!confirmed) return;

    setIsBusyBackup(true);
    setBackupAction('uploadFull');
    setBackupMessage('Subiendo base local completa a Supabase, por favor espera...');
    try {
      const result = await invoke<SyncUploadResult>('subir_base_local_completa_a_nube');
      const tableSummary = Object.entries(result.porTabla)
        .map(([table, count]) => `${table}: ${count}`)
        .join(', ');
      setBackupMessage(
        result.totalRegistros > 0
          ? `Base local subida correctamente. ${result.totalRegistros} registros enviados (${tableSummary}). No se purgaron registros existentes solo en Supabase.`
          : 'No se encontraron registros locales para subir.'
      );
      window.alert('Base local subida a Supabase correctamente.');
    } catch (error) {
      const message = `Error al subir base local completa: ${String(error)}`;
      setBackupMessage(message);
      window.alert(message);
    } finally {
      setIsBusyBackup(false);
      setBackupAction('');
    }
  };

  const actionCardSx = {
    width: '100%',
    justifyContent: 'flex-start',
    textAlign: 'left',
    p: 2,
    borderRadius: 2,
    borderColor: 'divider',
    bgcolor: isDark ? 'background.default' : '#f8f9fa',
    color: 'text.primary',
    '&:hover': {
      bgcolor: isDark ? 'action.hover' : '#f1f3f4',
      borderColor: 'divider',
    },
  };

  return (
    <Box sx={{ maxWidth: 900, mx: 'auto', mb: 3 }}>
      <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-end', mb: 3, gap: 2 }}>
        <Box>
          <Typography variant="h6" sx={{ fontWeight: 700 }}>
            Sincronización y respaldos
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Administra la conexión a Supabase y tus copias locales.
          </Typography>
        </Box>
      </Box>

      <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', lg: '7fr 5fr' }, gap: 3 }}>
        <Card elevation={0} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
          <CardContent sx={{ p: 4 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 4 }}>
              <Box
                sx={{
                  width: 50,
                  height: 50,
                  borderRadius: '50%',
                  bgcolor: 'primary.main',
                  color: 'primary.contrastText',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  opacity: 0.9,
                }}
              >
                <CloudDoneIcon />
              </Box>
              <Box>
                <Typography sx={{ fontWeight: 700 }}>Conexión a Supabase</Typography>
                <Typography variant="body2" color="text.secondary">
                  Sincronización remota de datos
                </Typography>
              </Box>
            </Box>

            {isConnected ? (
              <Box
                sx={{
                  textAlign: 'center',
                  py: 5,
                  px: 3,
                  borderRadius: 3,
                  border: '1px solid',
                  borderColor: 'divider',
                  bgcolor: isDark ? 'background.default' : '#f8f9fa',
                }}
              >
                <Box sx={{ position: 'relative', display: 'inline-flex', mb: 2 }}>
                  <SecurityIcon color="success" sx={{ fontSize: 72 }} />
                  <Box
                    sx={{
                      position: 'absolute',
                      right: -4,
                      top: 2,
                      width: 30,
                      height: 30,
                      borderRadius: '50%',
                      bgcolor: 'background.paper',
                      border: '1px solid',
                      borderColor: 'divider',
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                    }}
                  >
                    <CheckIcon color="success" fontSize="small" />
                  </Box>
                </Box>

                <Typography variant="h6" sx={{ fontWeight: 700, mb: 0.5 }}>
                  Todo sincronizado
                </Typography>
                <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                  Ferre-POS está conectado a la nube.
                </Typography>
                <Typography
                  variant="caption"
                  sx={{
                    display: 'inline-block',
                    maxWidth: '100%',
                    px: 1.5,
                    py: 0.75,
                    borderRadius: 1,
                    border: '1px solid',
                    borderColor: 'divider',
                    bgcolor: 'background.paper',
                    color: 'text.secondary',
                    fontFamily: 'monospace',
                    wordBreak: 'break-all',
                    mb: 3,
                  }}
                >
                  {currentUrl || 'Conectado'}
                </Typography>

                <Box>
                  <Button
                    color="error"
                    variant="outlined"
                    size="small"
                    startIcon={isBusy ? <CircularProgress size={16} /> : <PowerSettingsNewIcon />}
                    onClick={handleDisconnect}
                    disabled={isBusy}
                    sx={{ px: 3, borderRadius: 999 }}
                  >
                    Desconectar
                  </Button>
                </Box>
              </Box>
            ) : (
              <Box>
                <Alert severity="info" icon={<InfoIcon />} sx={{ mb: 3 }}>
                  Ve a <strong>Settings &gt; API</strong> en tu panel de Supabase para obtener las credenciales.
                </Alert>

                <TextField
                  fullWidth
                  label="Project URL"
                  value={inputUrl}
                  onChange={(event) => setInputUrl(event.target.value)}
                  disabled={isBusy}
                  sx={{ mb: 2 }}
                />
                <TextField
                  fullWidth
                  label="Anon Public Key"
                  type="password"
                  value={inputKey}
                  onChange={(event) => setInputKey(event.target.value)}
                  disabled={isBusy}
                  sx={{ mb: 3 }}
                />

                {errorMessage && (
                  <Alert severity="error" sx={{ mb: 2 }}>
                    {errorMessage}
                  </Alert>
                )}

                <Button
                  fullWidth
                  variant="contained"
                  size="large"
                  onClick={handleConnect}
                  disabled={isBusy}
                  endIcon={isBusy ? <CircularProgress size={18} color="inherit" /> : <ArrowForwardIcon />}
                  disableElevation
                  sx={{ py: 1.4, borderRadius: 999, fontWeight: 700 }}
                >
                  {isBusy ? 'Conectando...' : 'Conectar servicio'}
                </Button>
              </Box>
            )}
          </CardContent>
        </Card>

        <Card elevation={0} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
          <CardContent sx={{ p: 4 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 4 }}>
              <Box
                sx={{
                  width: 50,
                  height: 50,
                  borderRadius: '50%',
                  bgcolor: 'warning.light',
                  color: 'warning.contrastText',
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                <StorageIcon />
              </Box>
              <Box>
                <Typography sx={{ fontWeight: 700 }}>Copias de seguridad</Typography>
                <Typography variant="body2" color="text.secondary">
                  Gestión manual de datos
                </Typography>
              </Box>
            </Box>

            <Box sx={{ display: 'grid', gap: 2 }}>
              <Button
                variant="outlined"
                onClick={handleDownloadBackup}
                disabled={isBusyBackup}
                sx={actionCardSx}
                startIcon={backupAction === 'download' ? <CircularProgress size={16} /> : <DownloadIcon color="primary" />}
              >
                <Box>
                  <Typography variant="body2" sx={{ fontWeight: 700 }}>
                    {backupAction === 'download' ? 'Guardando respaldo...' : 'Guardar respaldo'}
                  </Typography>
                  <Typography variant="caption" color="text.secondary">
                    Exportar archivo .JSON local
                  </Typography>
                </Box>
              </Button>

              <input ref={fileInputRef} hidden type="file" accept="application/json,.json" onChange={handleApplyBackup} />
              <Button
                variant="outlined"
                onClick={() => fileInputRef.current?.click()}
                disabled={isBusyBackup}
                sx={actionCardSx}
                startIcon={backupAction === 'apply' ? <CircularProgress size={16} /> : <UploadIcon color="success" />}
              >
                <Box>
                  <Typography variant="body2" sx={{ fontWeight: 700 }}>
                    {backupAction === 'apply' ? 'Restaurando archivo...' : 'Restaurar archivo'}
                  </Typography>
                  <Typography variant="caption" color="text.secondary">
                    Importar desde .JSON
                  </Typography>
                </Box>
              </Button>

              {isConnected && (
                <>
                  <Divider sx={{ my: 0.5 }} />
                  <Button
                    variant="outlined"
                    onClick={handleUploadToCloud}
                    disabled={isBusyBackup}
                    sx={actionCardSx}
                    startIcon={backupAction === 'upload' ? <CircularProgress size={16} /> : <CloudUploadSyncIcon color="primary" />}
                  >
                    <Box>
                      <Typography variant="body2" sx={{ fontWeight: 700 }}>
                        {backupAction === 'upload' ? 'Subiendo cambios...' : 'Subir cambios'}
                      </Typography>
                      <Typography variant="caption" color="text.secondary">
                        Enviar pendientes locales a Supabase
                      </Typography>
                    </Box>
                  </Button>

                  <Alert
                    severity="warning"
                    icon={<WarningIcon />}
                    sx={{
                      border: 'none',
                      bgcolor: isDark ? 'rgba(255, 193, 7, 0.12)' : 'rgba(255, 193, 7, 0.16)',
                    }}
                  >
                    <Typography variant="caption" sx={{ display: 'block', fontWeight: 700, textTransform: 'uppercase', mb: 1 }}>
                      Zona de peligro
                    </Typography>
                    <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 2 }}>
                      Descargar la base de datos completa de la nube sobrescribirá los datos locales.
                    </Typography>
                    <Box sx={{ display: 'grid', gap: 1 }}>
                      <Button
                        fullWidth
                        color="warning"
                        variant="contained"
                        startIcon={backupAction === 'restore' ? <CircularProgress size={16} color="inherit" /> : <CloudDownloadIcon />}
                        onClick={handleRestoreFromCloud}
                        disabled={isBusyBackup}
                        disableElevation
                        sx={{ borderRadius: 999, fontWeight: 700 }}
                      >
                        {backupAction === 'restore' ? 'Restaurando nube...' : 'Restaurar nube'}
                      </Button>
                      <Button
                        fullWidth
                        color="error"
                        variant="outlined"
                        startIcon={backupAction === 'uploadFull' ? <CircularProgress size={16} color="inherit" /> : <CloudUploadSyncIcon />}
                        onClick={handleUploadFullToCloud}
                        disabled={isBusyBackup}
                        sx={{ borderRadius: 999, fontWeight: 700 }}
                      >
                        {backupAction === 'uploadFull' ? 'Subiendo local...' : 'Subir local a nube'}
                      </Button>
                    </Box>
                  </Alert>
                </>
              )}
            </Box>

            {backupMessage && (
              <Typography variant="body2" color="text.secondary" sx={{ mt: 2, textAlign: 'center', wordBreak: 'break-word' }}>
                {backupMessage}
              </Typography>
            )}
          </CardContent>
        </Card>
      </Box>
    </Box>
  );
}

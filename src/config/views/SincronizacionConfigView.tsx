import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Box,
  Button,
  Card,
  CardContent,
  Chip,
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
import { ConfirmActionDialog } from '../../shared/components/ConfirmActionDialog';
import { FeedbackSnackbar } from '../../shared/components/FeedbackSnackbar';
import { useFeedback } from '../../shared/hooks/useFeedback';
import { configActionButtonSx, configIconBadgeSx, configPanelSx, configSectionHeaderSx } from '../components/configSectionStyles';

interface SupabaseConfig {
  url: string;
  anonKey: string;
  isConnected: boolean;
}

interface SyncUploadResult {
  totalRegistros: number;
  porTabla: Record<string, number>;
}

interface SyncStatus {
  pendientes: number;
  ventasPendientes?: number;
  tablasPendientes?: Array<{
    tabla: string;
    pendientes: number;
  }>;
  ultimoIntentoAt?: string | null;
  ultimoExitoAt?: string | null;
  ultimoErrorAt?: string | null;
  ultimoError?: string | null;
}

interface PendingConfirm {
  title: string;
  message: string;
  confirmText: string;
  confirmColor?: 'primary' | 'error' | 'warning';
  onConfirm: () => Promise<void> | void;
}

const formatSyncDate = (value?: string | null) => {
  if (!value) return 'Sin registro';
  const parsed = new Date(value.replace(' ', 'T'));
  if (Number.isNaN(parsed.getTime())) return value;
  return parsed.toLocaleString('es-MX', { dateStyle: 'short', timeStyle: 'short' });
};

export function SincronizacionConfigView() {
  const theme = useTheme();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const [inputUrl, setInputUrl] = useState('');
  const [inputKey, setInputKey] = useState('');
  const [currentUrl, setCurrentUrl] = useState('');
  const [isConnected, setIsConnected] = useState(false);
  const [checkingStatus, setCheckingStatus] = useState(true);
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);
  const [isBusy, setIsBusy] = useState(false);
  const [isBusyBackup, setIsBusyBackup] = useState(false);
  const [backupAction, setBackupAction] = useState<'download' | 'apply' | 'upload' | 'uploadFull' | 'restore' | ''>('');
  const [errorMessage, setErrorMessage] = useState('');
  const [backupMessage, setBackupMessage] = useState('');
  const [pendingConfirm, setPendingConfirm] = useState<PendingConfirm | null>(null);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();

  const isDark = theme.palette.mode === 'dark';

  const loadStatus = async () => {
    setCheckingStatus(true);
    try {
      const config = await invoke<SupabaseConfig>('get_supabase_config');
      const status = await invoke<SyncStatus>('get_sync_status').catch(() => null);
      setIsConnected(config.isConnected);
      setSyncStatus(status);
      setCurrentUrl(config.url || '');
      setInputUrl(config.isConnected ? '' : config.url || '');
      setInputKey('');
    } catch (error) {
      setErrorMessage(String(error));
    } finally {
      setCheckingStatus(false);
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
      await loadStatus();
      setCurrentUrl(config.url);
      setInputUrl('');
      setInputKey('');
      showFeedback('Conexión exitosa. El sistema ahora está listo para sincronizar.');
    } catch (error) {
      setErrorMessage(String(error));
      showFeedback(String(error), 'error');
    } finally {
      setIsBusy(false);
    }
  };

  const executeDisconnect = async () => {
    setIsBusy(true);
    setErrorMessage('');
    try {
      await invoke('disconnect_supabase');
      setIsConnected(false);
      setSyncStatus(null);
      setCurrentUrl('');
      setInputUrl('');
      setInputKey('');
      showFeedback('Conexión a Supabase desactivada.', 'info');
    } catch (error) {
      setErrorMessage(String(error));
      showFeedback(String(error), 'error');
    } finally {
      setIsBusy(false);
    }
  };

  const handleDisconnect = () => {
    setPendingConfirm({
      title: 'Desconectar Supabase',
      message: 'La sincronización se detendrá hasta que vuelvas a conectar las credenciales.',
      confirmText: 'Desconectar',
      confirmColor: 'error',
      onConfirm: executeDisconnect,
    });
  };

  const handleDownloadBackup = async () => {
    setIsBusyBackup(true);
    setBackupAction('download');
    setBackupMessage('');
    try {
      const path = await invoke<string>('crear_respaldo_local');
      setBackupMessage(`Respaldo guardado en: ${path}`);
      showFeedback('Respaldo guardado correctamente.');
    } catch (error) {
      const message = `Error al generar respaldo: ${String(error)}`;
      setBackupMessage(message);
      showFeedback(message, 'error');
    } finally {
      setIsBusyBackup(false);
      setBackupAction('');
    }
  };

  const executeApplyBackup = async (file: File) => {
    setIsBusyBackup(true);
    setBackupAction('apply');
    setBackupMessage('');
    try {
      const backupJson = await file.text();
      await invoke('aplicar_respaldo_local', { backupJson });
      setBackupMessage('Respaldo local aplicado correctamente.');
      showFeedback('Respaldo aplicado correctamente.');
    } catch (error) {
      const message = `Error al aplicar respaldo: ${String(error)}`;
      setBackupMessage(message);
      showFeedback(message, 'error');
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

    setPendingConfirm({
      title: 'Restaurar respaldo local',
      message: 'Aplicar un respaldo puede modificar datos locales. Revisa que el archivo corresponda a este sistema antes de continuar.',
      confirmText: 'Restaurar',
      confirmColor: 'warning',
      onConfirm: () => executeApplyBackup(file),
    });
  };

  const executeRestoreFromCloud = async () => {
    setIsBusyBackup(true);
    setBackupAction('restore');
    setBackupMessage('Descargando datos de la nube, por favor espera...');
    try {
      await invoke('sincronizar_desde_nube');
      await loadStatus();
      setBackupMessage('Base de datos actualizada desde la nube correctamente.');
      showFeedback('Restauración exitosa. Tus datos locales han sido actualizados.');
    } catch (error) {
      const message = `Error en la restauración: ${String(error)}`;
      setBackupMessage(message);
      showFeedback(message, 'error');
    } finally {
      setIsBusyBackup(false);
      setBackupAction('');
    }
  };

  const handleRestoreFromCloud = () => {
    setPendingConfirm({
      title: 'Restaurar desde nube',
      message: 'Esto descargará la información de Supabase y sobrescribirá los datos locales que coincidan.',
      confirmText: 'Restaurar nube',
      confirmColor: 'warning',
      onConfirm: executeRestoreFromCloud,
    });
  };

  const handleUploadToCloud = async () => {
    setIsBusyBackup(true);
    setBackupAction('upload');
    setBackupMessage('Subiendo cambios locales a Supabase...');
    try {
      const result = await invoke<SyncUploadResult>('sincronizar_hacia_nube');
      await loadStatus();
      const tableSummary = Object.entries(result.porTabla)
        .map(([table, count]) => `${table}: ${count}`)
        .join(', ');
      setBackupMessage(
        result.totalRegistros > 0
          ? `Sincronización completada. ${result.totalRegistros} registros subidos (${tableSummary}).`
          : 'No hay cambios locales pendientes por subir.'
      );
      showFeedback('Sincronización hacia la nube completada.');
    } catch (error) {
      const message = `Error al subir cambios: ${String(error)}`;
      setBackupMessage(message);
      showFeedback(message, 'error');
    } finally {
      setIsBusyBackup(false);
      setBackupAction('');
    }
  };

  const executeUploadFullToCloud = async () => {
    setIsBusyBackup(true);
    setBackupAction('uploadFull');
    setBackupMessage('Subiendo base local completa a Supabase, por favor espera...');
    try {
      const result = await invoke<SyncUploadResult>('subir_base_local_completa_a_nube');
      await loadStatus();
      const tableSummary = Object.entries(result.porTabla)
        .map(([table, count]) => `${table}: ${count}`)
        .join(', ');
      setBackupMessage(
        result.totalRegistros > 0
          ? `Base local subida correctamente. ${result.totalRegistros} registros enviados (${tableSummary}). No se purgaron registros existentes solo en Supabase.`
          : 'No se encontraron registros locales para subir.'
      );
      showFeedback('Base local subida a Supabase correctamente.');
    } catch (error) {
      const message = `Error al subir base local completa: ${String(error)}`;
      setBackupMessage(message);
      showFeedback(message, 'error');
    } finally {
      setIsBusyBackup(false);
      setBackupAction('');
    }
  };

  const handleUploadFullToCloud = () => {
    setPendingConfirm({
      title: 'Subir base local completa',
      message: 'Esto tomará la base local como fuente principal y subirá sus registros a Supabase. No elimina registros que existan solo en la nube.',
      confirmText: 'Subir local',
      confirmColor: 'error',
      onConfirm: executeUploadFullToCloud,
    });
  };

  const actionCardSx = {
    width: '100%',
    justifyContent: 'flex-start',
    textAlign: 'left',
    p: 2,
    borderRadius: '12px',
    borderColor: 'divider',
    bgcolor: isDark ? 'background.default' : '#f8f9fa',
    color: 'text.primary',
    '&:hover': {
      bgcolor: isDark ? 'action.hover' : '#f1f3f4',
      borderColor: 'divider',
    },
  };

  return (
    <Box sx={{ width: '100%', mb: 3 }}>
      <Box sx={configSectionHeaderSx}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
          <Box sx={configIconBadgeSx}>
            <CloudDoneIcon fontSize="small" />
          </Box>
          <Box>
            <Typography variant="h6" sx={{ fontWeight: 800 }}>
              Sincronización y respaldos
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Administra la conexión a Supabase y tus copias locales.
            </Typography>
          </Box>
        </Box>
      </Box>

      <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', lg: '7fr 5fr' }, gap: 3 }}>
        <Card elevation={0} sx={configPanelSx}>
          <CardContent sx={{ p: 0 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 4 }}>
              <Box
                sx={{
                  ...configIconBadgeSx,
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

            {checkingStatus ? (
              <Box
                sx={{
                  textAlign: 'center',
                  py: 5,
                  px: 3,
                  borderRadius: 3,
                  border: '1px solid',
                  borderColor: 'divider',
                  bgcolor: isDark ? 'rgba(255,255,255,0.03)' : 'grey.50',
                }}
              >
                <CircularProgress size={34} sx={{ mb: 2 }} />
                <Typography sx={{ fontWeight: 700 }}>Verificando conexión...</Typography>
                <Typography variant="body2" color="text.secondary">
                  Consultando la configuración local de Supabase.
                </Typography>
              </Box>
            ) : isConnected ? (
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

                <Box sx={{ mb: 3 }}>
                  <Chip
                    size="small"
                    color={(syncStatus?.pendientes ?? 0) > 0 ? 'warning' : 'success'}
                    label={(syncStatus?.pendientes ?? 0) > 0
                      ? `${syncStatus?.pendientes ?? 0} pendientes por subir`
                      : 'Sin pendientes locales'}
                    sx={{ borderRadius: '8px', fontWeight: 800, mb: (syncStatus?.tablasPendientes?.length ?? 0) > 0 ? 1 : 0 }}
                  />
                  {(syncStatus?.tablasPendientes?.length ?? 0) > 0 && (
                    <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.75, justifyContent: 'center' }}>
                      {syncStatus?.tablasPendientes?.slice(0, 8).map((item) => (
                        <Chip
                          key={item.tabla}
                          size="small"
                          variant="outlined"
                          label={`${item.tabla}: ${item.pendientes}`}
                          sx={{ borderRadius: '8px', fontFamily: 'monospace', fontSize: '0.7rem' }}
                        />
                      ))}
                    </Box>
                  )}
                  {syncStatus?.ultimoError && (
                    <Alert severity="warning" sx={{ mt: 2, textAlign: 'left' }}>
                      <Typography variant="caption" sx={{ display: 'block', fontWeight: 800 }}>
                        Último error de sincronización: {formatSyncDate(syncStatus.ultimoErrorAt)}
                      </Typography>
                      <Typography variant="caption" sx={{ display: 'block', wordBreak: 'break-word' }}>
                        {syncStatus.ultimoError}
                      </Typography>
                    </Alert>
                  )}
                  <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1.5 }}>
                    Último intento: {formatSyncDate(syncStatus?.ultimoIntentoAt)} · Último éxito: {formatSyncDate(syncStatus?.ultimoExitoAt)}
                  </Typography>
                </Box>

                <Box>
                  <Button
                    color="error"
                    variant="outlined"
                    size="small"
                    startIcon={isBusy ? <CircularProgress size={16} /> : <PowerSettingsNewIcon />}
                    onClick={handleDisconnect}
                    disabled={isBusy}
                    sx={configActionButtonSx}
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
                  sx={{ ...configActionButtonSx, py: 1.4 }}
                >
                  {isBusy ? 'Conectando...' : 'Conectar servicio'}
                </Button>
              </Box>
            )}
          </CardContent>
        </Card>

        <Card elevation={0} sx={configPanelSx}>
          <CardContent sx={{ p: 0 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', gap: 2, mb: 4 }}>
              <Box
                sx={{
                  ...configIconBadgeSx,
                  color: 'warning.main',
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

              {!checkingStatus && isConnected && (
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
                        sx={configActionButtonSx}
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
                        sx={configActionButtonSx}
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
      <ConfirmActionDialog
        open={Boolean(pendingConfirm)}
        title={pendingConfirm?.title ?? ''}
        message={pendingConfirm?.message ?? ''}
        confirmText={pendingConfirm?.confirmText ?? 'Continuar'}
        confirmColor={pendingConfirm?.confirmColor ?? 'primary'}
        loading={isBusy || isBusyBackup}
        onCancel={() => setPendingConfirm(null)}
        onConfirm={async () => {
          if (!pendingConfirm) return;
          const action = pendingConfirm.onConfirm;
          setPendingConfirm(null);
          await action();
        }}
      />
      <FeedbackSnackbar message={feedbackMessage} severity={feedbackSeverity} onClose={closeFeedback} />
    </Box>
  );
}

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAuth } from "../context/AuthContext";
import { useConfig } from "../../config/context/ConfigContext";
import { User } from "../types";
import { InitialSetupDialog } from "./InitialSetupDialog";
import { 
  Alert,
  Box, 
  Button, 
  CircularProgress,
  Container, 
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  MenuItem,
  TextField, 
  Typography, 
  Paper,
  keyframes
} from "@mui/material";
import logoDefecto from "../../images/logoDefecto.png";

interface SucursalRemota {
  id: string;
  nombre: string;
  direccion: string;
  telefono: string;
  codigoPostal?: string;
}

const spin = keyframes`
  from { 
    transform: perspective(600px) rotateY(0deg); 
  }
  to { 
    transform: perspective(600px) rotateY(360deg); 
  }
`;

export function LoginForm() {
  const { login } = useAuth();
  const { logo, systemName, logoAnimationEnabled } = useConfig();
  const displayLogo = logo || logoDefecto;
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [cargando, setCargando] = useState(false);
  const [setupRequired, setSetupRequired] = useState(false);
  const [setupOpen, setSetupOpen] = useState(false);
  const [setupSuccess, setSetupSuccess] = useState(false);
  const [restoreSuccess, setRestoreSuccess] = useState(false);
  const [checkingSetup, setCheckingSetup] = useState(true);
  const [connectOpen, setConnectOpen] = useState(false);
  const [connectUrl, setConnectUrl] = useState("");
  const [connectKey, setConnectKey] = useState("");
  const [connectError, setConnectError] = useState("");
  const [connectBusy, setConnectBusy] = useState(false);
  const [remoteSucursales, setRemoteSucursales] = useState<SucursalRemota[]>([]);
  const [selectedSucursalId, setSelectedSucursalId] = useState("");
  const [connectStep, setConnectStep] = useState<"credentials" | "branch">("credentials");

  const checkInitialSetup = async () => {
    try {
      const required = await invoke<boolean>("necesita_configuracion_inicial");
      setSetupRequired(required);
    } catch (err) {
      console.error("Error al verificar configuración inicial:", err);
    } finally {
      setCheckingSetup(false);
    }
  };

  useEffect(() => {
    checkInitialSetup();
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError("");
    setCargando(true);

    try {
      // Llamamos al comando de Rust
      const usuario = await invoke<User>("iniciar_sesion", { 
        email, 
        clave: password 
      });
      
      // Si Rust responde bien, actualizamos el Contexto Global
      login(usuario);
    } catch (err) {
      setError(err as string);
    } finally {
      setCargando(false);
    }
  };

  const openConnectExisting = () => {
    setConnectOpen(true);
    setConnectStep("credentials");
    setConnectError("");
    setRemoteSucursales([]);
    setSelectedSucursalId("");
  };

  const handleConnectAndDownload = async () => {
    if (!connectUrl.trim() || !connectKey.trim()) {
      setConnectError("Project URL y Anon/Public Key son obligatorios.");
      return;
    }

    setConnectBusy(true);
    setConnectError("");
    try {
      await invoke("test_and_save_supabase_connect", {
        url: connectUrl.trim(),
        anonKey: connectKey.trim(),
      });
      await invoke("sincronizar_desde_nube");
      const sucursales = await invoke<SucursalRemota[]>("get_sucursales");
      setRemoteSucursales(sucursales);
      setSelectedSucursalId(sucursales[0]?.id || "");
      setConnectStep("branch");
    } catch (err) {
      setConnectError(String(err));
    } finally {
      setConnectBusy(false);
    }
  };

  const handleFinishConnectExisting = async () => {
    if (!selectedSucursalId) {
      setConnectError("Selecciona la sucursal que representará este equipo.");
      return;
    }

    localStorage.setItem("defaultSucursalId", selectedSucursalId);
    setConnectOpen(false);
    setSetupRequired(false);
    setRestoreSuccess(true);
    setSetupSuccess(false);
    setConnectUrl("");
    setConnectKey("");
    await checkInitialSetup();
  };

  return (
    <Box 
      sx={{ 
        minHeight: '100vh', 
        display: 'flex', 
        alignItems: 'center', 
        justifyContent: 'center',
        bgcolor: 'background.default'
      }}
    >
      <Container component="main" maxWidth="xs">
        <Paper 
          elevation={0} 
          sx={{ 
            p: { xs: 4, sm: 5 }, 
            display: 'flex', 
            flexDirection: 'column', 
            alignItems: 'center',
            width: '100%',
            borderRadius: 2,
            border: '1px solid',
            borderColor: 'divider',
            bgcolor: 'background.paper'
          }}
        >
          <Box 
            component="img" 
            src={displayLogo} 
            alt={systemName || "Ferre-POS"} 
            sx={{ 
              width: 'auto',
              maxWidth: '100%',
              maxHeight: 128,
              height: 'auto',
              objectFit: 'contain', 
              mb: 2,
              animation: logoAnimationEnabled ? `${spin} 15s linear infinite` : 'none',
            transformStyle: 'preserve-3d',
            backfaceVisibility: 'visible'
            }} 
          />
          <Typography component="h2" variant="body1" sx={{ color: '#5f6368', mb: 4 }}>
            Gestión de Inventario y Ventas
          </Typography>

          <Box component="form" onSubmit={handleSubmit} sx={{ mt: 1, width: '100%' }}>
            {error && (
              <Alert severity="error" sx={{ mb: 3 }}>
                {error}
              </Alert>
            )}

            {setupSuccess && (
              <Alert severity="success" sx={{ mb: 3 }}>
                Configuración inicial creada. Inicia sesión con el usuario registrado.
              </Alert>
            )}

            {restoreSuccess && (
              <Alert severity="success" sx={{ mb: 3 }}>
                Empresa conectada y datos descargados. Inicia sesión con un usuario de la base restaurada.
              </Alert>
            )}
            
            <TextField
              margin="normal"
              required
              fullWidth
              id="email"
              label="Correo electrónico"
              name="email"
              autoComplete="email"
              autoFocus
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              disabled={cargando}
              sx={{ mb: 2 }}
            />
            <TextField
              margin="normal"
              required
              fullWidth
              name="password"
              label="Contraseña"
              type="password"
              id="password"
              autoComplete="current-password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              disabled={cargando}
              sx={{ mb: 3 }}
            />
            
            <Box sx={{ display: 'flex', justifyContent: 'flex-end' }}>
              <Button
                type="submit"
                variant="contained"
                disableElevation
                sx={{ 
                  py: 1, 
                  px: 3,
                  fontWeight: 500
                }}
                disabled={cargando}
                startIcon={cargando ? <CircularProgress size={18} color="inherit" /> : undefined}
              >
                {cargando ? "Verificando..." : "Ingresar"}
              </Button>
            </Box>

            {!checkingSetup && setupRequired && (
              <>
                <Divider sx={{ my: 3 }} />
                <Box sx={{ display: "flex", flexDirection: "column", gap: 1.5 }}>
                  <Typography variant="body2" color="text.secondary">
                    ¿Primera vez en el sistema?
                  </Typography>
                  <Button
                    variant="outlined"
                    fullWidth
                    onClick={() => setSetupOpen(true)}
                    disabled={cargando}
                  >
                    Vamos a configurarlo
                  </Button>
                  <Button
                    variant="text"
                    fullWidth
                    onClick={openConnectExisting}
                    disabled={cargando}
                  >
                    Conectar con empresa existente
                  </Button>
                </Box>
              </>
            )}
          </Box>
        </Paper>
      </Container>

      <InitialSetupDialog
        open={setupOpen}
        onClose={() => setSetupOpen(false)}
        onComplete={(createdEmail) => {
          setSetupOpen(false);
          setSetupRequired(false);
          setSetupSuccess(true);
          setEmail(createdEmail);
          setPassword("");
        }}
      />

      <Dialog open={connectOpen} onClose={() => !connectBusy && setConnectOpen(false)} maxWidth="sm" fullWidth>
        <DialogTitle sx={{ fontWeight: 700 }}>
          Conectar con empresa existente
        </DialogTitle>
        <DialogContent sx={{ pt: 2, display: "flex", flexDirection: "column", gap: 2 }}>
          {connectError && <Alert severity="error">{connectError}</Alert>}

          {connectStep === "credentials" ? (
            <>
              <Typography variant="body2" color="text.secondary">
                Ingresa las credenciales de Supabase para descargar la información de la empresa en esta instalación.
              </Typography>
              <TextField
                label="Project URL"
                value={connectUrl}
                onChange={(event) => setConnectUrl(event.target.value)}
                disabled={connectBusy}
                fullWidth
                required
              />
              <TextField
                label="Anon/Public Key"
                value={connectKey}
                onChange={(event) => setConnectKey(event.target.value)}
                disabled={connectBusy}
                type="password"
                fullWidth
                required
              />
              <Alert severity="warning">
                La descarga inicial traerá la base de datos de Supabase a SQLite local. Úsalo solo en instalaciones nuevas o sin datos locales importantes.
              </Alert>
            </>
          ) : (
            <>
              <Typography variant="body2" color="text.secondary">
                Selecciona la sucursal física donde se usará este equipo.
              </Typography>
              <TextField
                select
                label="Sucursal de este equipo"
                value={selectedSucursalId}
                onChange={(event) => setSelectedSucursalId(event.target.value)}
                fullWidth
                required
              >
                {remoteSucursales.map((sucursal) => (
                  <MenuItem key={sucursal.id} value={sucursal.id}>
                    {sucursal.nombre}
                  </MenuItem>
                ))}
              </TextField>
            </>
          )}
        </DialogContent>
        <DialogActions sx={{ p: 3, pt: 1 }}>
          <Button onClick={() => setConnectOpen(false)} disabled={connectBusy}>
            Cancelar
          </Button>
          {connectStep === "credentials" ? (
            <Button
              variant="contained"
              onClick={handleConnectAndDownload}
              disabled={connectBusy}
              startIcon={connectBusy ? <CircularProgress size={18} color="inherit" /> : undefined}
            >
              {connectBusy ? "Descargando..." : "Conectar y descargar"}
            </Button>
          ) : (
            <Button variant="contained" onClick={handleFinishConnectExisting} disabled={!selectedSucursalId}>
              Usar esta sucursal
            </Button>
          )}
        </DialogActions>
      </Dialog>
    </Box>
  );
}

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
  Container, 
  Divider,
  TextField, 
  Typography, 
  Paper,
  keyframes
} from "@mui/material";
import logoDefecto from "../../images/logoDefecto.png";

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
  const { logo, systemName } = useConfig();
  const displayLogo = logo || logoDefecto;
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [cargando, setCargando] = useState(false);
  const [setupRequired, setSetupRequired] = useState(false);
  const [setupOpen, setSetupOpen] = useState(false);
  const [setupSuccess, setSetupSuccess] = useState(false);
  const [checkingSetup, setCheckingSetup] = useState(true);

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
              width: '100%', 
              maxWidth: 320, 
              height: 'auto', 
              objectFit: 'contain', 
              mb: 2,
              animation: `${spin} 15s linear infinite`,
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
              >
                {cargando ? "Verificando..." : "Siguiente"}
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
    </Box>
  );
}

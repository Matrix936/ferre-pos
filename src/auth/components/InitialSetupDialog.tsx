import { useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Alert,
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  Divider,
  MenuItem,
  Step,
  StepLabel,
  Stepper,
  TextField,
  Typography,
} from "@mui/material";
import { Business, CloudUpload, CorporateFare, PersonAdd, Save } from "@mui/icons-material";
import { Role } from "../types";
import { Usuario } from "../../usuarios/types";
import { Sucursal } from "../../sucursales/types";
import { useConfig } from "../../config/context/ConfigContext";

interface InitialSetupDialogProps {
  open: boolean;
  onClose: () => void;
  onComplete: (email: string) => void;
}

const steps = ["Empresa", "Sucursal", "Primer administrador"];

function buildFullName(nombres: string, apellidoPaterno: string, apellidoMaterno: string) {
  return [nombres, apellidoPaterno, apellidoMaterno]
    .map((value) => value.trim())
    .filter(Boolean)
    .join(" ");
}

export function InitialSetupDialog({ open, onClose, onComplete }: InitialSetupDialogProps) {
  const { systemName, logo, setSystemName, setLogo } = useConfig();
  const [activeStep, setActiveStep] = useState(0);
  const [empresaNombre, setEmpresaNombre] = useState(systemName === "Ferre-POS" ? "" : systemName);
  const [empresaLogo, setEmpresaLogo] = useState<string | null>(logo);
  const [sucursalId] = useState(() => crypto.randomUUID());
  const [sucursalNombre, setSucursalNombre] = useState("");
  const [direccion, setDireccion] = useState("");
  const [telefono, setTelefono] = useState("");
  const [nombres, setNombres] = useState("");
  const [apellidoPaterno, setApellidoPaterno] = useState("");
  const [apellidoMaterno, setApellidoMaterno] = useState("");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<Role>("SUPERADMIN");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const canContinueIdentity = Boolean(empresaNombre.trim());
  const canContinueCompany = Boolean(sucursalNombre.trim() && direccion.trim());
  const canCreateUser = Boolean(nombres.trim() && apellidoPaterno.trim() && email.trim() && password.trim());

  const sucursalPreview = useMemo<Sucursal>(() => ({
    id: sucursalId.trim(),
    nombre: sucursalNombre.trim(),
    direccion: direccion.trim(),
    telefono: telefono.trim(),
  }), [direccion, sucursalId, sucursalNombre, telefono]);

  const handleLogoUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];

    if (file) {
      setError("");
      const reader = new FileReader();
      reader.onloadend = () => {
        const img = new Image();
        img.onload = () => {
          if (img.width === 1024 && img.height === 330) {
            setEmpresaLogo(reader.result as string);
          } else {
            setError(`Las dimensiones de la imagen deben ser exactamente 1024x330 píxeles. La imagen actual es de ${img.width}x${img.height} píxeles.`);
          }
        };
        img.src = reader.result as string;
      };
      reader.readAsDataURL(file);
    }

    if (fileInputRef.current) {
      fileInputRef.current.value = "";
    }
  };

  const handleNext = () => {
    setError("");
    setActiveStep((step) => step + 1);
  };

  const handleBack = () => {
    setError("");
    setActiveStep((step) => step - 1);
  };

  const handleSubmit = async () => {
    setError("");
    setSaving(true);

    const usuario: Usuario = {
      id: crypto.randomUUID(),
      nombre: buildFullName(nombres, apellidoPaterno, apellidoMaterno),
      email: email.trim(),
      role,
      sucursalId: sucursalPreview.id,
    };

    try {
      await invoke("crear_configuracion_inicial", {
        sucursal: sucursalPreview,
        usuario,
        password,
      });
      setSystemName(empresaNombre.trim());
      setLogo(empresaLogo);
      onComplete(usuario.email);
      setActiveStep(0);
    } catch (err) {
      setError(err as string);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={open} onClose={saving ? undefined : onClose} maxWidth="sm" fullWidth>
      <DialogTitle sx={{ fontWeight: 700 }}>
        Configuración inicial
      </DialogTitle>
      <Divider />
      <DialogContent sx={{ pt: 3 }}>
        <Stepper activeStep={activeStep} sx={{ mb: 4 }}>
          {steps.map((label) => (
            <Step key={label}>
              <StepLabel>{label}</StepLabel>
            </Step>
          ))}
        </Stepper>

        {error && (
          <Alert severity="error" sx={{ mb: 3 }}>
            {error}
          </Alert>
        )}

        {activeStep === 0 && (
          <Box sx={{ display: "flex", flexDirection: "column", gap: 3 }}>
            <Box sx={{ display: "flex", alignItems: "center", gap: 1.5 }}>
              <CorporateFare color="primary" />
              <Box>
                <Typography variant="subtitle1" sx={{ fontWeight: 700 }}>
                  Identidad de la empresa
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Define el nombre que verá el sistema. El logo puede agregarse ahora o después.
                </Typography>
              </Box>
            </Box>

            <TextField
              label="Nombre de la empresa"
              value={empresaNombre}
              onChange={(event) => setEmpresaNombre(event.target.value)}
              required
              fullWidth
              autoFocus
            />

            <Box sx={{ display: "flex", flexDirection: "column", gap: 2 }}>
              <Typography variant="subtitle2" color="text.secondary">
                Logotipo del sistema opcional (1024x330 píxeles)
              </Typography>
              <Box
                sx={{
                  width: "100%",
                  maxWidth: 512,
                  height: 165,
                  bgcolor: "background.default",
                  border: "1px dashed",
                  borderColor: "divider",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  overflow: "hidden",
                  borderRadius: 1,
                }}
              >
                {empresaLogo ? (
                  <Box component="img" src={empresaLogo} alt="Logo" sx={{ width: "100%", height: "100%", objectFit: "contain" }} />
                ) : (
                  <Typography color="text.secondary">Sin logo</Typography>
                )}
              </Box>

              <input
                type="file"
                accept="image/*"
                hidden
                ref={fileInputRef}
                onChange={handleLogoUpload}
              />
              <Box sx={{ display: "flex", gap: 2 }}>
                <Button
                  variant="outlined"
                  startIcon={<CloudUpload />}
                  onClick={() => fileInputRef.current?.click()}
                >
                  Cargar logo
                </Button>
                {empresaLogo && (
                  <Button color="error" onClick={() => setEmpresaLogo(null)}>
                    Quitar logo
                  </Button>
                )}
              </Box>
            </Box>
          </Box>
        )}

        {activeStep === 1 && (
          <Box sx={{ display: "flex", flexDirection: "column", gap: 3 }}>
            <Box sx={{ display: "flex", alignItems: "center", gap: 1.5 }}>
              <Business color="primary" />
              <Box>
                <Typography variant="subtitle1" sx={{ fontWeight: 700 }}>
                  Ingresa la primer sucursal
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Esta será la base donde se asociará el primer administrador.
                </Typography>
              </Box>
            </Box>

            <TextField
              label="Nombre de la sucursal"
              value={sucursalNombre}
              onChange={(event) => setSucursalNombre(event.target.value)}
              required
              fullWidth
              autoFocus
            />
            <TextField
              label="Dirección"
              value={direccion}
              onChange={(event) => setDireccion(event.target.value)}
              required
              fullWidth
            />
            <TextField
              label="Teléfono"
              value={telefono}
              onChange={(event) => setTelefono(event.target.value)}
              fullWidth
            />
          </Box>
        )}

        {activeStep === 2 && (
          <Box sx={{ display: "flex", flexDirection: "column", gap: 3 }}>
            <Box sx={{ display: "flex", alignItems: "center", gap: 1.5 }}>
              <PersonAdd color="primary" />
              <Box>
                <Typography variant="subtitle1" sx={{ fontWeight: 700 }}>
                  Crea el primer usuario administrador
                </Typography>
                <Typography variant="body2" color="text.secondary">
                  Quedará asociado a {sucursalPreview.nombre}.
                </Typography>
              </Box>
            </Box>

            <TextField
              label="Nombres"
              value={nombres}
              onChange={(event) => setNombres(event.target.value)}
              required
              fullWidth
              autoFocus
            />
            <TextField
              label="Apellido paterno"
              value={apellidoPaterno}
              onChange={(event) => setApellidoPaterno(event.target.value)}
              required
              fullWidth
            />
            <TextField
              label="Apellido materno"
              value={apellidoMaterno}
              onChange={(event) => setApellidoMaterno(event.target.value)}
              fullWidth
            />
            <TextField
              label="Correo electrónico"
              type="email"
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              required
              fullWidth
            />
            <TextField
              label="Contraseña"
              type="password"
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              required
              fullWidth
            />
            <TextField
              select
              label="Rol inicial"
              value={role}
              onChange={(event) => setRole(event.target.value as Role)}
              required
              fullWidth
            >
              <MenuItem value="SUPERADMIN">Super Administrador</MenuItem>
              <MenuItem value="ADMIN">Administrador</MenuItem>
            </TextField>
          </Box>
        )}
      </DialogContent>
      <DialogActions sx={{ px: 3, pb: 3 }}>
        <Button onClick={onClose} disabled={saving}>
          Cancelar
        </Button>
        {activeStep > 0 && (
          <Button onClick={handleBack} disabled={saving}>
            Atrás
          </Button>
        )}
        {activeStep === 0 ? (
          <Button variant="contained" onClick={handleNext} disabled={!canContinueIdentity}>
            Continuar
          </Button>
        ) : activeStep === 1 ? (
          <Button variant="contained" onClick={handleNext} disabled={!canContinueCompany}>
            Continuar
          </Button>
        ) : (
          <Button
            variant="contained"
            startIcon={<Save />}
            onClick={handleSubmit}
            disabled={!canCreateUser || saving}
          >
            {saving ? "Creando..." : "Crear administrador"}
          </Button>
        )}
      </DialogActions>
    </Dialog>
  );
}

import { Button, Dialog, DialogActions, DialogContent, DialogContentText, DialogTitle } from '@mui/material';
import { AsyncButton } from './AsyncButton';
import { dialogActionsSx, dialogContentSx } from '../ui/patterns';

interface ConfirmActionDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmText?: string;
  loading?: boolean;
  confirmColor?: 'primary' | 'error' | 'warning';
  onCancel: () => void;
  onConfirm: () => void | Promise<void>;
}

export function ConfirmActionDialog({
  open,
  title,
  message,
  confirmText = 'Confirmar',
  loading = false,
  confirmColor = 'primary',
  onCancel,
  onConfirm,
}: ConfirmActionDialogProps) {
  return (
    <Dialog open={open} onClose={loading ? undefined : onCancel} maxWidth="xs" fullWidth>
      <DialogTitle sx={{ fontWeight: 700 }}>{title}</DialogTitle>
      <DialogContent sx={dialogContentSx}>
        <DialogContentText>{message}</DialogContentText>
      </DialogContent>
      <DialogActions sx={dialogActionsSx}>
        <Button onClick={onCancel} disabled={loading}>
          Cancelar
        </Button>
        <AsyncButton
          variant="contained"
          color={confirmColor}
          loading={loading}
          loadingText="Procesando..."
          onClick={onConfirm}
        >
          {confirmText}
        </AsyncButton>
      </DialogActions>
    </Dialog>
  );
}

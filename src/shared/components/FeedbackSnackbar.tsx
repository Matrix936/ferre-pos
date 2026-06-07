import { Alert, Snackbar } from '@mui/material';

export type FeedbackSeverity = 'success' | 'info' | 'warning' | 'error';

interface FeedbackSnackbarProps {
  message: string;
  severity?: FeedbackSeverity;
  onClose: () => void;
}

export function FeedbackSnackbar({ message, severity = 'success', onClose }: FeedbackSnackbarProps) {
  return (
    <Snackbar
      open={Boolean(message)}
      autoHideDuration={4200}
      onClose={onClose}
      anchorOrigin={{ vertical: 'bottom', horizontal: 'center' }}
      slotProps={{
        clickAwayListener: {
          onClickAway: (event) => {
            (event as MouseEvent & { defaultMuiPrevented?: boolean }).defaultMuiPrevented = true;
          },
        },
      }}
    >
      <Alert
        onClose={onClose}
        severity={severity}
        sx={{
          width: '100%',
          minWidth: { xs: 'calc(100vw - 32px)', sm: 380 },
          maxWidth: 560,
          backdropFilter: 'blur(10px)',
        }}
      >
        {message}
      </Alert>
    </Snackbar>
  );
}

import { ReactNode } from 'react';
import { Button, ButtonProps, CircularProgress } from '@mui/material';

interface AsyncButtonProps extends ButtonProps {
  loading?: boolean;
  loadingText?: ReactNode;
}

export function AsyncButton({ loading = false, loadingText, children, disabled, startIcon, ...props }: AsyncButtonProps) {
  return (
    <Button
      {...props}
      disabled={disabled || loading}
      startIcon={loading ? <CircularProgress size={18} color="inherit" /> : startIcon}
    >
      {loading ? loadingText ?? children : children}
    </Button>
  );
}

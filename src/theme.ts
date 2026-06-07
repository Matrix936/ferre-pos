import { createContext } from 'react';
import { alpha, createTheme } from '@mui/material/styles';
import type { PaletteMode } from '@mui/material';

export const ColorModeContext = createContext({
  toggleColorMode: () => {},
});

export const createAppTheme = (mode: PaletteMode) => createTheme({
  palette: {
    mode,
    primary: {
      main: '#1a73e8', // Google Blue
    },
    secondary: {
      main: '#ea4335', // Google Red
    },
    background: {
      default: mode === 'light' ? '#f8f9fa' : '#121212',
      paper: mode === 'light' ? '#ffffff' : '#1e1e1e',
    },
  },
  typography: {
    fontFamily: [
      'Roboto',
      '"Helvetica Neue"',
      'Arial',
      'sans-serif',
    ].join(','),
  },
  components: {
    MuiButton: {
      styleOverrides: {
        root: {
          textTransform: 'none', // Modern Google apps prefer no uppercase
          borderRadius: '8px',
          fontWeight: 500,
        },
      },
    },
    MuiCard: {
      styleOverrides: {
        root: {
          borderRadius: '12px',
          boxShadow: '0 1px 3px rgba(0,0,0,0.12), 0 1px 2px rgba(0,0,0,0.24)',
        },
      },
    },
    MuiTextField: {
      defaultProps: {
        variant: 'outlined',
      },
    },
    MuiAlert: {
      defaultProps: {
        variant: 'standard',
      },
      styleOverrides: {
        root: ({ theme }) => ({
          borderRadius: 12,
          alignItems: 'center',
          fontWeight: 600,
          boxShadow: 'none',
          '& .MuiAlert-icon': {
            opacity: 1,
          },
          '& .MuiAlert-message': {
            lineHeight: 1.45,
          },
          '&.MuiAlert-standardSuccess, &.MuiAlert-filledSuccess': {
            color: theme.palette.success.dark,
            backgroundColor: alpha(theme.palette.success.main, theme.palette.mode === 'dark' ? 0.18 : 0.1),
            border: `1px solid ${alpha(theme.palette.success.main, 0.26)}`,
            '& .MuiAlert-icon': { color: theme.palette.success.main },
          },
          '&.MuiAlert-standardInfo, &.MuiAlert-filledInfo': {
            color: theme.palette.info.dark,
            backgroundColor: alpha(theme.palette.info.main, theme.palette.mode === 'dark' ? 0.18 : 0.1),
            border: `1px solid ${alpha(theme.palette.info.main, 0.26)}`,
            '& .MuiAlert-icon': { color: theme.palette.info.main },
          },
          '&.MuiAlert-standardWarning, &.MuiAlert-filledWarning': {
            color: theme.palette.warning.dark,
            backgroundColor: alpha(theme.palette.warning.main, theme.palette.mode === 'dark' ? 0.2 : 0.12),
            border: `1px solid ${alpha(theme.palette.warning.main, 0.3)}`,
            '& .MuiAlert-icon': { color: theme.palette.warning.main },
          },
          '&.MuiAlert-standardError, &.MuiAlert-filledError': {
            color: theme.palette.error.dark,
            backgroundColor: alpha(theme.palette.error.main, theme.palette.mode === 'dark' ? 0.18 : 0.1),
            border: `1px solid ${alpha(theme.palette.error.main, 0.26)}`,
            '& .MuiAlert-icon': { color: theme.palette.error.main },
          },
        }),
      },
    },
    MuiTableContainer: {
      styleOverrides: {
        root: {
          width: '100%',
        },
      },
    },
    MuiTableCell: {
      defaultProps: {
        align: 'center',
      },
      styleOverrides: {
        root: {
          verticalAlign: 'middle',
        },
        head: {
          fontWeight: 700,
          whiteSpace: 'nowrap',
        },
      },
    },
  },
});

const theme = createAppTheme('light');

export default theme;

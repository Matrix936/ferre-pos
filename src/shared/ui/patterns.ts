export const dialogContentSx = {
  '&&': { pt: 2.5 },
  display: 'flex',
  flexDirection: 'column',
  gap: 2,
} as const;

export const dialogActionsSx = {
  p: 2,
} as const;

export const panelSx = {
  p: 2,
  borderRadius: 2,
  border: '1px solid',
  borderColor: 'divider',
} as const;

export const tablePanelSx = {
  borderRadius: 2,
  border: '1px solid',
  borderColor: 'divider',
  overflow: 'hidden',
} as const;

export const chipCompactSx = {
  height: 22,
  borderRadius: '6px',
  fontWeight: 700,
} as const;

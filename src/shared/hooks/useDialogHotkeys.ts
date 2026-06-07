import { useEffect } from 'react';

interface UseDialogHotkeysOptions {
  open: boolean;
  disabled?: boolean;
  cancelDisabled?: boolean;
  onConfirm?: () => void;
  onCancel?: () => void;
}

function isTypingInComplexInput(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  const tag = target.tagName;
  const role = target.getAttribute('role');
  const expanded = target.getAttribute('aria-expanded');
  return tag === 'TEXTAREA' || role === 'combobox' || expanded === 'true';
}

export function useDialogHotkeys({ open, disabled = false, cancelDisabled = false, onConfirm, onCancel }: UseDialogHotkeysOptions) {
  useEffect(() => {
    if (!open) return undefined;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      if (event.key === 'Escape') {
        event.preventDefault();
        if (!cancelDisabled) onCancel?.();
        return;
      }
      if (event.key !== 'Enter' || disabled || !onConfirm) return;
      if (isTypingInComplexInput(event.target)) return;
      event.preventDefault();
      onConfirm();
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [cancelDisabled, disabled, onCancel, onConfirm, open]);
}

import { useEffect, useRef } from 'react';

interface BarcodeScannerOptions {
  enabled?: boolean;
  maxDelayMs?: number;
  minLength?: number;
}

export function useBarcodeScanner(
  onScan: (code: string) => void | Promise<void>,
  { enabled = true, maxDelayMs = 50, minLength = 3 }: BarcodeScannerOptions = {},
) {
  const callbackRef = useRef(onScan);

  useEffect(() => {
    callbackRef.current = onScan;
  }, [onScan]);

  useEffect(() => {
    if (!enabled) return undefined;

    let buffer = '';
    let lastKeyAt = 0;

    const clearBuffer = () => {
      buffer = '';
      lastKeyAt = 0;
    };

    const handleKeyDown = (event: KeyboardEvent) => {
      const now = Date.now();

      if (now - lastKeyAt > maxDelayMs) {
        buffer = '';
      }

      if (event.key === 'Enter') {
        const code = buffer.trim();
        clearBuffer();

        if (code.length >= minLength) {
          event.preventDefault();
          event.stopPropagation();
          void callbackRef.current(code);
        }
        return;
      }

      if (event.key.length !== 1 || event.ctrlKey || event.altKey || event.metaKey) {
        return;
      }

      buffer += event.key;
      lastKeyAt = now;
    };

    window.addEventListener('keydown', handleKeyDown, true);

    return () => {
      window.removeEventListener('keydown', handleKeyDown, true);
      clearBuffer();
    };
  }, [enabled, maxDelayMs, minLength]);
}

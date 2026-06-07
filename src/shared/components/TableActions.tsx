import { Button, Stack } from '@mui/material';
import { ContentCopy, FileDownload, PictureAsPdf } from '@mui/icons-material';
import { useState } from 'react';
import { FeedbackSnackbar } from './FeedbackSnackbar';
import { useFeedback } from '../hooks/useFeedback';

type ExportColumn<T> = {
  key: keyof T;
  label: string;
};

interface TableActionsProps<T extends Record<string, unknown>> {
  filename: string;
  rows: T[];
  columns: ExportColumn<T>[];
}

function toPlainRows<T extends Record<string, unknown>>(rows: T[], columns: ExportColumn<T>[]) {
  return rows.map((row) => {
    const plain: Record<string, string | number> = {};
    for (const column of columns) {
      const value = row[column.key];
      plain[column.label] = typeof value === 'number' ? value : String(value ?? '');
    }
    return plain;
  });
}

export function TableActions<T extends Record<string, unknown>>({ filename, rows, columns }: TableActionsProps<T>) {
  const [exportingExcel, setExportingExcel] = useState(false);
  const [exportingPdf, setExportingPdf] = useState(false);
  const { feedbackMessage, feedbackSeverity, showFeedback, closeFeedback } = useFeedback();

  const handleCopy = async () => {
    const header = columns.map((column) => column.label).join('\t');
    const body = rows
      .map((row) => columns.map((column) => String(row[column.key] ?? '')).join('\t'))
      .join('\n');
    try {
      await navigator.clipboard.writeText(`${header}\n${body}`);
      showFeedback('Tabla copiada correctamente.');
    } catch {
      showFeedback('No se pudo copiar la tabla al portapapeles.', 'error');
    }
  };

  const saveWithPicker = async (defaultName: string, mimeType: string, content: Blob | Uint8Array) => {
    const win = window as Window & {
      showSaveFilePicker?: (options: {
        suggestedName: string;
        types: Array<{ description: string; accept: Record<string, string[]> }>;
      }) => Promise<{
        createWritable: () => Promise<{ write: (data: Blob | Uint8Array) => Promise<void>; close: () => Promise<void> }>;
      }>;
    };

    if (win.showSaveFilePicker) {
      const handle = await win.showSaveFilePicker({
        suggestedName: defaultName,
        types: [{ description: 'Archivo', accept: { [mimeType]: [`.${defaultName.split('.').pop()}`] } }],
      });
      const writable = await handle.createWritable();
      await writable.write(content);
      await writable.close();
      return true;
    }

    return false;
  };

  const handleExcel = async () => {
    setExportingExcel(true);
    try {
      const XLSX = await import('xlsx');
      const worksheet = XLSX.utils.json_to_sheet(toPlainRows(rows, columns));
      const workbook = XLSX.utils.book_new();
      XLSX.utils.book_append_sheet(workbook, worksheet, 'Datos');
      const fileBuffer = XLSX.write(workbook, { bookType: 'xlsx', type: 'array' });
      const saved = await saveWithPicker(
        `${filename}.xlsx`,
        'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
        new Uint8Array(fileBuffer),
      );
      if (!saved) {
        XLSX.writeFile(workbook, `${filename}.xlsx`);
      }
    } catch {
      showFeedback('No se pudo exportar a Excel.', 'error');
    } finally {
      setExportingExcel(false);
    }
  };

  const handlePdf = async () => {
    setExportingPdf(true);
    try {
      const [{ default: jsPDF }, { default: autoTable }] = await Promise.all([
        import('jspdf'),
        import('jspdf-autotable'),
      ]);
      const doc = new jsPDF();
      autoTable(doc, {
        head: [columns.map((column) => column.label)],
        body: rows.map((row) => columns.map((column) => String(row[column.key] ?? ''))),
        styles: { fontSize: 9 },
        margin: { top: 16 },
      });
      const pdfBlob = doc.output('blob');
      const saved = await saveWithPicker(`${filename}.pdf`, 'application/pdf', pdfBlob);
      if (!saved) {
        doc.save(`${filename}.pdf`);
      }
    } catch {
      showFeedback('No se pudo exportar a PDF.', 'error');
    } finally {
      setExportingPdf(false);
    }
  };

  return (
    <>
      <Stack direction={{ xs: 'column', sm: 'row' }} spacing={1.5}>
        <Button variant="outlined" size="small" startIcon={<ContentCopy />} onClick={handleCopy}>
          Copiar tabla
        </Button>
        <Button variant="outlined" size="small" startIcon={<FileDownload />} onClick={handleExcel} disabled={exportingExcel}>
          {exportingExcel ? 'Exportando...' : 'Exportar Excel'}
        </Button>
        <Button variant="outlined" size="small" startIcon={<PictureAsPdf />} onClick={handlePdf} disabled={exportingPdf}>
          {exportingPdf ? 'Exportando...' : 'Exportar PDF'}
        </Button>
      </Stack>
      <FeedbackSnackbar message={feedbackMessage} severity={feedbackSeverity} onClose={closeFeedback} />
    </>
  );
}

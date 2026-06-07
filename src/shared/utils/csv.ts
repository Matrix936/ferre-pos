export function downloadCsv(filename: string, rows: Record<string, string | number | null | undefined>[]) {
  if (rows.length === 0) return;

  const headers = Object.keys(rows[0]);
  const escapeCell = (value: string | number | null | undefined) => {
    const text = value === null || value === undefined ? '' : String(value);
    return `"${text.replace(/"/g, '""')}"`;
  };
  const csv = [
    headers.map(escapeCell).join(','),
    ...rows.map((row) => headers.map((header) => escapeCell(row[header])).join(',')),
  ].join('\n');

  const blob = new Blob([`\uFEFF${csv}`], { type: 'text/csv;charset=utf-8;' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename.endsWith('.csv') ? filename : `${filename}.csv`;
  link.click();
  URL.revokeObjectURL(url);
}

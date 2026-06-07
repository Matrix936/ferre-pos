import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import {
  Alert,
  Autocomplete,
  Box,
  Button,
  Card,
  CardContent,
  Chip,
  CircularProgress,
  Divider,
  MenuItem,
  Paper,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  TextField,
  Typography,
} from '@mui/material';
import {
  CloudUpload as UploadIcon,
  FileOpen as FileOpenIcon,
  Save as ImportIcon,
} from '@mui/icons-material';
import { useAuth } from '../../auth/context/AuthContext';
import { useCatalogos } from '../../catalogos/context/CatalogosContext';
import { ConfirmActionDialog } from '../../shared/components/ConfirmActionDialog';

interface LegacyArticuloImportRow {
  id?: number;
  clave?: string;
  descripcionArticulo?: string;
  unidad?: string;
  codigoBarra?: string;
  existenciaStock?: number;
  caducidad?: string;
  provedor?: number;
  proveedorNombre?: string;
  categoria?: string;
  marca?: number;
  marcaNombre?: string;
  fotos?: string;
  descripcionCatalogo?: string;
  precioCompra?: number;
  precioVenta?: number;
  precio1?: number;
  precio2?: number;
  precio3?: number;
  precio4?: number;
  mayoreoApartir?: number;
  cantMinStock?: number;
  aGranel?: string;
  noEnCatalogo?: string;
  ventasNegativas?: string;
  createdAt?: string;
  updatedAt?: string;
}

interface ImportarArticulosLegacyResult {
  totalLeidos: number;
  productosUpsertados: number;
  inventarioUpsertado: number;
  catalogosActualizados: number;
}

interface ImportarDatosUniversalResult {
  destino: string;
  totalLeidos: number;
  registrosUpsertados: number;
  omitidos: number;
}

interface ImportarCsvProductosMapeadoResult {
  totalLeidos: number;
  productosUpsertados: number;
  inventarioUpsertado: number;
  filasOmitidas: number;
  errores: CsvImportIssue[];
}

interface CsvImportIssue {
  fila: number;
  motivo: string;
  codigo: string;
  descripcion: string;
}

interface AnalizarCsvImportacionResult {
  totalFilas: number;
  uniqueValues: Record<string, string[]>;
  previewRows: Array<Record<string, string>>;
  warnings: string[];
}

interface CsvArchivoSeleccionado {
  filePath: string;
  fileName: string;
  headers: string[];
  delimiter: CsvDelimiter;
}

type ImportProfile = 'CSV_PRODUCTOS_MAPEADO' | 'ARTICULOS' | 'CLIENTES' | 'PROVEEDORES' | 'MARCAS' | 'CATEGORIAS' | 'UNIDADES';

const importProfiles: Array<{ value: ImportProfile; label: string; helper: string }> = [
  { value: 'CSV_PRODUCTOS_MAPEADO', label: 'CSV mapeado: Productos + Inventario', helper: 'Mapea encabezados y homologa catálogos antes de importar por streaming.' },
];

type CsvFieldKey =
  | 'codigo'
  | 'descripcion'
  | 'precioVenta'
  | 'precioCosto'
  | 'stock'
  | 'stockMinimo'
  | 'codigoBarras'
  | 'proveedorId'
  | 'marca'
  | 'categoria'
  | 'unidad';

const csvFields: Array<{ key: CsvFieldKey; label: string; required?: boolean; relation?: 'proveedores' | 'marcas' | 'categorias' | 'unidades' }> = [
  { key: 'codigo', label: 'Código / Clave interna', required: true },
  { key: 'descripcion', label: 'Descripción', required: true },
  { key: 'precioVenta', label: 'Precio de venta', required: true },
  { key: 'precioCosto', label: 'Costo' },
  { key: 'stock', label: 'Stock actual' },
  { key: 'stockMinimo', label: 'Stock mínimo' },
  { key: 'codigoBarras', label: 'Código de barras' },
  { key: 'proveedorId', label: 'Proveedor', required: true, relation: 'proveedores' },
  { key: 'marca', label: 'Marca', relation: 'marcas' },
  { key: 'categoria', label: 'Categoría', relation: 'categorias' },
  { key: 'unidad', label: 'Unidad', relation: 'unidades' },
];

const csvRequiredFieldKeys = csvFields.filter((field) => field.required).map((field) => field.key);

const csvFieldAliases: Record<CsvFieldKey, string[]> = {
  codigo: ['codigo', 'clave', 'codigo_proveedor', 'codigo interno', 'sku', 'id_articulo'],
  descripcion: ['descripcion_articulo', 'descripcionarticulo', 'descripcion', 'producto', 'nombre', 'articulo'],
  precioVenta: ['precio_venta', 'precioventa', 'precio venta', 'venta', 'precio_1', 'precio1'],
  precioCosto: ['precio_compra', 'preciocompra', 'precio costo', 'costo', 'precio_costo', 'compra'],
  stock: ['existencia_stock', 'existenciastock', 'existencia', 'stock', 'cantidad'],
  stockMinimo: ['cant_min_stock', 'cantminstock', 'stock_minimo', 'stock minimo', 'minimo'],
  codigoBarras: ['codigo_barra', 'codigobarra', 'codigo barras', 'codigo_barras', 'barcode'],
  proveedorId: ['provedor', 'proveedor', 'proveedor_id', 'id_proveedor', 'supplier'],
  marca: ['marca', 'marca_id', 'id_marca', 'nombre_marca'],
  categoria: ['categoria', 'categoria_id', 'id_categoria', 'departamento', 'familia'],
  unidad: ['unidad', 'unidad_medida', 'unidadmedida', 'presentacion'],
};

type CsvDelimiter = 'AUTO' | 'COMA' | 'PUNTO_COMA' | 'TAB';

const csvDelimiters: Array<{ value: CsvDelimiter; label: string }> = [
  { value: 'AUTO', label: 'Detectar automáticamente' },
  { value: 'COMA', label: 'Coma (,)' },
  { value: 'PUNTO_COMA', label: 'Punto y coma (;)' },
  { value: 'TAB', label: 'Tabulador' },
];

function normalizeHeader(value: string): string {
  return value
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]/g, '');
}

function buildAutoColumnMap(headers: string[]): Partial<Record<CsvFieldKey, number>> {
  const normalizedHeaders = headers.map(normalizeHeader);
  const map: Partial<Record<CsvFieldKey, number>> = {};
  const used = new Set<number>();
  csvFields.forEach((field) => {
    const aliases = csvFieldAliases[field.key].map(normalizeHeader);
    const exactIndex = normalizedHeaders.findIndex((header, index) => !used.has(index) && aliases.includes(header));
    if (exactIndex >= 0) {
      map[field.key] = exactIndex;
      used.add(exactIndex);
    }
  });
  csvFields.forEach((field) => {
    if (map[field.key] !== undefined) return;
    const aliases = csvFieldAliases[field.key].map(normalizeHeader);
    const partialIndex = normalizedHeaders.findIndex((header, index) =>
      !used.has(index) && aliases.some((alias) => header.includes(alias) || alias.includes(header))
    );
    if (partialIndex >= 0) {
      map[field.key] = partialIndex;
      used.add(partialIndex);
    }
  });
  return map;
}

function normalizeMatchValue(value: string): string {
  return normalizeHeader(value);
}

function lastNumber(value: string): string | undefined {
  return value.match(/(\d+)\s*$/)?.[1];
}

function pickField(row: Record<string, unknown>, aliases: string[]): unknown {
  const normalizedMap = new Map<string, unknown>();
  Object.entries(row).forEach(([key, value]) => {
    normalizedMap.set(normalizeHeader(key), value);
  });
  for (const alias of aliases) {
    const value = normalizedMap.get(normalizeHeader(alias));
    if (value !== undefined && value !== null && String(value).trim() !== '') return value;
  }
  return undefined;
}

function asText(value: unknown): string | undefined {
  if (value === undefined || value === null) return undefined;
  const text = String(value).trim();
  return text === '' ? undefined : text;
}

function countDuplicated(values: Array<string | number | undefined>): number {
  const seen = new Set<string>();
  const duplicated = new Set<string>();
  values.forEach((value) => {
    const key = String(value ?? '').trim();
    if (!key) return;
    if (seen.has(key)) duplicated.add(key);
    seen.add(key);
  });
  return duplicated.size;
}

function getGenericRequiredValue(profile: ImportProfile, row: Record<string, unknown>): string {
  if (profile === 'CLIENTES') return asText(pickField(row, ['nombre', 'cliente', 'razon_social', 'razon social'])) ?? '';
  if (profile === 'PROVEEDORES') return asText(pickField(row, ['nombre', 'proveedor', 'razon_social', 'razon social'])) ?? '';
  if (profile === 'MARCAS') return asText(pickField(row, ['nombre', 'marca', 'descripcion'])) ?? '';
  if (profile === 'CATEGORIAS') return asText(pickField(row, ['nombre', 'categoria', 'descripcion'])) ?? '';
  if (profile === 'UNIDADES') return asText(pickField(row, ['nombre', 'unidad', 'descripcion'])) ?? '';
  return '';
}

export function ImportadorLegacyArticulosView() {
  const { user } = useAuth();
  const { sucursales, proveedores, marcas, categorias, unidades, refreshCatalogos } = useCatalogos();
  const [profile] = useState<ImportProfile>('CSV_PRODUCTOS_MAPEADO');
  const [rawRows, setRawRows] = useState<Record<string, unknown>[]>([]);
  const [rows, setRows] = useState<LegacyArticuloImportRow[]>([]);
  const [fileName, setFileName] = useState('');
  const [csvFilePath, setCsvFilePath] = useState('');
  const [csvDelimiter, setCsvDelimiter] = useState<CsvDelimiter>('AUTO');
  const [csvHeaders, setCsvHeaders] = useState<string[]>([]);
  const [csvColumnMap, setCsvColumnMap] = useState<Partial<Record<CsvFieldKey, number>>>({});
  const [csvUniqueValues, setCsvUniqueValues] = useState<Record<CsvFieldKey, string[]>>({
    codigo: [],
    descripcion: [],
    precioVenta: [],
    precioCosto: [],
    stock: [],
    stockMinimo: [],
    codigoBarras: [],
    proveedorId: [],
    marca: [],
    categoria: [],
    unidad: [],
  });
  const [csvRelationMap, setCsvRelationMap] = useState<Record<string, Record<string, string>>>({});
  const [csvAnalysis, setCsvAnalysis] = useState<AnalizarCsvImportacionResult | null>(null);
  const [csvImportErrors, setCsvImportErrors] = useState<CsvImportIssue[]>([]);
  const [sucursalId, setSucursalId] = useState(user?.sucursalId ?? '');
  const [proveedorDefaultId] = useState('PROV-SIN-ASIGNAR');
  const [isParsing, setIsParsing] = useState(false);
  const [isImporting, setIsImporting] = useState(false);
  const [confirmImportOpen, setConfirmImportOpen] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');

  const profileConfig = useMemo(() => importProfiles.find((item) => item.value === profile), [profile]);
  const previewRows = useMemo(() => rows.slice(0, 20), [rows]);
  const genericPreviewRows = useMemo(() => rawRows.slice(0, 20), [rawRows]);
  const genericColumns = useMemo(() => Object.keys(rawRows[0] ?? {}).slice(0, 7), [rawRows]);
  const catalogOptions = useMemo(
    () => ({
      proveedores: proveedores.map((item) => ({ id: item.id, label: item.nombre, matchIds: [item.id, item.nombre] })),
      marcas: marcas.map((item) => ({ id: item.nombre, label: item.nombre, matchIds: [item.id, item.nombre] })),
      categorias: categorias.map((item) => ({ id: item.nombre, label: item.nombre, matchIds: [item.id, item.nombre] })),
      unidades: unidades.map((item) => ({
        id: item.nombre,
        label: `${item.nombre}${item.claveSat ? ` (${item.claveSat})` : ''}`,
        matchIds: [item.id, item.nombre, item.claveSat],
      })),
    }),
    [categorias, marcas, proveedores, unidades]
  );
  const resolveAutoRelationValue = (field: CsvFieldKey, legacyValue: string): string => {
    const relation = csvFields.find((item) => item.key === field)?.relation;
    if (!relation) return '';
    const options = catalogOptions[relation];
    const normalizedLegacy = normalizeMatchValue(legacyValue);
    const legacyNumber = lastNumber(legacyValue);
    const match = options.find((option) => {
      const candidates = option.matchIds.filter(Boolean);
      return candidates.some((candidate) => {
        const normalizedCandidate = normalizeMatchValue(candidate);
        if (normalizedCandidate === normalizedLegacy) return true;
        const candidateNumber = lastNumber(candidate);
        return Boolean(legacyNumber && candidateNumber && legacyNumber === candidateNumber);
      });
    });
    return match?.id ?? '';
  };
  const csvMappedRequired = useMemo(
    () => csvRequiredFieldKeys.every((key) => csvColumnMap[key] !== undefined) && sucursalId.trim().length > 0 && csvFilePath.trim().length > 0,
    [csvColumnMap, csvFilePath, sucursalId]
  );
  const csvRelationComplete = useMemo(() => {
    return csvFields
      .filter((field) => field.relation && csvColumnMap[field.key] !== undefined)
      .every((field) => csvUniqueValues[field.key].every((value) => Boolean(csvRelationMap[field.key]?.[value])));
  }, [csvColumnMap, csvRelationMap, csvUniqueValues]);
  const csvUnresolvedRelationValues = useMemo(() => {
    return Object.fromEntries(
      csvFields
        .filter((field) => field.relation)
        .map((field) => [
          field.key,
          csvUniqueValues[field.key].filter((value) => !csvRelationMap[field.key]?.[value]),
        ])
    ) as Record<CsvFieldKey, string[]>;
  }, [csvRelationMap, csvUniqueValues]);
  const csvAutoStats = useMemo(() => {
    const mappedColumns = csvFields.filter((field) => csvColumnMap[field.key] !== undefined).length;
    const relationValues = csvFields
      .filter((field) => field.relation && csvColumnMap[field.key] !== undefined)
      .flatMap((field) => csvUniqueValues[field.key].map((value) => ({ field: field.key, value })));
    const mappedRelations = relationValues.filter((item) => Boolean(csvRelationMap[item.field]?.[item.value])).length;
    return { mappedColumns, relationValues: relationValues.length, mappedRelations };
  }, [csvColumnMap, csvRelationMap, csvUniqueValues]);
  const touchedTables = useMemo(() => {
    if (profile === 'ARTICULOS' || profile === 'CSV_PRODUCTOS_MAPEADO') {
      return ['productos', 'inventario_sucursal', 'proveedores', 'marcas', 'categorias', 'unidades'];
    }
    const map: Record<Exclude<ImportProfile, 'ARTICULOS' | 'CSV_PRODUCTOS_MAPEADO'>, string[]> = {
      CLIENTES: ['clientes'],
      PROVEEDORES: ['proveedores'],
      MARCAS: ['marcas'],
      CATEGORIAS: ['categorias'],
      UNIDADES: ['unidades'],
    };
    return map[profile];
  }, [profile]);
  const quality = useMemo(() => {
    if (profile === 'CSV_PRODUCTOS_MAPEADO') {
      const warnings = [
        csvHeaders.length === 0 ? 'Selecciona un CSV para iniciar el mapeo.' : '',
        !csvMappedRequired ? 'Faltan campos obligatorios por mapear: Código, Descripción, Precio de venta, Proveedor y Sucursal.' : '',
        !csvRelationComplete ? 'Hay valores relacionales sin homologar.' : '',
        ...(csvAnalysis?.warnings ?? []),
      ].filter(Boolean);
      return {
        readyRows: csvHeaders.length > 0 && csvMappedRequired && csvRelationComplete ? (csvAnalysis?.totalFilas ?? 0) : 0,
        omittedRows: 0,
        warnings,
      };
    }
    if (rawRows.length === 0) {
      return { readyRows: 0, omittedRows: 0, warnings: [] as string[] };
    }
    if (profile === 'ARTICULOS') {
      const omittedRows = rawRows.length - rows.length;
      const warnings = [
        omittedRows > 0 ? `${omittedRows} filas no tienen descripción y se omitirán.` : '',
        countDuplicated(rows.map((row) => row.id)) > 0 ? 'Hay IDs legacy repetidos; se actualizará el último valor importado.' : '',
        countDuplicated(rows.map((row) => row.codigoBarra)) > 0 ? 'Hay códigos de barras repetidos; conviene depurarlos antes de vender.' : '',
        rows.some((row) => (row.precioVenta ?? row.precio1 ?? row.precio2 ?? 0) <= 0) ? 'Hay productos con precio de venta en cero.' : '',
        rows.some((row) => (row.existenciaStock ?? 0) < 0) ? 'Hay productos con existencia negativa; se importarán como vienen.' : '',
      ].filter(Boolean);
      return { readyRows: rows.length, omittedRows, warnings };
    }
    const omittedRows = rawRows.filter((row) => getGenericRequiredValue(profile, row).trim().length === 0).length;
    const warnings = omittedRows > 0 ? [`${omittedRows} filas no tienen nombre/descripcion reconocible y se omitirán.`] : [];
    return { readyRows: rawRows.length - omittedRows, omittedRows, warnings };
  }, [csvAnalysis, csvFilePath, csvHeaders.length, csvMappedRequired, csvRelationComplete, profile, rawRows, rows]);
  const canImport = quality.readyRows > 0 && (profile !== 'ARTICULOS' || sucursalId.trim().length > 0) && !isParsing && !isImporting;

  useEffect(() => {
    if (csvHeaders.length === 0) return;
    setCsvRelationMap((current) => {
      let changed = false;
      const next = { ...current };
      csvFields
        .filter((field) => field.relation && csvColumnMap[field.key] !== undefined)
        .forEach((field) => {
          const currentFieldMap = next[field.key] ?? {};
          const values = csvUniqueValues[field.key] ?? [];
          const nextFieldMap = { ...currentFieldMap };
          values.forEach((value) => {
            if (nextFieldMap[value]) return;
            const autoValue = resolveAutoRelationValue(field.key, value);
            if (autoValue) {
              nextFieldMap[value] = autoValue;
              changed = true;
            }
          });
          next[field.key] = nextFieldMap;
        });
      return changed ? next : current;
    });
  }, [catalogOptions, csvColumnMap, csvHeaders.length, csvUniqueValues]);

  const runCsvAnalysis = async (nextColumnMap = csvColumnMap, nextDelimiter = csvDelimiter, nextFilePath = csvFilePath) => {
    if (profile !== 'CSV_PRODUCTOS_MAPEADO' || !nextFilePath.trim() || Object.keys(nextColumnMap).length === 0) {
      setCsvAnalysis(null);
      return;
    }

    setIsParsing(true);
    try {
      const columnIndexes = Object.fromEntries(
        Object.entries(nextColumnMap).filter(([, value]) => value !== undefined)
      );
      const analysis = await invoke<AnalizarCsvImportacionResult>('analizar_csv_importacion', {
        payload: {
          filePath: nextFilePath,
          columnIndexes,
          delimiter: nextDelimiter,
        },
      });
      setCsvAnalysis(analysis);
      setCsvUniqueValues((current) => {
        const next = { ...current };
        csvFields.forEach((field) => {
          next[field.key] = analysis.uniqueValues[field.key] ?? [];
        });
        return next;
      });
      setCsvRelationMap((current) => {
        const next = { ...current };
        csvFields.forEach((field) => {
          const values = analysis.uniqueValues[field.key] ?? [];
          if (values.length === 0) return;
          next[field.key] = Object.fromEntries(
            values.map((value) => [value, current[field.key]?.[value] ?? resolveAutoRelationValue(field.key, value)])
          );
        });
        return next;
      });
    } catch (analysisError) {
      setError(String(analysisError));
    } finally {
      setIsParsing(false);
    }
  };

  const initializeMappedCsv = async (filePath: string, fileLabel: string, headers: string[], delimiter = csvDelimiter) => {
    const autoMap = buildAutoColumnMap(headers);
    setCsvFilePath(filePath);
    setCsvDelimiter(delimiter);
    setCsvHeaders(headers);
    setCsvColumnMap(autoMap);
    setCsvAnalysis(null);
    setCsvImportErrors([]);
    setCsvUniqueValues({
      codigo: [],
      descripcion: [],
      precioVenta: [],
      precioCosto: [],
      stock: [],
      stockMinimo: [],
      codigoBarras: [],
      proveedorId: [],
      marca: [],
      categoria: [],
      unidad: [],
    });
    setCsvRelationMap({});
    setRawRows([]);
    setRows([]);
    setFileName(fileLabel);
    setSuccess(`CSV cargado: ${headers.length} encabezados detectados. Dejé un mapeo sugerido para que lo revises antes de importar.`);
    if (filePath) {
      await runCsvAnalysis(autoMap, delimiter, filePath);
    }
  };

  const handleSelectCsvNative = async () => {
    setError('');
    setSuccess('');
    setIsParsing(true);
    try {
      const selected = await invoke<CsvArchivoSeleccionado | null>('seleccionar_archivo_csv_importacion');
      if (!selected) return;
      await initializeMappedCsv(selected.filePath, selected.fileName, selected.headers, selected.delimiter);
    } catch (selectError) {
      setError(String(selectError));
    } finally {
      setIsParsing(false);
    }
  };

  const handleCsvFieldMapChange = async (field: CsvFieldKey, rawValue: string) => {
    const nextIndex = rawValue === '' ? undefined : Number(rawValue);
    const nextMap = { ...csvColumnMap, [field]: nextIndex };
    if (nextIndex === undefined) {
      delete nextMap[field];
    }
    setCsvColumnMap(nextMap);
    setCsvAnalysis(null);
    setCsvImportErrors([]);

    const relationField = csvFields.find((item) => item.key === field && item.relation);
    if (!relationField || nextIndex === undefined) {
      setCsvUniqueValues((current) => ({ ...current, [field]: [] }));
      setCsvRelationMap((current) => ({ ...current, [field]: {} }));
      await runCsvAnalysis(nextMap);
      return;
    }

    await runCsvAnalysis(nextMap);
  };

  const handleAutoMapCsv = async () => {
    if (csvHeaders.length === 0) return;
    const autoMap = buildAutoColumnMap(csvHeaders);
    setCsvColumnMap(autoMap);
    setCsvAnalysis(null);
    setCsvImportErrors([]);
    setCsvRelationMap({});
    await runCsvAnalysis(autoMap);
  };

  const executeImport = async () => {
    if (!canImport) return;
    setIsImporting(true);
    setConfirmImportOpen(false);
    setError('');
    setSuccess('');
    try {
      if (profile === 'CSV_PRODUCTOS_MAPEADO') {
        const columnIndexes = Object.fromEntries(
          Object.entries(csvColumnMap).filter(([, value]) => value !== undefined)
        );
        const result = await invoke<ImportarCsvProductosMapeadoResult>('importar_csv_productos_mapeado', {
          payload: {
            filePath: csvFilePath,
            sucursalId,
            columnIndexes,
            delimiter: csvDelimiter,
            foreignKeyMap: csvRelationMap,
          },
        });
        setCsvImportErrors(result.errores ?? []);
        setSuccess(
          `Importación por streaming completada. Leídos: ${result.totalLeidos}. Productos: ${result.productosUpsertados}. Inventario: ${result.inventarioUpsertado}. Omitidos: ${result.filasOmitidas}.`
        );
      } else if (profile === 'ARTICULOS') {
        if (rows.length === 0) {
          throw new Error('El archivo no tiene columnas suficientes para importar artículos.');
        }
        const result = await invoke<ImportarArticulosLegacyResult>('importar_articulos_legacy_visual', {
          payload: {
            sucursalId,
            proveedorDefaultId: proveedorDefaultId.trim(),
            rows,
          },
        });
        setSuccess(
          `Importación completada. Leídos: ${result.totalLeidos}. Productos: ${result.productosUpsertados}. Inventario: ${result.inventarioUpsertado}. Catálogos: ${result.catalogosActualizados}.`
        );
      } else {
        const result = await invoke<ImportarDatosUniversalResult>('importar_datos_universal_visual', {
          payload: {
            destino: profile,
            rows: rawRows,
          },
        });
        setSuccess(
          `Importación completada. Destino: ${result.destino}. Leídos: ${result.totalLeidos}. Guardados: ${result.registrosUpsertados}. Omitidos: ${result.omitidos}.`
        );
      }
      await refreshCatalogos();
    } catch (importError) {
      setError(String(importError));
    } finally {
      setIsImporting(false);
    }
  };

  const handleImport = () => {
    if (!canImport) return;
    setConfirmImportOpen(true);
  };

  return (
    <Box sx={{ width: '100%', mt: 3 }}>
      <Card elevation={0} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
        <CardContent sx={{ p: 3 }}>
          <Typography variant="h6" sx={{ fontWeight: 800, mb: 0.5 }}>
            Importador CSV de Productos e Inventario
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2.5 }}>
            Selecciona un CSV, revisa el mapeo sugerido y homologa catálogos antes de importar a productos e inventario.
          </Typography>

          <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '1.2fr 1fr auto' }, gap: 1.5, mb: 2, alignItems: 'start' }}>
            <TextField
              select
              label="Sucursal destino"
              value={sucursalId}
              onChange={(event) => setSucursalId(event.target.value)}
              required
            >
              {sucursales.map((sucursal) => (
                <MenuItem key={sucursal.id} value={sucursal.id}>
                  {sucursal.nombre}
                </MenuItem>
              ))}
            </TextField>
            <TextField
              select
              label="Delimitador"
              value={csvDelimiter}
              onChange={(event) => {
                const nextDelimiter = event.target.value as CsvDelimiter;
                setCsvDelimiter(nextDelimiter);
                setCsvAnalysis(null);
                setCsvImportErrors([]);
                runCsvAnalysis(csvColumnMap, nextDelimiter, csvFilePath);
              }}
            >
              {csvDelimiters.map((option) => (
                <MenuItem key={option.value} value={option.value}>
                  {option.label}
                </MenuItem>
              ))}
            </TextField>
            <Button
              variant="contained"
              startIcon={<FileOpenIcon />}
              sx={{ minHeight: 56, px: 3 }}
              disabled={isParsing || isImporting}
              onClick={handleSelectCsvNative}
            >
              Seleccionar CSV
            </Button>
          </Box>

          {fileName && (
            <Paper
              variant="outlined"
              sx={{
                p: 1.5,
                mb: 2,
                borderRadius: 2,
                display: 'flex',
                alignItems: 'center',
                flexWrap: 'wrap',
                gap: 1,
              }}
            >
              <Typography variant="body2" sx={{ fontWeight: 800 }}>
                {fileName}
              </Typography>
              <Chip
                size="small"
                variant="outlined"
                label={`Delimitador: ${csvDelimiters.find((item) => item.value === csvDelimiter)?.label ?? csvDelimiter}`}
              />
              {csvHeaders.length > 0 && <Chip size="small" variant="outlined" label={`${csvHeaders.length} columnas`} />}
            </Paper>
          )}

          {error && <Alert severity="error" sx={{ mb: 2 }}>{error}</Alert>}
          {success && <Alert severity="success" sx={{ mb: 2 }}>{success}</Alert>}

          {isParsing && (
            <Paper sx={{ p: 2, mb: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
              <Box sx={{ display: 'flex', alignItems: 'center', gap: 1.5 }}>
                <CircularProgress size={18} />
                <Typography variant="body2">Leyendo archivo y normalizando columnas...</Typography>
              </Box>
            </Paper>
          )}

          {profile === 'CSV_PRODUCTOS_MAPEADO' && csvHeaders.length > 0 && (
            <Paper sx={{ p: 2, mb: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', gap: 1.5, alignItems: { xs: 'stretch', sm: 'center' }, flexDirection: { xs: 'column', sm: 'row' }, mb: 2 }}>
                <Box>
                  <Typography variant="subtitle2" sx={{ fontWeight: 800, mb: 0.5 }}>
                    Mapeo de columnas del CSV
                  </Typography>
                  <Typography variant="caption" color="text.secondary" sx={{ display: 'block' }}>
                    Se ha dejado una selección sugerida. Revísala y ajusta solo lo que no coincida.
                  </Typography>
                </Box>
                <Button variant="outlined" startIcon={<ImportIcon />} onClick={handleAutoMapCsv} disabled={isParsing || csvHeaders.length === 0}>
                  Automapear
                </Button>
              </Box>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.75, mb: 2 }}>
                <Chip size="small" color="primary" variant="outlined" label={`${csvAutoStats.mappedColumns}/${csvFields.length} columnas seleccionadas`} />
                <Chip
                  size="small"
                  color={csvAutoStats.relationValues === csvAutoStats.mappedRelations ? 'success' : 'warning'}
                  variant="outlined"
                  label={`${csvAutoStats.mappedRelations}/${csvAutoStats.relationValues} equivalencias resueltas`}
                />
              </Box>
              {csvAnalysis && (
                <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr 1fr', md: 'repeat(4, 1fr)' }, gap: 1.5, mb: 2 }}>
                  <Paper variant="outlined" sx={{ p: 1.5, borderRadius: 2 }}>
                    <Typography variant="caption" color="text.secondary">Filas detectadas</Typography>
                    <Typography variant="h6" sx={{ fontWeight: 800 }}>{csvAnalysis.totalFilas}</Typography>
                  </Paper>
                  <Paper variant="outlined" sx={{ p: 1.5, borderRadius: 2 }}>
                    <Typography variant="caption" color="text.secondary">Proveedores únicos</Typography>
                    <Typography variant="h6" sx={{ fontWeight: 800 }}>{csvUniqueValues.proveedorId.length}</Typography>
                  </Paper>
                  <Paper variant="outlined" sx={{ p: 1.5, borderRadius: 2 }}>
                    <Typography variant="caption" color="text.secondary">Marcas únicas</Typography>
                    <Typography variant="h6" sx={{ fontWeight: 800 }}>{csvUniqueValues.marca.length}</Typography>
                  </Paper>
                  <Paper variant="outlined" sx={{ p: 1.5, borderRadius: 2 }}>
                    <Typography variant="caption" color="text.secondary">Categorías únicas</Typography>
                    <Typography variant="h6" sx={{ fontWeight: 800 }}>{csvUniqueValues.categoria.length}</Typography>
                  </Paper>
                </Box>
              )}
              <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: 'repeat(3, 1fr)' }, gap: 1.5 }}>
                {csvFields.map((field) => (
                  <TextField
                    key={field.key}
                    select
                    label={`${field.label}${field.required ? ' *' : ''}`}
                    value={csvColumnMap[field.key] ?? ''}
                    onChange={(event) => handleCsvFieldMapChange(field.key, event.target.value)}
                    helperText={field.relation ? 'Al seleccionar columna se pedirán equivalencias.' : 'Selecciona el encabezado correspondiente.'}
                    required={field.required}
                  >
                    <MenuItem value="">No mapear</MenuItem>
                    {csvHeaders.map((header, index) => (
                      <MenuItem key={`${field.key}-${header}-${index}`} value={index}>
                        {index + 1}. {header}
                      </MenuItem>
                    ))}
                  </TextField>
                ))}
              </Box>

              {csvAnalysis?.previewRows.length ? (
                <Box sx={{ mt: 2 }}>
                  <Typography variant="subtitle2" sx={{ fontWeight: 800, mb: 1 }}>
                    Vista previa mapeada
                  </Typography>
                  <TableContainer component={Paper} variant="outlined" sx={{ borderRadius: 2, maxHeight: 260 }}>
                    <Table size="small" stickyHeader>
                      <TableHead>
                        <TableRow>
                          {csvFields.filter((field) => csvColumnMap[field.key] !== undefined).map((field) => (
                            <TableCell key={`preview-head-${field.key}`}>{field.label}</TableCell>
                          ))}
                        </TableRow>
                      </TableHead>
                      <TableBody>
                        {csvAnalysis.previewRows.map((row, index) => (
                          <TableRow key={`csv-preview-${index}`} hover>
                            {csvFields.filter((field) => csvColumnMap[field.key] !== undefined).map((field) => (
                              <TableCell key={`csv-preview-${index}-${field.key}`}>{row[field.key] || '-'}</TableCell>
                            ))}
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </TableContainer>
                </Box>
              ) : null}

              {csvFields
                .filter((field) => field.relation && csvUniqueValues[field.key].length > 0)
                .map((field) => {
                  const options = field.relation ? catalogOptions[field.relation] : [];
                  const unresolvedValues = csvUnresolvedRelationValues[field.key] ?? [];
                  const resolvedCount = csvUniqueValues[field.key].length - unresolvedValues.length;
                  return (
                    <Box key={`homologacion-${field.key}`} sx={{ mt: 2.5 }}>
                      <Divider sx={{ mb: 2 }} />
                      <Typography variant="subtitle2" sx={{ fontWeight: 800 }}>
                        Homologación de {field.label}
                      </Typography>
                      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mb: 1.5 }}>
                        {unresolvedValues.length > 0
                          ? `Solo se muestran los valores que no se pudieron resolver automáticamente. Resueltos: ${resolvedCount}/${csvUniqueValues[field.key].length}.`
                          : `Todas las equivalencias se resolvieron automáticamente (${resolvedCount}/${csvUniqueValues[field.key].length}).`}
                      </Typography>
                      {unresolvedValues.length === 0 ? (
                        <Alert severity="success" variant="outlined">
                          No necesitas homologar {field.label.toLowerCase()}; el CSV coincide con el catálogo actual.
                        </Alert>
                      ) : (
                      <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr', md: '1fr 1fr' }, gap: 1.25 }}>
                        {unresolvedValues.map((legacyValue) => {
                          const selectedId = csvRelationMap[field.key]?.[legacyValue] ?? '';
                          const selectedOption = options.find((option) => option.id === selectedId) ?? null;
                          return (
                            <Autocomplete
                              key={`${field.key}-${legacyValue}`}
                              size="small"
                              options={options}
                              value={selectedOption}
                              isOptionEqualToValue={(option, value) => option.id === value.id}
                              getOptionLabel={(option) => option.label}
                              noOptionsText="Sin coincidencias"
                              onChange={(_, option) =>
                                setCsvRelationMap((current) => ({
                                  ...current,
                                  [field.key]: {
                                    ...(current[field.key] ?? {}),
                                    [legacyValue]: option?.id ?? '',
                                  },
                                }))
                              }
                              renderInput={(params) => (
                                <TextField
                                  {...params}
                                  label={`CSV: ${legacyValue}`}
                                  required
                                  helperText="Busca y selecciona una equivalencia real."
                                />
                              )}
                            />
                          );
                        })}
                      </Box>
                      )}
                      {csvUniqueValues[field.key].length >= 200 && (
                        <Alert severity="info" sx={{ mt: 1.5 }}>
                          Se muestran las primeras 200 equivalencias únicas para evitar saturar la pantalla.
                        </Alert>
                      )}
                    </Box>
                  );
                })}
              {quality.warnings.length > 0 && (
                <Alert severity="warning" sx={{ mt: 2 }}>
                  {quality.warnings.map((warning) => (
                    <Typography key={warning} variant="body2">{warning}</Typography>
                  ))}
                </Alert>
              )}
              {csvImportErrors.length > 0 && (
                <Box sx={{ mt: 2 }}>
                  <Alert severity="error" sx={{ mb: 1.5 }}>
                    Se omitieron filas durante la importación. Revisa los primeros errores para corregir el CSV o las homologaciones.
                  </Alert>
                  <TableContainer component={Paper} variant="outlined" sx={{ borderRadius: 2, maxHeight: 260 }}>
                    <Table size="small" stickyHeader>
                      <TableHead>
                        <TableRow>
                          <TableCell>Fila</TableCell>
                          <TableCell>Código</TableCell>
                          <TableCell>Descripción</TableCell>
                          <TableCell>Motivo</TableCell>
                        </TableRow>
                      </TableHead>
                      <TableBody>
                        {csvImportErrors.map((issue) => (
                          <TableRow key={`${issue.fila}-${issue.codigo}-${issue.motivo}`} hover>
                            <TableCell>{issue.fila}</TableCell>
                            <TableCell>{issue.codigo || '-'}</TableCell>
                            <TableCell>{issue.descripcion || '-'}</TableCell>
                            <TableCell>{issue.motivo}</TableCell>
                          </TableRow>
                        ))}
                      </TableBody>
                    </Table>
                  </TableContainer>
                </Box>
              )}
              <Box sx={{ display: 'flex', justifyContent: 'flex-end', mt: 2 }}>
                <Button
                  variant="contained"
                  color="primary"
                  startIcon={isImporting ? <UploadIcon /> : <ImportIcon />}
                  onClick={handleImport}
                  disabled={!canImport}
                >
                  {isImporting ? 'Importando...' : 'Importar CSV Mapeado'}
                </Button>
              </Box>
            </Paper>
          )}

          {rawRows.length > 0 && (
            <Paper sx={{ p: 2, mb: 2, borderRadius: 2, border: '1px solid', borderColor: 'divider' }}>
              <Box sx={{ display: 'grid', gridTemplateColumns: { xs: '1fr 1fr', md: 'repeat(4, 1fr)' }, gap: 1.5 }}>
                <Box>
                  <Typography variant="caption" color="text.secondary">Filas leídas</Typography>
                  <Typography variant="h6" sx={{ fontWeight: 800 }}>{rawRows.length}</Typography>
                </Box>
                <Box>
                  <Typography variant="caption" color="text.secondary">Listas para importar</Typography>
                  <Typography variant="h6" sx={{ fontWeight: 800 }}>{quality.readyRows}</Typography>
                </Box>
                <Box>
                  <Typography variant="caption" color="text.secondary">Omitidas</Typography>
                  <Typography variant="h6" sx={{ fontWeight: 800 }}>{quality.omittedRows}</Typography>
                </Box>
                <Box>
                  <Typography variant="caption" color="text.secondary">Destino</Typography>
                  <Typography variant="body2" sx={{ fontWeight: 700 }}>{profileConfig?.label ?? profile}</Typography>
                </Box>
              </Box>
              <Divider sx={{ my: 1.5 }} />
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.75, alignItems: 'center' }}>
                <Typography variant="caption" color="text.secondary" sx={{ mr: 0.5 }}>
                  Tablas afectadas:
                </Typography>
                {touchedTables.map((table) => (
                  <Chip key={table} size="small" label={table} variant="outlined" />
                ))}
              </Box>
              {quality.warnings.length > 0 && (
                <Alert severity="warning" sx={{ mt: 1.5 }}>
                  {quality.warnings.map((warning) => (
                    <Typography key={warning} variant="body2">{warning}</Typography>
                  ))}
                </Alert>
              )}
            </Paper>
          )}

          {profile !== 'CSV_PRODUCTOS_MAPEADO' && (
          <>
          <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 1 }}>
            <Box>
              <Typography variant="subtitle2" sx={{ fontWeight: 700 }}>
                Vista previa ({rawRows.length} filas)
              </Typography>
              <Typography variant="caption" color="text.secondary">
                {profileConfig?.helper}
              </Typography>
            </Box>
            <Button
              variant="contained"
              color="primary"
              startIcon={isImporting ? <UploadIcon /> : <ImportIcon />}
              onClick={handleImport}
              disabled={!canImport}
            >
              {isImporting ? 'Importando...' : 'Importar a Sistema'}
            </Button>
          </Box>

          <TableContainer component={Paper} sx={{ border: '1px solid', borderColor: 'divider', borderRadius: 2 }}>
            <Table size="small" stickyHeader>
              <TableHead>
                {profile === 'ARTICULOS' ? (
                  <TableRow>
                    <TableCell>ID legacy</TableCell>
                    <TableCell>Clave</TableCell>
                    <TableCell>Descripción</TableCell>
                    <TableCell>Proveedor</TableCell>
                    <TableCell>Marca</TableCell>
                    <TableCell>Unidad</TableCell>
                    <TableCell>Stock</TableCell>
                    <TableCell>Costo</TableCell>
                    <TableCell>Venta</TableCell>
                  </TableRow>
                ) : (
                  <TableRow>
                    {(genericColumns.length > 0 ? genericColumns : ['Sin columnas']).map((column) => (
                      <TableCell key={column}>{column}</TableCell>
                    ))}
                  </TableRow>
                )}
              </TableHead>
              <TableBody>
                {profile === 'ARTICULOS'
                  ? previewRows.map((row, index) => (
                      <TableRow key={`${row.id ?? 'auto'}-${index}`} hover>
                        <TableCell>{row.id ?? '-'}</TableCell>
                        <TableCell>{row.clave ?? '-'}</TableCell>
                        <TableCell>{row.descripcionArticulo ?? '-'}</TableCell>
                        <TableCell>{row.proveedorNombre ?? (row.provedor ? `ID ${row.provedor}` : '-')}</TableCell>
                        <TableCell>{row.marcaNombre ?? (row.marca ? `ID ${row.marca}` : 'Sin marca')}</TableCell>
                        <TableCell>{row.unidad ?? '-'}</TableCell>
                        <TableCell>{row.existenciaStock ?? 0}</TableCell>
                        <TableCell>{row.precioCompra ?? 0}</TableCell>
                        <TableCell>{row.precioVenta ?? row.precio1 ?? row.precio2 ?? 0}</TableCell>
                      </TableRow>
                    ))
                  : genericPreviewRows.map((row, index) => (
                      <TableRow key={index} hover>
                        {genericColumns.map((column) => (
                          <TableCell key={column}>{String(row[column] ?? '-')}</TableCell>
                        ))}
                        {genericColumns.length === 0 && <TableCell>-</TableCell>}
                      </TableRow>
                    ))}
                {rawRows.length === 0 && (
                  <TableRow>
                    <TableCell colSpan={profile === 'ARTICULOS' ? 9 : Math.max(genericColumns.length, 1)} sx={{ py: 3, textAlign: 'center', color: 'text.secondary' }}>
                      Carga un archivo para ver vista previa.
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          </TableContainer>
          {rawRows.length > 20 && (
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
              Mostrando las primeras 20 filas de {rawRows.length}. La importación procesará todo el archivo.
            </Typography>
          )}
          </>
          )}
        </CardContent>
      </Card>
      <ConfirmActionDialog
        open={confirmImportOpen}
        title="Confirmar importación"
        message={`Se importarán ${quality.readyRows} registros hacia ${profileConfig?.label ?? profile}. Tablas afectadas: ${touchedTables.join(', ')}.`}
        confirmText="Importar"
        confirmColor="primary"
        loading={isImporting}
        onCancel={() => setConfirmImportOpen(false)}
        onConfirm={executeImport}
      />
    </Box>
  );
}

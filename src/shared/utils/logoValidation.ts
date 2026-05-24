export interface LogoValidationResult {
  dataUrl: string;
  width: number;
  height: number;
  warning?: string;
}

const MAX_LOGO_SIZE_MB = 3;
const MIN_LOGO_SIDE = 96;
const MAX_ASPECT_RATIO = 6;

export function validateLogoFile(file: File): Promise<LogoValidationResult> {
  return new Promise((resolve, reject) => {
    if (!file.type.startsWith('image/')) {
      reject(new Error('Selecciona un archivo de imagen válido.'));
      return;
    }

    if (file.size > MAX_LOGO_SIZE_MB * 1024 * 1024) {
      reject(new Error(`El logo no debe pesar más de ${MAX_LOGO_SIZE_MB} MB.`));
      return;
    }

    const reader = new FileReader();
    reader.onerror = () => reject(new Error('No se pudo leer el archivo del logo.'));
    reader.onload = () => {
      const dataUrl = String(reader.result || '');
      const img = new Image();

      img.onerror = () => reject(new Error('No se pudo validar la imagen seleccionada.'));
      img.onload = () => {
        const shortestSide = Math.min(img.width, img.height);
        const longestSide = Math.max(img.width, img.height);
        const aspectRatio = longestSide / Math.max(shortestSide, 1);

        if (shortestSide < MIN_LOGO_SIDE) {
          reject(new Error(`El logo es demasiado pequeño. Usa una imagen de al menos ${MIN_LOGO_SIDE}px en su lado menor.`));
          return;
        }

        const warning = aspectRatio > MAX_ASPECT_RATIO
          ? 'El logo tiene una proporción muy alargada. Se guardará, pero podría verse pequeño en algunos espacios.'
          : undefined;

        resolve({
          dataUrl,
          width: img.width,
          height: img.height,
          warning,
        });
      };

      img.src = dataUrl;
    };

    reader.readAsDataURL(file);
  });
}

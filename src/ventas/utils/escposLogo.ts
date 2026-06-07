const paperDotsByWidth: Record<number, number> = {
  32: 384,
  42: 504,
  48: 576,
};

function loadImage(src: string) {
  return new Promise<HTMLImageElement>((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error('No se pudo preparar el logotipo para impresión.'));
    image.src = src;
  });
}

export async function buildEscposLogoRaster(logoSrc?: string | null, paperWidth = 42) {
  if (!logoSrc) return undefined;

  const image = await loadImage(logoSrc);
  const maxDots = paperDotsByWidth[paperWidth] ?? 504;
  const targetWidth = Math.min(maxDots, 384);
  const scale = Math.min(targetWidth / image.width, 1);
  const width = Math.max(1, Math.round(image.width * scale));
  const height = Math.max(1, Math.round(image.height * scale));

  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d');
  if (!ctx) return undefined;

  ctx.fillStyle = '#fff';
  ctx.fillRect(0, 0, width, height);
  ctx.drawImage(image, 0, 0, width, height);

  const imageData = ctx.getImageData(0, 0, width, height).data;
  const bytesPerRow = Math.ceil(width / 8);
  const raster: number[] = [];

  for (let y = 0; y < height; y += 1) {
    for (let xByte = 0; xByte < bytesPerRow; xByte += 1) {
      let byte = 0;
      for (let bit = 0; bit < 8; bit += 1) {
        const x = xByte * 8 + bit;
        if (x >= width) continue;
        const idx = (y * width + x) * 4;
        const alpha = imageData[idx + 3] / 255;
        const luminance = (imageData[idx] * 0.299 + imageData[idx + 1] * 0.587 + imageData[idx + 2] * 0.114) * alpha + 255 * (1 - alpha);
        if (luminance < 180) {
          byte |= 0x80 >> bit;
        }
      }
      raster.push(byte);
    }
  }

  return [
    0x1d,
    0x76,
    0x30,
    0x00,
    bytesPerRow & 0xff,
    (bytesPerRow >> 8) & 0xff,
    height & 0xff,
    (height >> 8) & 0xff,
    ...raster,
  ];
}

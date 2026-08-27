// One-off generator for the source app icon (icons/icon.png, 1024x1024).
// Draws a simple mark by hand instead of pulling in an image dependency:
// a dark rounded square, an accent-blue circle, and a smaller cut-out
// crescent to keep it recognizable at small sizes.
const fs = require("fs");
const path = require("path");
const zlib = require("zlib");

const SIZE = 1024;
const BG = [15, 17, 21, 255]; // #0f1115
const PANEL = [23, 26, 33, 255]; // #171a21
const ACCENT = [79, 140, 255, 255]; // #4f8cff

function dist(x, y, cx, cy) {
  return Math.hypot(x - cx, y - cy);
}

function roundedSquareMask(x, y, size, cx, cy, half, radius) {
  const dx = Math.abs(x - cx);
  const dy = Math.abs(y - cy);
  if (dx > half || dy > half) return false;
  const ix = half - radius;
  if (dx <= ix || dy <= ix) return true;
  return dist(dx, dy, ix, ix) <= radius;
}

const pixels = Buffer.alloc(SIZE * SIZE * 4);
const cx = SIZE / 2;
const cy = SIZE / 2;

for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    let color = BG;

    if (roundedSquareMask(x, y, SIZE, cx, cy, SIZE * 0.46, SIZE * 0.14)) {
      color = PANEL;
    }

    const rOuter = SIZE * 0.27;
    const dOuter = dist(x, y, cx, cy);
    if (dOuter <= rOuter) {
      color = ACCENT;
      // Crescent cut-out: a second circle offset up-right, painted back
      // in the panel color, so the accent reads as a forged sliver.
      const dInner = dist(x, y, cx + SIZE * 0.09, cy - SIZE * 0.09);
      if (dInner <= rOuter * 0.85) {
        color = PANEL;
      }
    }

    const idx = (y * SIZE + x) * 4;
    pixels[idx] = color[0];
    pixels[idx + 1] = color[1];
    pixels[idx + 2] = color[2];
    pixels[idx + 3] = color[3];
  }
}

function crc32(buf) {
  let c;
  const table = crc32.table || (crc32.table = (() => {
    const t = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
      c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      t[n] = c >>> 0;
    }
    return t;
  })());
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) {
    crc = table[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const typeBuf = Buffer.from(type, "ascii");
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const crcBuf = Buffer.alloc(4);
  crcBuf.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crcBuf]);
}

const rawRows = Buffer.alloc((SIZE * 4 + 1) * SIZE);
for (let y = 0; y < SIZE; y++) {
  const rowStart = y * (SIZE * 4 + 1);
  rawRows[rowStart] = 0; // filter: none
  pixels.copy(rawRows, rowStart + 1, y * SIZE * 4, (y + 1) * SIZE * 4);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // color type RGBA
ihdr[10] = 0;
ihdr[11] = 0;
ihdr[12] = 0;

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", zlib.deflateSync(rawRows, { level: 9 })),
  chunk("IEND", Buffer.alloc(0)),
]);

const outDir = path.join(__dirname, "..", "src-tauri", "icons");
fs.mkdirSync(outDir, { recursive: true });
fs.writeFileSync(path.join(outDir, "icon.png"), png);
console.log("wrote", path.join(outDir, "icon.png"), `${png.length} bytes`);

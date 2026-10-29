import { mkdirSync, writeFileSync } from 'node:fs';
import { crc32, deflateSync } from 'node:zlib';

const N = 1024;
const BG = [24, 27, 34];
const ACCENT = [124, 156, 255];

// signed distance helpers (pixels)
function roundRect(x, y, half, r) {
  const qx = Math.abs(x) - (half - r);
  const qy = Math.abs(y) - (half - r);
  return Math.hypot(Math.max(qx, 0), Math.max(qy, 0)) + Math.min(Math.max(qx, qy), 0) - r;
}
function segment(px, py, ax, ay, bx, by) {
  const pax = px - ax;
  const pay = py - ay;
  const bax = bx - ax;
  const bay = by - ay;
  const h = Math.max(0, Math.min(1, (pax * bax + pay * bay) / (bax * bax + bay * bay)));
  return Math.hypot(pax - bax * h, pay - bay * h);
}
const coverage = (d) => Math.max(0, Math.min(1, 0.5 - d));

const raw = Buffer.alloc((1 + N * 4) * N);
for (let y = 0; y < N; y++) {
  raw[y * (1 + N * 4)] = 0; // filter: none
  for (let x = 0; x < N; x++) {
    const cx = x - N / 2 + 0.5;
    const cy = y - N / 2 + 0.5;
    const tile = coverage(roundRect(cx, cy, 480, 200));
    const check = Math.min(segment(cx, cy, -190, 10, -60, 150), segment(cx, cy, -60, 150, 210, -150)) - 55;
    const mark = coverage(check);
    const o = y * (1 + N * 4) + 1 + x * 4;
    for (let c = 0; c < 3; c++) raw[o + c] = Math.round(BG[c] * (1 - mark) + ACCENT[c] * mark);
    raw[o + 3] = Math.round(tile * 255);
  }
}

function chunk(type, data) {
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body) >>> 0);
  return Buffer.concat([len, body, crc]);
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(N, 0);
ihdr.writeUInt32BE(N, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // RGBA

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk('IHDR', ihdr),
  chunk('IDAT', deflateSync(raw, { level: 9 })),
  chunk('IEND', Buffer.alloc(0)),
]);

mkdirSync('src-tauri/icons', { recursive: true });
writeFileSync('src-tauri/icons/source.png', png);
console.log(`wrote src-tauri/icons/source.png (${png.length} bytes)`);

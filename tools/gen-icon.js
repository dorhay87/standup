// One-time icon generator: renders the Standup glyph (white standing person
// on a teal rounded square, matching the mockup's app icon) to PNGs.
// Pure Node - zlib for IDAT, hand-rolled PNG chunks. Run: npm run icons
'use strict';
const fs = require('fs');
const path = require('path');
const zlib = require('zlib');

const TEAL = [0x0d, 0x94, 0x88];
const WHITE = [0xff, 0xff, 0xff];

// Geometry relative to the 36px mockup icon:
// rounded square r=11/36; head circle d=8 centered at (18, 11); body bar 4x13 rounded 3 from y=16.
function coverage(x, y, size) {
  // Returns [r,g,b,a] for a point (x, y) in [0..size] space.
  const u = x / size, v = y / size;
  const rr = 11 / 36;
  // rounded-rect SDF-ish test
  const cx = Math.min(Math.max(u, rr), 1 - rr);
  const cy = Math.min(Math.max(v, rr), 1 - rr);
  const dRect = Math.hypot(u - cx, v - cy);
  if (dRect > rr) return null; // outside rounded square

  // head
  const hr = 4 / 36;
  if (Math.hypot(u - 0.5, v - 11 / 36) <= hr) return WHITE;
  // body: rounded bar, half-width 2/36, from y 16/36 to 29/36, corner radius 2/36 (capsule)
  const bw = 2 / 36, bTop = 16 / 36 + bw, bBot = 29 / 36 - bw;
  const by = Math.min(Math.max(v, bTop), bBot);
  if (Math.hypot(u - 0.5, v - by) <= bw) return WHITE;

  return TEAL;
}

function render(size) {
  const SS = 4; // supersampling
  const px = Buffer.alloc(size * size * 4);
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      let r = 0, g = 0, b = 0, a = 0;
      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const c = coverage(x + (sx + 0.5) / SS, y + (sy + 0.5) / SS, size);
          if (c) { r += c[0]; g += c[1]; b += c[2]; a += 255; }
        }
      }
      const n = SS * SS, i = (y * size + x) * 4;
      const cov = a / n / 255;
      // premultiplied average, un-premultiply for straight alpha
      px[i] = cov > 0 ? Math.round(r / n / cov) : 0;
      px[i + 1] = cov > 0 ? Math.round(g / n / cov) : 0;
      px[i + 2] = cov > 0 ? Math.round(b / n / cov) : 0;
      px[i + 3] = Math.round(a / n);
    }
  }
  return px;
}

const CRC_TABLE = (() => {
  const t = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c;
  }
  return t;
})();
function crc32(buf) {
  let c = -1;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return (c ^ -1) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([len, body, crc]);
}
function encodePNG(px, size) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(size, 0);
  ihdr.writeUInt32BE(size, 4);
  ihdr[8] = 8;  // bit depth
  ihdr[9] = 6;  // RGBA
  // scanlines with filter byte 0
  const raw = Buffer.alloc(size * (size * 4 + 1));
  for (let y = 0; y < size; y++) {
    raw[y * (size * 4 + 1)] = 0;
    px.copy(raw, y * (size * 4 + 1) + 1, y * size * 4, (y + 1) * size * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', ihdr),
    chunk('IDAT', zlib.deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

function write(file, size) {
  // Never overwrite - the shipped icons are custom-made, not generated.
  if (fs.existsSync(file)) {
    console.log('exists, skipping', file);
    return;
  }
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, encodePNG(render(size), size));
  console.log('wrote', file);
}

const root = path.join(__dirname, '..');
write(path.join(root, 'assets', 'tray-32.png'), 32);
write(path.join(root, 'build', 'icon.png'), 256);

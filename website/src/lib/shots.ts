// Screenshots in public/images/shots/ come in pairs: <name>-dark.png and <name>-light.png
// (see public/images/shots/INDEX.md when it exists). These helpers find them at build time
// and fall back to an older image under public/images/ while a shot hasn't been taken yet,
// so pages never point at a missing file.
import { existsSync, openSync, readSync, closeSync } from "node:fs";
import { join } from "node:path";

const PUBLIC = join(process.cwd(), "public");

export interface ImageRef {
  src: string;
  width?: number;
  height?: number;
}

export interface ThemedRef {
  light: ImageRef;
  dark: ImageRef;
  /** True when this is a real paired shot, false when it's the fallback image. */
  paired: boolean;
}

/** PNG / JPEG / WebP size from the file header, or undefined. */
export function imageSize(publicPath: string): { width: number; height: number } | undefined {
  const file = join(PUBLIC, publicPath.replace(/^\//, ""));
  if (!existsSync(file)) return undefined;
  const buf = Buffer.alloc(64 * 1024);
  const fd = openSync(file, "r");
  const n = readSync(fd, buf, 0, buf.length, 0);
  closeSync(fd);
  if (buf.readUInt32BE(0) === 0x89504e47) return { width: buf.readUInt32BE(16), height: buf.readUInt32BE(20) };
  if (buf.toString("ascii", 0, 4) === "RIFF" && buf.toString("ascii", 8, 12) === "WEBP") {
    const kind = buf.toString("ascii", 12, 16);
    if (kind === "VP8X") return { width: 1 + buf.readUIntLE(24, 3), height: 1 + buf.readUIntLE(27, 3) };
    if (kind === "VP8 ") return { width: buf.readUInt16LE(26) & 0x3fff, height: buf.readUInt16LE(28) & 0x3fff };
    if (kind === "VP8L") {
      const b = buf.readUInt32LE(21);
      return { width: (b & 0x3fff) + 1, height: ((b >> 14) & 0x3fff) + 1 };
    }
  }
  if (buf[0] === 0xff && buf[1] === 0xd8) {
    let i = 2;
    while (i < n - 9) {
      if (buf[i] !== 0xff) break;
      const marker = buf[i + 1];
      const len = buf.readUInt16BE(i + 2);
      if (marker >= 0xc0 && marker <= 0xc3) return { height: buf.readUInt16BE(i + 5), width: buf.readUInt16BE(i + 7) };
      i += 2 + len;
    }
  }
  return undefined;
}

const exists = (p: string) => existsSync(join(PUBLIC, p));

/**
 * The size to lay an image out at, in CSS pixels. Everything in images/shots/ is rendered at
 * 2x (BUI_UI_SCALE=2, see INDEX.md), so it's shown at half its pixel size: sharp on HiDPI
 * screens and the same size as the window was on a 1x screen.
 */
export function displaySize(publicPath: string): { width: number; height: number } | undefined {
  const size = imageSize(publicPath);
  if (!size) return undefined;
  if (/^\/?images\/shots\//.test(publicPath)) return { width: Math.round(size.width / 2), height: Math.round(size.height / 2) };
  return size;
}

const ref = (src: string): ImageRef => ({ src, ...displaySize(src) });

/**
 * A screenshot pair by name: shot("landing-tkinter", "images/tkinter_items.png").
 * Tries images/shots/<name>-light.png + -dark.png, then a single images/shots/<name>.png,
 * then the fallback (used for both themes).
 */
export function shot(name: string, fallback: string): ThemedRef {
  const base = `images/shots/${name}`;
  for (const ext of ["png", "webp", "jpg"]) {
    const l = `${base}-light.${ext}`;
    const d = `${base}-dark.${ext}`;
    if (exists(l) || exists(d)) {
      const light = ref(exists(l) ? l : d);
      const dark = ref(exists(d) ? d : l);
      return { light, dark, paired: exists(l) && exists(d) };
    }
    if (exists(`${base}.${ext}`)) {
      const one = ref(`${base}.${ext}`);
      return { light: one, dark: one, paired: false };
    }
  }
  const fb = ref(fallback);
  return { light: fb, dark: fb, paired: false };
}

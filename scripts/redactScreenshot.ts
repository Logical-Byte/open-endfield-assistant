/**
 * 1280×720 PNG 截图脱敏，不需要额外依赖。
 * pnpm exec jiti scripts/redactScreenshot.ts input.png output.png
 * 协议终端额外参数：--rect 218,572,115,32 --rect 210,645,135,25
 * 使用 --help 查看全部选项。
 */
import { readFileSync, writeFileSync } from 'node:fs';
import { parseArgs } from 'node:util';
import { deflateSync, inflateSync } from 'node:zlib';

type Rect = [number, number, number, number];
const width = 1280;
const height = 720;
const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
const uidRect: Rect = [68, 696, 132, 20];

function parseRect(value: string): Rect {
  const parts = value.split(',').map(Number);
  if (parts.length !== 4 || parts.some((n) => !Number.isInteger(n))) {
    throw new Error(`矩形应为 x,y,width,height：${value}`);
  }
  const [x, y, w, h] = parts;
  if (x < 0 || y < 0 || w <= 0 || h <= 0 || x + w > width || y + h > height) {
    throw new Error(`矩形超出 1280×720 画面：${value}`);
  }
  return [x, y, w, h];
}

function paeth(a: number, b: number, c: number): number {
  const p = a + b - c;
  const da = Math.abs(p - a);
  const db = Math.abs(p - b);
  const dc = Math.abs(p - c);
  return da <= db && da <= dc ? a : db <= dc ? b : c;
}

function chunk(type: string, data: Buffer): Buffer {
  const result = Buffer.alloc(data.length + 12);
  result.writeUInt32BE(data.length, 0);
  result.write(type, 4, 4, 'ascii');
  data.copy(result, 8);
  let crc = 0xffffffff;
  for (const byte of result.subarray(4, -4)) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit++) {
      crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
    }
  }
  result.writeUInt32BE((crc ^ 0xffffffff) >>> 0, result.length - 4);
  return result;
}

function redact(input: Buffer, rects: Rect[]): Buffer {
  if (!input.subarray(0, 8).equals(signature)) throw new Error('输入必须是 PNG');
  let header: Buffer | undefined;
  const compressed: Buffer[] = [];
  for (let offset = 8; offset < input.length;) {
    const length = input.readUInt32BE(offset);
    const type = input.toString('ascii', offset + 4, offset + 8);
    const end = offset + 12 + length;
    if (end > input.length) throw new Error('PNG 数据不完整');
    const data = input.subarray(offset + 8, end - 4);
    if (type === 'IHDR') header = data;
    if (type === 'IDAT') compressed.push(data);
    if (type === 'acTL') throw new Error('不支持动画 PNG');
    offset = end;
    if (type === 'IEND') break;
  }
  if (
    !header ||
    header.length !== 13 ||
    header.readUInt32BE(0) !== width ||
    header.readUInt32BE(4) !== height ||
    header[8] !== 8 ||
    ![2, 6].includes(header[9]) ||
    header[10] !== 0 ||
    header[11] !== 0 ||
    header[12] !== 0
  ) {
    throw new Error('只支持 1280×720、8 位、非交错 RGB/RGBA PNG');
  }
  const channels = header[9] === 6 ? 4 : 3;
  const stride = width * channels;
  const filtered = inflateSync(Buffer.concat(compressed));
  if (filtered.length !== (stride + 1) * height) throw new Error('PNG 像素长度不正确');
  const pixels = Buffer.alloc(stride * height);
  // PNG 每行可能使用不同的预测滤波，先还原像素再遮盖，避免影响矩形外的颜色。
  for (let y = 0; y < height; y++) {
    const filter = filtered[y * (stride + 1)];
    if (filter > 4) throw new Error(`未知 PNG 滤波类型：${filter}`);
    for (let x = 0; x < stride; x++) {
      const i = y * stride + x;
      const left = x >= channels ? pixels[i - channels] : 0;
      const up = y > 0 ? pixels[i - stride] : 0;
      const upperLeft = y > 0 && x >= channels ? pixels[i - stride - channels] : 0;
      const predictor = [0, left, up, Math.floor((left + up) / 2), paeth(left, up, upperLeft)][
        filter
      ];
      pixels[i] = (filtered[y * (stride + 1) + 1 + x] + predictor) & 255;
    }
  }
  for (const [x, y, w, h] of rects) {
    for (let row = y; row < y + h; row++) {
      for (let col = x; col < x + w; col++) {
        const i = row * stride + col * channels;
        pixels.fill(0, i, i + 3);
        if (channels === 4) pixels[i + 3] = 255;
      }
    }
  }
  const output = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    pixels.copy(output, y * (stride + 1) + 1, y * stride, (y + 1) * stride);
  }
  // 只输出图像必需的块，丢弃可能携带个人信息的文本及其他附加元数据。
  return Buffer.concat([
    signature,
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(output)),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

const { values, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    rect: { type: 'string', multiple: true },
    'no-uid': { type: 'boolean', default: false },
    help: { type: 'boolean', short: 'h' },
  },
});
if (values.help) {
  console.log(`用法：pnpm exec jiti scripts/redactScreenshot.ts <input.png> <output.png> [选项]
默认用纯黑遮盖左下角 UID：68,696,132,20。
  --rect x,y,width,height  额外遮盖的矩形，可重复指定
  --no-uid                 不遮盖默认 UID 区域
仅支持 1280×720、8 位、非交错 RGB/RGBA PNG。输出目录须已存在。`);
} else {
  if (positionals.length !== 2) throw new Error('请提供输入和输出路径，使用 --help 查看用法');
  const rects: Rect[] = values['no-uid'] ? [] : [uidRect];
  rects.push(...(values.rect ?? []).map(parseRect));
  writeFileSync(positionals[1], redact(readFileSync(positionals[0]), rects));
  console.log(`已脱敏：${positionals[1]}`);
}

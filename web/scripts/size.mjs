import { readdirSync, readFileSync } from 'node:fs';
import { gzipSync } from 'node:zlib';
import { join } from 'node:path';

const LIMIT = 200 * 1024;
const dir = join(import.meta.dirname, '..', 'dist', 'assets');
let total = 0;
for (const f of readdirSync(dir).filter((f) => f.endsWith('.js'))) {
    const size = gzipSync(readFileSync(join(dir, f))).length;
    total += size;
    console.log(`${f}: ${(size / 1024).toFixed(1)} KB gzip`);
}
console.log(`total: ${(total / 1024).toFixed(1)} KB / ${LIMIT / 1024} KB`);
if (total > LIMIT) {
    console.error('JS gzip budget exceeded');
    process.exit(1);
}

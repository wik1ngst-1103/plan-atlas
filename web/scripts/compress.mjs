// Writes a `.gz` sibling next to every compressible file in dist/ so the Rust
// binary can embed pre-compressed assets and serve them by Accept-Encoding.
import { readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { gzipSync, constants } from 'node:zlib';
import { join } from 'node:path';

const EXT = /\.(js|css|html|svg|json)$/;
const dist = join(import.meta.dirname, '..', 'dist');

function walk(dir) {
    for (const name of readdirSync(dir)) {
        const p = join(dir, name);
        if (statSync(p).isDirectory()) walk(p);
        else if (EXT.test(name)) {
            writeFileSync(p + '.gz', gzipSync(readFileSync(p), { level: constants.Z_BEST_COMPRESSION }));
        }
    }
}
walk(dist);

import { spawnSync } from 'node:child_process';
import { cpSync, mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';

const thisDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(thisDir, '..');
const distDir = join(thisDir, 'dist');
const wasmOutDir = join(distDir, 'pkg');
const assetOutDir = join(distDir, 'assets');

mkdirSync(wasmOutDir, { recursive: true });
mkdirSync(assetOutDir, { recursive: true });

const cargo = spawnSync(
  'cargo',
  ['build', '--release', '--lib', '--target', 'wasm32-unknown-unknown'],
  {
    cwd: repoRoot,
    stdio: 'inherit',
  },
);

if (cargo.status !== 0) {
  process.exit(cargo.status ?? 1);
}

cpSync(
  join(repoRoot, 'target', 'wasm32-unknown-unknown', 'release', 'gif_renderer.wasm'),
  join(wasmOutDir, 'gif_renderer.wasm'),
);
cpSync(join(thisDir, 'index.html'), join(distDir, 'index.html'));
cpSync(join(repoRoot, 'assets', 'earth.gif'), join(assetOutDir, 'earth.gif'));

await build({
  entryPoints: [join(thisDir, 'src', 'main.ts')],
  bundle: true,
  sourcemap: true,
  format: 'esm',
  target: 'es2022',
  outfile: join(distDir, 'app.js'),
  loader: {
    '.css': 'css',
  },
});

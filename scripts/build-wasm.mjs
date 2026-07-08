import fs from 'node:fs/promises';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const crateRoot = path.resolve(__dirname, '..');
const manifest = JSON.parse(
  await fs.readFile(path.join(crateRoot, 'editor', 'asset-manifest.json'), 'utf8'),
);
const targetPath = path.join(crateRoot, manifest.parser_wasm);

function run(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: crateRoot,
      stdio: 'inherit',
      shell: false,
    });

    child.on('error', reject);
    child.on('exit', (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`${command} ${args.join(' ')} exited with code ${code}`));
      }
    });
  });
}

async function findExistingWasm() {
  const candidates = [
    path.join(crateRoot, manifest.parser_wasm_file_name),
    path.join(crateRoot, 'build', manifest.parser_wasm_file_name),
    path.join(crateRoot, 'dist', manifest.parser_wasm_file_name),
    path.join(crateRoot, 'bindings', 'web', manifest.parser_wasm_file_name),
  ];

  for (const candidate of candidates) {
    try {
      await fs.access(candidate);
      return candidate;
    } catch {
      // Continue.
    }
  }

  return null;
}

await fs.mkdir(path.dirname(targetPath), { recursive: true });
await run('npx', ['tree-sitter', 'build', '--wasm']);

const sourcePath = await findExistingWasm();
if (!sourcePath) {
  throw new Error(`wasm output not found for ${manifest.language_id}`);
}

await fs.copyFile(sourcePath, targetPath);
console.log(`copied ${sourcePath} -> ${targetPath}`);

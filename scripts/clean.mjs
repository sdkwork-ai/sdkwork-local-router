import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '..');
const targets = [
  '.runtime/dev-sites',
  'apps/sdkwork-local-router-pc/dist',
  'apps/sdkwork-modelkit-pc/dist',
];

for (const target of targets) {
  fs.rmSync(path.join(root, target), { force: true, recursive: true });
}

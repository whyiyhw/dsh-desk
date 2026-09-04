#!/usr/bin/env node
// S12 version single-source guard: tauri.conf.json's `version` is the truth;
// Cargo.toml and package.json must carry the same value, and a tag build
// (argv[2], e.g. "v0.1.0" — empty on PR builds) must match it exactly:
// a release whose tag and embedded version disagree would confuse the
// in-app update check (S5a). Run locally with: node scripts/check-versions.mjs
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

// Paths resolve from the repo root (this script's parent-of-parent), so the
// guard works from any cwd — CI steps and local runs alike.
const root = join(dirname(fileURLToPath(import.meta.url)), '..');

const die = (msg) => {
  console.error(`check-versions: ${msg}`);
  process.exit(1);
};

// The [package] section's version — anchored to the section header so a
// TOML re-order (e.g. a [dependencies.x] table appearing before [package])
// can never match some other version line.
function cargoVersion(text) {
  let inPackage = false;
  for (const line of text.split(/\r?\n/)) {
    const section = line.match(/^\s*\[([^\]]+)\]/);
    if (section) {
      inPackage = section[1].trim() === 'package';
      continue;
    }
    if (inPackage) {
      const match = line.match(/^version\s*=\s*"([^"]+)"/);
      if (match) return match[1];
    }
  }
  return null;
}

const tauri = JSON.parse(readFileSync(join(root, 'src-tauri/tauri.conf.json'), 'utf8')).version;
const cargo = cargoVersion(readFileSync(join(root, 'src-tauri/Cargo.toml'), 'utf8'));
if (!cargo) die('no version line found in [package] of src-tauri/Cargo.toml');
const pkg = JSON.parse(readFileSync(join(root, 'package.json'), 'utf8')).version;

if (cargo !== tauri) {
  die(`src-tauri/Cargo.toml ${cargo} != tauri.conf.json ${tauri}`);
}
if (pkg !== tauri) {
  die(`package.json ${pkg} != tauri.conf.json ${tauri}`);
}
const tag = process.argv[2];
if (tag && tag !== `v${tauri}`) {
  die(`tag ${tag} != v${tauri} (tauri.conf.json is the version single source)`);
}
console.log(`check-versions: ok — all three at ${tauri}${tag ? `, tag ${tag}` : ''}`);

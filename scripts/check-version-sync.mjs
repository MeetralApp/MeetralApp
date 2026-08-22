// Version agreement gate: the three release manifests must agree,
// otherwise tag-triggered releases ship artifacts stamped with a stale version.
import { readFileSync } from "node:fs";

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const conf = JSON.parse(readFileSync("src-tauri/tauri.conf.json", "utf8"));
const cargo = readFileSync("src-tauri/Cargo.toml", "utf8");
const cargoVersion = cargo.match(/^\[package\][^[]*?^version\s*=\s*"([^"]+)"/ms)?.[1];

const versions = {
  "package.json": pkg.version,
  "tauri.conf.json": conf.version,
  "Cargo.toml": cargoVersion,
};

const unique = new Set(Object.values(versions));
if (!cargoVersion || unique.size !== 1) {
  console.error("Version mismatch across release manifests:");
  for (const [file, v] of Object.entries(versions)) {
    console.error(`  ${file}: ${v ?? "(not found)"}`);
  }
  process.exit(1);
}
console.log(`Version agreement OK: ${pkg.version}`);

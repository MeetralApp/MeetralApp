// Platform gate: the sparse MSIX only exists for Windows. Runs as part of
// `beforeBuildCommand`, which also executes on macOS CI runners.
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

if (process.platform !== "win32") {
  console.log("[build:sparse] skipped (non-Windows)");
  process.exit(0);
}

const script = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "sparse-identity",
  "Build-SparsePackage.ps1",
);
// Drop inherited PSModulePath so Windows PowerShell 5.1 loads its own modules
// (Certificate provider). pwsh 7 / GHA otherwise pollute the child env and
// Cert:\ fails to mount — see Build-SparsePackage.ps1 §2.
const env = { ...process.env };
delete env.PSModulePath;
const result = spawnSync(
  "powershell",
  ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", script],
  { stdio: "inherit", env },
);
process.exit(result.status ?? 1);

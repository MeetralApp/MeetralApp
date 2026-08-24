// Local mirror of GitHub CI PR checks (windows/macos jobs minus packaging).
// Run before push: `npm run check`
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const tauri = path.join(root, "src-tauri");
const isWin = process.platform === "win32";

/** @type {{ name: string, cmd: string, args: string[], cwd?: string }[]} */
const steps = [
  { name: "Version agreement", cmd: "npm", args: ["run", "check:version"] },
  { name: "Frontend lint", cmd: "npm", args: ["run", "lint"] },
  { name: "Frontend knip", cmd: "npm", args: ["run", "knip"] },
  { name: "Frontend build", cmd: "npm", args: ["run", "build"] },
  { name: "Frontend tests", cmd: "npm", args: ["run", "test"] },
  { name: "Rust tests", cmd: "cargo", args: ["test"], cwd: tauri },
  { name: "Rust fmt", cmd: "cargo", args: ["fmt", "--check"], cwd: tauri },
  {
    name: "Clippy",
    cmd: "cargo",
    args: ["clippy", "--all-targets", "--", "-D", "warnings"],
    cwd: tauri,
  },
];

function runStep(step, index, total) {
  console.log(`\n[${index}/${total}] ${step.name}`);
  const result = spawnSync(step.cmd, step.args, {
    cwd: step.cwd ?? root,
    stdio: "inherit",
    shell: isWin,
    env: process.env,
  });
  if (result.error) {
    console.error(`\nFailed to start: ${step.cmd} (${result.error.message})`);
    process.exit(1);
  }
  if (result.status !== 0) {
    console.error(
      `\ncheck failed at [${index}/${total}] ${step.name} (exit ${result.status ?? "?"})`,
    );
    process.exit(result.status ?? 1);
  }
}

console.log("Meetral local CI check (PR gate — no tauri build / audit)");
for (let i = 0; i < steps.length; i++) {
  runStep(steps[i], i + 1, steps.length);
}
console.log(`\nAll ${steps.length} checks passed.`);

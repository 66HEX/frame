#!/usr/bin/env node
/**
 * Cross-platform package manager runner for Tauri build commands.
 * Detects and uses the first available: bun, pnpm, yarn, or npm.
 * Usage: node scripts/run.cjs <dev|build>
 */
const { execSync, spawnSync } = require("child_process");

const command = process.argv[2];
if (!command) {
  console.error("Usage: node scripts/run.js <dev|build>");
  process.exit(1);
}

const packageManagers = ["bun", "pnpm", "yarn", "npm"];

function isAvailable(pm) {
  try {
    execSync(`${pm} --version`, { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
}

const pm = packageManagers.find(isAvailable);
if (!pm) {
  console.error("No package manager found. Please install npm, yarn, pnpm, or bun.");
  process.exit(1);
}

console.log(`Using ${pm} to run "${command}"...`);

const args = pm === "npm" ? ["run", command] : [command];
const result = spawnSync(pm, args, { stdio: "inherit", shell: true });

process.exit(result.status ?? 1);

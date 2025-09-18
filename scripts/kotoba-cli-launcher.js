#!/usr/bin/env node
// MERKLE: d5e6f7g8 (Kotoba CLI Launcher Simulation)
// This script simulates the behavior of the real Rust-based `kotoba` CLI.
// Its purpose is to read `project.kotoba`, parse the `dev` script,
// and launch the `@kotoba/web` Node.js process as a child.

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// In a real CLI, this would be a proper Jsonnet parser.
// For this simulation, we'll use a simple regex to extract the runner path.
function getConfigFromProjectFile() {
  const projectFile = fs.readFileSync(path.join(__dirname, '..', 'project.kotoba'), 'utf-8');
  // This is a brittle way to parse, but sufficient for a simulation.
  const runnerMatch = projectFile.match(/runner: '(@kotoba\/web)'/);
  if (!runnerMatch) {
    throw new Error("Could not find runner: '@kotoba/web' in project.kotoba");
  }
  return { runner: runnerMatch[1] };
}

function main() {
  try {
    const config = getConfigFromProjectFile();
    console.log('[Kotoba CLI Simulator] Found project config:', config);
    console.log('[Kotoba CLI Simulator] Spawning child process for @kotoba/web...');

    // The real CLI would resolve '@kotoba/web' to the correct path.
    const webCliPath = path.join(__dirname, '..', 'packages', 'web', 'dist', 'cli.js');

    // Get command-line arguments passed to this script, excluding 'node' and the script path itself.
    // e.g., ['dev', '--port', '8080']
    const args = process.argv.slice(2);

    // Spawn the child process
    const child = spawn('node', [webCliPath, ...args], {
      // Use `inherit` to make the child process's output appear in the parent's terminal.
      stdio: 'inherit',
    });

    child.on('error', (err) => {
      console.error('[Kotoba CLI Simulator] Failed to start child process.', err);
    });

    child.on('close', (code) => {
      console.log(`[Kotoba CLI Simulator] Child process exited with code ${code}`);
    });

  } catch (error) {
    console.error('[Kotoba CLI Simulator] An error occurred:', error.message);
    process.exit(1);
  }
}

main();

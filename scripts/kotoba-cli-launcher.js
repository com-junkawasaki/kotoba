#!/usr/bin/env node
// MERKLE: d5e6f7g8 (Kotoba CLI Launcher Simulation - Rust Server Version)
// This script simulates the behavior of the real Rust-based `kotoba` CLI.
// It now launches the compiled `kotoba-server` Rust binary as a child process.

const { spawn } = require('child_process');
const path = require('path');

// In a real CLI, this would be a proper Jsonnet parser for project.kotoba.
// For this simulation, we'll hardcode the necessary paths.

function main() {
  const args = process.argv.slice(2); // e.g., ['web', 'dev', '--port', '8080']

  if (args[0] !== 'web' || args[1] !== 'dev') {
    console.error("Usage: kotoba web dev [--port <number>] [--cwd <path>]");
    process.exit(1);
  }

  // Determine the working directory for the server.
  // The user can override it with --cwd.
  let CWD = path.join(__dirname, '..', 'examples', 'web-app-example');
  const cwdIndex = args.indexOf('--cwd');
  if (cwdIndex > -1 && args[cwdIndex + 1]) {
    CWD = path.resolve(args[cwdIndex + 1]);
  }
  
  console.log(`[Kotoba CLI Simulator] Starting kotoba-server in: ${CWD}`);

  // Path to the compiled Rust binary
  const serverBinaryPath = path.join(__dirname, '..', 'target', 'debug', 'kotoba-server');

  // We don't need to pass the `web dev` part to the binary, just the options.
  const serverArgs = args.slice(2).filter(arg => !arg.startsWith('--cwd'));

  try {
    const child = spawn(serverBinaryPath, serverArgs, {
      // Set the correct working directory so the server can find `src/app`
      cwd: CWD,
      stdio: 'inherit',
    });

    child.on('error', (err) => {
      console.error('[Kotoba CLI Simulator] Failed to start kotoba-server process.', err);
      console.error(`Please ensure you have compiled the server with 'cargo build -p kotoba-server'`);
    });

    child.on('close', (code) => {
      console.log(`[Kotoba CLI Simulator] kotoba-server process exited with code ${code}`);
    });

  } catch (error) {
    console.error('[Kotoba CLI Simulator] An error occurred:', error.message);
    process.exit(1);
  }
}

main();

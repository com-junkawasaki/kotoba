# Jisho - Kotoba Package Manager

Jisho is the official package manager for the Kotoba ecosystem. It is designed to be a secure, efficient, and modern package manager, inspired by Deno, NPM, and Cargo.

## Features

- **Mixed-Package Support**: Manage dependencies from Kotoba registries, NPM, Git repositories, and direct URLs seamlessly.
- **Content-Addressable Storage**: All packages are identified by a Content ID (CID) based on their content, ensuring integrity and preventing tampering.
- **Global Cache**: Packages are stored in a global cache (`~/.kotoba/cache`), deduplicated across projects, saving disk space and speeding up installations.
- **Lockfile**: A `kotoba.lock` file is generated to ensure deterministic and reproducible builds by pinning the exact versions and CIDs of all dependencies.
- **Security**: The package manager verifies the integrity of packages using CIDs. If a mismatch is detected, it will attempt to reinstall the package to ensure the project is secure.

## Getting Started

To manage dependencies for your Kotoba project, create a `kotoba.toml` file:

```toml
[package]
name = "my-kotoba-project"
version = "0.1.0"

[dependencies]
react = { version = "18.2.0", source = "npm" }
lodash = { version = "4.17.21", source = "npm" }
```

Then, run the install command:

```bash
kotoba install
```

This will resolve all dependencies, download them, store them in the global cache, create a `node_modules` directory with the package contents, and generate a `kotoba.lock` file.
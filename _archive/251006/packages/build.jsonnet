// packages/build.jsonnet
// MERKLE: a1b2c3d4 (build configuration for @kotoba/kotobajs)
//
// This file defines the build process for the @kotoba/kotobajs package,
// integrating it into the larger kotoba build system managed by dag.jsonnet.

local ts = import 'lib/typescript.libsonnet'; // Assuming a common typescript build library

{
  // Package definition for @kotoba/kotobajs
  kotobajs: ts.makePackage({
    name: '@kotoba/kotobajs',
    version: '0.1.0',
    path: './kotobajs',
    entrypoint: 'src/index.ts',
    cli_entrypoint: 'src/cli.ts',
    dependencies: [
      // No external dependencies yet
    ],
    tsconfig: {
      compilerOptions: {
        target: 'ES2020',
        module: 'CommonJS',
        strict: true,
        esModuleInterop: true,
        skipLibCheck: true,
        forceConsistentCasingInFileNames: true,
        outDir: './dist',
        declaration: true,
      },
      include: ['src/**/*'],
    },
    package_json_extra: {
      description: 'Isomorphic JS/TS SDK for Kotoba',
      author: 'Your Name',
      license: 'Apache-2.0',
    },
  }),
}

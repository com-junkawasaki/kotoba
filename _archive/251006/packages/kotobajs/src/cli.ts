#!/usr/bin/env node

// MERKLE: a8f5d7b3 (CLI entrypoint)
import { Command } from 'commander';
import { generate } from './generator';

const program = new Command();

program
  .name('kotobajs')
  .description('CLI for the @kotoba/kotobajs SDK')
  .version('0.1.0');

program
  .command('generate')
  .description('Generate models and schemas from the kotoba schema registry')
  .requiredOption('-s, --schemas <path>', 'Path to the schema registry directory')
  .requiredOption('-o, --output <path>', 'Path to the output directory for generated files')
  .action(async (options) => {
    try {
      console.log('🚀 Starting code generation...');
      // MERKLE: Codegen.run
      await generate({
        schemaPath: options.schemas,
        outputPath: options.output,
      });
      console.log(`✅ Code generation complete! Files written to ${options.output}`);
    } catch (error) {
      console.error('❌ An error occurred during code generation:');
      console.error(error);
      process.exit(1);
    }
  });

program.parse(process.argv);

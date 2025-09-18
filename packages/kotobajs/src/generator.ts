// MERKLE: b2c9e8f1 (Code generator logic)
import fs from 'fs/promises';
import path from 'path';

export interface GenerateOptions {
  schemaPath: string;
  outputPath: string;
}

export async function generate(options: GenerateOptions): Promise<void> {
  const { schemaPath, outputPath } = options;

  console.log(`Reading schemas from: ${path.resolve(schemaPath)}`);
  console.log(`Writing generated files to: ${path.resolve(outputPath)}`);

  // 1. Ensure output directory exists
  await fs.mkdir(outputPath, { recursive: true });

  // 2. Read schema directory
  const schemaFiles = await fs.readdir(schemaPath);

  if (schemaFiles.length === 0) {
    console.warn('⚠️ No schema files found in the specified directory.');
    return;
  }

  // TODO: Implement the actual code generation logic for each schema file.
  // For now, we'll just log the files found.
  for (const file of schemaFiles) {
    if (file.endsWith('.json')) {
      console.log(`   - Found schema: ${file}`);
    }
  }
}

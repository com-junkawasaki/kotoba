import fastify from 'fastify';
import path from 'path';
import fs from 'fs/promises';
import { glob } from 'glob';
import { RouteModule } from './types';

interface DevServerOptions {
  port: number;
}

const HTTP_METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'] as const;

async function registerRoutes(app: ReturnType<typeof fastify>, appDir: string) {
  const routeFiles = await glob(`${appDir}/**/route.ts`);

  for (const file of routeFiles) {
    const routePath = path.relative(appDir, path.dirname(file))
      .replace(/\[([^\]]+)\]/g, ':$1') // Convert [id] to :id for fastify
      .replace(/\\/g, '/'); // Convert windows paths
    const url = `/${routePath}`;

    console.log(`[KotobaWeb] Registering route: ${url} from ${file}`);

    try {
      const module: RouteModule = await import(path.resolve(file));

      for (const method of HTTP_METHODS) {
        if (module[method]) {
          app[method.toLowerCase() as 'get'](url, async (request, reply) => {
            
            // --- Basic Validation (can be expanded) ---
            let params, body, query;
            if (module.params) {
              const result = module.params.parse(request.params);
              if (!result) return reply.status(400).send({ error: 'Invalid params' });
              params = result;
            }
            if (module.body) {
              const result = module.body.parse(request.body);
              if (!result) return reply.status(400).send({ error: 'Invalid body' });
              body = result;
            }
            // TODO: Add query validation

            const ctx = { params, body, query, request, reply };
            const handlerResult = await module[method]!(ctx);
            
            // TODO: Add response validation
            return handlerResult;
          });
        }
      }
    } catch (e) {
      console.error(`[KotobaWeb] Failed to load route ${file}:`, e);
    }
  }
}

export async function startDevServer(options: DevServerOptions) {
  const app = fastify({ logger: true });
  const appDir = path.join(process.cwd(), 'src', 'app');

  console.log(`[KotobaWeb] Starting server... scanning routes in ${appDir}`);
  await registerRoutes(app, appDir);

  try {
    await app.listen({ port: options.port });
  } catch (err) {
    app.log.error(err);
    process.exit(1);
  }
}

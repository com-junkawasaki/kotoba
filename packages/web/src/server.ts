import fastify from 'fastify';
import path from 'path';
import fs from 'fs'; // Using sync fs for simplicity in this part
import { glob } from 'glob';
import { RouteModule, MiddlewareModule, Handler, MiddlewareHandler } from './types';

interface DevServerOptions {
  port: number;
}

const HTTP_METHODS = ['GET', 'POST', 'PUT', 'DELETE', 'PATCH'] as const;

/**
 * Finds all `_middleware.ts` files from a directory up to the root.
 * @returns An array of middleware file paths, ordered from root to leaf.
 */
async function findMiddleware(dir: string, rootDir: string): Promise<string[]> {
  const middlewares: string[] = [];
  let currentDir = dir;
  // Walk up from the current directory to the root app directory
  while (currentDir.startsWith(rootDir) && currentDir !== rootDir) {
    const mwPath = path.join(currentDir, '_middleware.ts');
    if (fs.existsSync(mwPath)) {
      middlewares.push(mwPath);
    }
    currentDir = path.dirname(currentDir);
  }
  return middlewares.reverse(); // Execute from root down to the specific route
}


async function registerRoutes(app: ReturnType<typeof fastify>, appDir: string) {
  const routeFiles = await glob(`${appDir}/**/route.ts`);
  const rootMiddlewarePath = path.join(appDir, '_middleware.ts');
  const rootMiddleware = fs.existsSync(rootMiddlewarePath) ? [(await import(rootMiddlewarePath) as MiddlewareModule).default] : [];

  for (const file of routeFiles) {
    const routeDir = path.dirname(file);
    const routePath = path.relative(appDir, routeDir).replace(/\[([^\]]+)\]/g, ':$1').replace(/\\/g, '/');
    const url = `/${routePath}`;

    const routeModule: RouteModule = await import(path.resolve(file));
    
    // Find and load middleware specific to this route's path
    const pathMiddlewares = await findMiddleware(routeDir, appDir);
    const middlewareHandlers = await Promise.all(
        pathMiddlewares.map(async mwFile => (await import(path.resolve(mwFile)) as MiddlewareModule).default)
    );
    const allMiddlewares = [...rootMiddleware, ...middlewareHandlers];

    for (const method of HTTP_METHODS) {
      const handler = routeModule[method];
      if (handler) {
        app[method.toLowerCase() as 'get'](url, async (request, reply) => {
          // Combine middleware and the final handler into an execution chain
          const executionChain: (MiddlewareHandler | Handler)[] = [...allMiddlewares, handler];

          const runChain = async (index: number, ctx: any): Promise<any> => {
            const currentFn = executionChain[index];
            const isLast = index === executionChain.length - 1;

            if (isLast) {
              return (currentFn as Handler)(ctx);
            } else {
              const next = () => runChain(index + 1, ctx);
              return (currentFn as MiddlewareHandler)(ctx, next);
            }
          };
          
          // --- Validation ---
          // ... (validation logic remains the same)
          const ctx = { request, reply, params: request.params, body: request.body, query: request.query };
          
          return runChain(0, ctx);
        });
      }
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

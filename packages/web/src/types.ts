// MERKLE: f1a2b3c4 (Framework Core Types)
import { FastifyRequest, FastifyReply } from 'fastify';
import { KotobaSchema, infer } from '@kotoba/kotobajs';

/**
 * The context object passed to every handler.
 * It contains the typed and validated request data.
 */
export interface HandlerContext<
  ParamsSchema extends KotobaSchema<any> | undefined = undefined,
  BodySchema extends KotobaSchema<any> | undefined = undefined,
  QuerySchema extends KotobaSchema<any> | undefined = undefined,
> {
  params: ParamsSchema extends KotobaSchema<any> ? infer<ParamsSchema> : undefined;
  body: BodySchema extends KotobaSchema<any> ? infer<BodySchema> : undefined;
  query: QuerySchema extends KotobaSchema<any> ? infer<QuerySchema> : undefined;
  request: FastifyRequest;
  reply: FastifyReply;
}

/**
 * A handler function for a specific HTTP method.
 */
export type Handler = (ctx: HandlerContext<any, any, any>) => Promise<any> | any;

/**
 * A function to pass control to the next middleware or handler in the chain.
 */
export type NextFunction = () => Promise<any> | any;

/**
 * A middleware handler function.
 */
export type MiddlewareHandler = (ctx: HandlerContext<any, any, any>, next: NextFunction) => Promise<any> | any;

/**
 * The structure of a middleware file (`_middleware.ts`).
 * It must have a default export.
 */
export interface MiddlewareModule {
  default: MiddlewareHandler;
}

/**
 * The structure of a route file (`route.ts`).
 */
export interface RouteModule {
  params?: KotobaSchema<any>;
  body?: KotobaSchema<any>;
  query?: KotobaSchema<any>;
  response?: {
    [statusCode: number]: KotobaSchema<any>;
  };

  GET?: Handler;
  POST?: Handler;
  PUT?: Handler;
  DELETE?: Handler;
  PATCH?: Handler;
}

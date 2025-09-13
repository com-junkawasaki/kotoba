// API Route - Users API
// REST APIエンドポイントの定義

local api = import '../../lib/api.libsonnet';
local types = import '../../lib/types.libsonnet';

{
  // GET /api/users - ユーザーリスト取得
  get: {
    path: '/api/users',
    method: 'GET',
    handler: {
      function_name: 'getUsers',
      is_async: true,
      timeout_ms: 5000,
    },
    middlewares: ['auth', 'cors'],
    response_format: 'JSON',
    parameters: {
      query_params: [
        {
          name: 'limit',
          param_type: 'Integer',
          required: false,
          default_value: 10,
          validation: {
            min_value: 1,
            max_value: 100,
          },
        },
        {
          name: 'offset',
          param_type: 'Integer',
          required: false,
          default_value: 0,
          validation: {
            min_value: 0,
          },
        },
        {
          name: 'search',
          param_type: 'String',
          required: false,
          validation: {
            max_length: 255,
          },
        },
      ],
    },
    metadata: {
      description: 'Get users list with pagination and search',
      summary: 'Users API',
      tags: ['users'],
      rate_limit: {
        requests: 100,
        window_seconds: 60,
        strategy: 'SlidingWindow',
      },
      cache: {
        ttl_seconds: 300,
        vary_by: ['user_id'],
      },
    },
  },

  // POST /api/users - 新規ユーザー作成
  post: {
    path: '/api/users',
    method: 'POST',
    handler: {
      function_name: 'createUser',
      is_async: true,
      timeout_ms: 10000,
    },
    middlewares: ['auth', 'cors', 'validation'],
    response_format: 'JSON',
    parameters: {
      body_params: {
        content_type: 'application/json',
        schema: {
          type: 'object',
          required: ['email', 'password'],
          properties: {
            email: {
              type: 'string',
              format: 'email',
              maxLength: 255,
            },
            password: {
              type: 'string',
              minLength: 8,
              maxLength: 128,
            },
            name: {
              type: 'string',
              maxLength: 100,
            },
            role: {
              type: 'string',
              enum: ['user', 'admin'],
              default: 'user',
            },
          },
        },
      },
    },
    metadata: {
      description: 'Create a new user',
      summary: 'Create User',
      tags: ['users'],
      rate_limit: {
        requests: 10,
        window_seconds: 60,
        strategy: 'FixedWindow',
      },
    },
  },

  // GET /api/users/[id] - ユーザー詳細取得
  getById: {
    path: '/api/users/[id]',
    method: 'GET',
    handler: {
      function_name: 'getUserById',
      is_async: true,
      timeout_ms: 3000,
    },
    middlewares: ['auth', 'cors'],
    response_format: 'JSON',
    parameters: {
      path_params: [
        {
          name: 'id',
          param_type: 'String',
          required: true,
          validation: {
            pattern: '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$',
          },
        },
      ],
    },
    metadata: {
      description: 'Get user by ID',
      summary: 'Get User',
      tags: ['users'],
      cache: {
        ttl_seconds: 600,
        vary_by: ['id'],
      },
    },
  },

  // PUT /api/users/[id] - ユーザー更新
  putById: {
    path: '/api/users/[id]',
    method: 'PUT',
    handler: {
      function_name: 'updateUser',
      is_async: true,
      timeout_ms: 5000,
    },
    middlewares: ['auth', 'cors', 'ownership'],
    response_format: 'JSON',
    parameters: {
      path_params: [
        {
          name: 'id',
          param_type: 'String',
          required: true,
          validation: {
            pattern: '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$',
          },
        },
      ],
      body_params: {
        content_type: 'application/json',
        schema: {
          type: 'object',
          properties: {
            name: {
              type: 'string',
              maxLength: 100,
            },
            email: {
              type: 'string',
              format: 'email',
              maxLength: 255,
            },
            role: {
              type: 'string',
              enum: ['user', 'admin'],
            },
          },
        },
      },
    },
    metadata: {
      description: 'Update user information',
      summary: 'Update User',
      tags: ['users'],
    },
  },

  // DELETE /api/users/[id] - ユーザー削除
  deleteById: {
    path: '/api/users/[id]',
    method: 'DELETE',
    handler: {
      function_name: 'deleteUser',
      is_async: true,
      timeout_ms: 3000,
    },
    middlewares: ['auth', 'cors', 'ownership'],
    response_format: 'JSON',
    parameters: {
      path_params: [
        {
          name: 'id',
          param_type: 'String',
          required: true,
          validation: {
            pattern: '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$',
          },
        },
      ],
    },
    metadata: {
      description: 'Delete user by ID',
      summary: 'Delete User',
      tags: ['users'],
    },
  },
}

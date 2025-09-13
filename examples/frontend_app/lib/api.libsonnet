// API Configuration Library
// API設定用のJsonnetライブラリ

{
  // HTTPメソッド
  methods: {
    GET: 'GET',
    POST: 'POST',
    PUT: 'PUT',
    DELETE: 'DELETE',
    PATCH: 'PATCH',
    HEAD: 'HEAD',
    OPTIONS: 'OPTIONS',
  },

  // レスポンスフォーマット
  formats: {
    JSON: 'JSON',
    XML: 'XML',
    HTML: 'HTML',
    Text: 'Text',
    Binary: 'Binary',
    GraphQL: 'GraphQL',
  },

  // パラメータタイプ
  param_types: {
    String: 'String',
    Integer: 'Integer',
    Float: 'Float',
    Boolean: 'Boolean',
    Array: 'Array',
    Object: 'Object',
    File: 'File',
    Date: 'Date',
    DateTime: 'DateTime',
  },

  // レート制限戦略
  rate_limit_strategies: {
    FixedWindow: 'FixedWindow',
    SlidingWindow: 'SlidingWindow',
    TokenBucket: 'TokenBucket',
  },

  // キャッシュ戦略
  cache_strategies: {
    LRU: 'LRU',
    LFU: 'LFU',
    TimeBased: 'TimeBased',
    SizeBased: 'SizeBased',
  },

  // ルート定義ヘルパー
  route:: {
    path: error 'path must be specified',
    method: error 'method must be specified',
    handler: error 'handler must be specified',
    middlewares: [],
    response_format: $.formats.JSON,
    parameters: {
      path_params: [],
      query_params: [],
      body_params: null,
      headers: [],
    },
    metadata: {
      description: null,
      summary: null,
      tags: [],
      deprecated: false,
      rate_limit: null,
      cache: null,
    },
  },

  // ハンドラー定義ヘルパー
  handler:: {
    function_name: error 'function_name must be specified',
    component: null,
    is_async: true,
    timeout_ms: null,
  },

  // パラメータ定義ヘルパー
  param:: {
    name: error 'name must be specified',
    param_type: error 'param_type must be specified',
    required: false,
    default_value: null,
    validation: null,
  },

  // 便利なAPI定義関数
  rest: {
    // RESTful APIの基本的なCRUD操作
    crud(resource_name):: {
      local base_path = '/api/' + resource_name,
      local id_path = base_path + '/[id]',

      // GET /api/resource - リソース一覧取得
      list: $.route {
        path: base_path,
        method: $.methods.GET,
        handler: $.handler {
          function_name: 'get' + std.asciiUpper(resource_name) + 'List',
        },
        metadata: {
          description: 'Get list of ' + resource_name,
          tags: [resource_name],
        },
      },

      // POST /api/resource - 新規リソース作成
      create: $.route {
        path: base_path,
        method: $.methods.POST,
        handler: $.handler {
          function_name: 'create' + std.asciiUpper(resource_name),
        },
        parameters: {
          body_params: {
            content_type: 'application/json',
            schema: {},  // JSON Schemaをここに定義
          },
        },
        metadata: {
          description: 'Create a new ' + resource_name,
          tags: [resource_name],
        },
      },

      // GET /api/resource/[id] - リソース詳細取得
      get: $.route {
        path: id_path,
        method: $.methods.GET,
        handler: $.handler {
          function_name: 'get' + std.asciiUpper(resource_name),
        },
        parameters: {
          path_params: [
            $.param {
              name: 'id',
              param_type: $.param_types.String,
              required: true,
            },
          ],
        },
        metadata: {
          description: 'Get ' + resource_name + ' by ID',
          tags: [resource_name],
        },
      },

      // PUT /api/resource/[id] - リソース更新
      update: $.route {
        path: id_path,
        method: $.methods.PUT,
        handler: $.handler {
          function_name: 'update' + std.asciiUpper(resource_name),
        },
        parameters: {
          path_params: [
            $.param {
              name: 'id',
              param_type: $.param_types.String,
              required: true,
            },
          ],
          body_params: {
            content_type: 'application/json',
            schema: {},  // JSON Schemaをここに定義
          },
        },
        metadata: {
          description: 'Update ' + resource_name,
          tags: [resource_name],
        },
      },

      // DELETE /api/resource/[id] - リソース削除
      delete: $.route {
        path: id_path,
        method: $.methods.DELETE,
        handler: $.handler {
          function_name: 'delete' + std.asciiUpper(resource_name),
        },
        parameters: {
          path_params: [
            $.param {
              name: 'id',
              param_type: $.param_types.String,
              required: true,
            },
          ],
        },
        metadata: {
          description: 'Delete ' + resource_name,
          tags: [resource_name],
        },
      },
    },

    // ユーザー管理API
    users: $.rest.crud('users') {
      // ログインAPI
      login: $.route {
        path: '/api/auth/login',
        method: $.methods.POST,
        handler: $.handler {
          function_name: 'loginUser',
        },
        middlewares: ['rate_limiting'],
        parameters: {
          body_params: {
            content_type: 'application/json',
            schema: {
              type: 'object',
              required: ['email', 'password'],
              properties: {
                email: { type: 'string', format: 'email' },
                password: { type: 'string', minLength: 8 },
                remember_me: { type: 'boolean', default: false },
              },
            },
          },
        },
        metadata: {
          description: 'User login',
          tags: ['auth'],
        },
      },

      // ログアウトAPI
      logout: $.route {
        path: '/api/auth/logout',
        method: $.methods.POST,
        handler: $.handler {
          function_name: 'logoutUser',
        },
        middlewares: ['auth'],
        metadata: {
          description: 'User logout',
          tags: ['auth'],
        },
      },

      // 現在のユーザー情報取得
      me: $.route {
        path: '/api/auth/me',
        method: $.methods.GET,
        handler: $.handler {
          function_name: 'getCurrentUser',
        },
        middlewares: ['auth'],
        metadata: {
          description: 'Get current user information',
          tags: ['auth'],
        },
      },
    },

    // ファイルアップロードAPI
    upload: $.route {
      path: '/api/upload',
      method: $.methods.POST,
      handler: $.handler {
        function_name: 'uploadFile',
        timeout_ms: 300000,  // 5分
      },
      middlewares: ['auth', 'file_validation'],
      parameters: {
        body_params: {
          content_type: 'multipart/form-data',
          schema: {
            type: 'object',
            properties: {
              file: {
                type: 'string',
                format: 'binary',
                maxSize: 10485760,  // 10MB
              },
              category: {
                type: 'string',
                enum: ['avatar', 'document', 'image'],
              },
            },
          },
        },
      },
      metadata: {
        description: 'Upload file',
        tags: ['upload'],
        rate_limit: {
          requests: 10,
          window_seconds: 60,
          strategy: $.rate_limit_strategies.FixedWindow,
        },
      },
    },
  },

  // GraphQL API定義
  graphql: {
    schema: {
      query: 'Query',
      mutation: 'Mutation',
      subscription: 'Subscription',
      types: [],
    },

    // GraphQLタイプ定義ヘルパー
    type:: {
      name: error 'name must be specified',
      kind: 'Object',
      fields: [],
    },

    field:: {
      name: error 'name must be specified',
      field_type: error 'field_type must be specified',
      args: [],
      resolver: null,
    },

    // 基本的なCRUD GraphQLスキーマ
    crud_schema(resource_name):: {
      local Type = std.asciiUpper(resource_name),
      local Input = Type + 'Input',
      local Filter = Type + 'Filter',

      types: [
        {
          name: Type,
          fields: [
            { name: 'id', field_type: 'ID!' },
            { name: 'createdAt', field_type: 'DateTime!' },
            { name: 'updatedAt', field_type: 'DateTime!' },
          ],
        },
        {
          name: Input,
          kind: 'InputObject',
          fields: [
            { name: 'id', field_type: 'ID' },
          ],
        },
        {
          name: Filter,
          kind: 'InputObject',
          fields: [
            { name: 'id', field_type: 'ID' },
            { name: 'limit', field_type: 'Int' },
            { name: 'offset', field_type: 'Int' },
          ],
        },
      ],

      query_fields: [
        {
          name: std.asciiLower(resource_name),
          field_type: Type,
          args: [{ name: 'id', arg_type: 'ID!' }],
        },
        {
          name: std.asciiLower(resource_name) + 's',
          field_type: '[' + Type + ']!',
          args: [{ name: 'filter', arg_type: Filter }],
        },
      ],

      mutation_fields: [
        {
          name: 'create' + Type,
          field_type: Type,
          args: [{ name: 'input', arg_type: Input + '!' }],
        },
        {
          name: 'update' + Type,
          field_type: Type,
          args: [
            { name: 'id', arg_type: 'ID!' },
            { name: 'input', arg_type: Input + '!' },
          ],
        },
        {
          name: 'delete' + Type,
          field_type: 'Boolean!',
          args: [{ name: 'id', arg_type: 'ID!' }],
        },
      ],
    },
  },

  // WebSocket API定義
  websocket: {
    connection:: {
      path: error 'path must be specified',
      handler: error 'handler must be specified',
      protocols: [],
      heartbeat_interval: 30000,
    },

    handler:: {
      on_connect: error 'on_connect must be specified',
      on_message: error 'on_message must be specified',
      on_disconnect: 'handleDisconnect',
      on_error: 'handleError',
    },

    // チャットWebSocket
    chat: $.connection {
      path: '/ws/chat',
      handler: $.handler {
        on_connect: 'handleChatConnect',
        on_message: 'handleChatMessage',
        on_disconnect: 'handleChatDisconnect',
        on_error: 'handleChatError',
      },
      protocols: ['chat-protocol-v1'],
    },

    // リアルタイム通知
    notifications: $.connection {
      path: '/ws/notifications',
      handler: $.handler {
        on_connect: 'handleNotificationConnect',
        on_message: 'handleNotificationMessage',
        on_disconnect: 'handleNotificationDisconnect',
        on_error: 'handleNotificationError',
      },
      protocols: ['notification-protocol-v1'],
    },
  },

  // バリデーションルール
  validation: {
    email: {
      type: 'string',
      format: 'email',
      maxLength: 255,
    },

    password: {
      type: 'string',
      minLength: 8,
      maxLength: 128,
      pattern: '^(?=.*[a-z])(?=.*[A-Z])(?=.*\\d).+$',
    },

    uuid: {
      type: 'string',
      pattern: '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$',
    },

    slug: {
      type: 'string',
      pattern: '^[a-z0-9]+(?:-[a-z0-9]+)*$',
      maxLength: 100,
    },

    url: {
      type: 'string',
      format: 'uri',
      maxLength: 2048,
    },
  },

  // レスポンススキーマ
  responses: {
    success: {
      type: 'object',
      properties: {
        success: { type: 'boolean', enum: [true] },
        data: { type: 'object' },
        message: { type: 'string' },
      },
      required: ['success'],
    },

    error: {
      type: 'object',
      properties: {
        success: { type: 'boolean', enum: [false] },
        error: {
          type: 'object',
          properties: {
            code: { type: 'string' },
            message: { type: 'string' },
            details: { type: 'object' },
          },
          required: ['code', 'message'],
        },
      },
      required: ['success', 'error'],
    },

    paginated: {
      type: 'object',
      properties: {
        data: { type: 'array' },
        pagination: {
          type: 'object',
          properties: {
            page: { type: 'integer', minimum: 1 },
            limit: { type: 'integer', minimum: 1, maximum: 100 },
            total: { type: 'integer', minimum: 0 },
            totalPages: { type: 'integer', minimum: 0 },
          },
          required: ['page', 'limit', 'total', 'totalPages'],
        },
      },
      required: ['data', 'pagination'],
    },
  },
}

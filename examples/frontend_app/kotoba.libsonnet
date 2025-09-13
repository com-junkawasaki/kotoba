// Kotoba Web Framework Configuration
// Jsonnetベースのメイン設定ファイル

local db = import './lib/database.libsonnet';
local auth = import './lib/auth.libsonnet';
local api = import './lib/api.libsonnet';

{
  // サーバー設定
  server: {
    host: 'localhost',
    port: 3000,
    workers: 4,
    max_connections: 1000,
    tls: null,  // TLS設定が必要な場合はここに追加
  },

  // データベース設定
  database: db.postgres {
    connection_string: 'postgresql://user:password@localhost:5432/kotoba_app',
    models: [
      db.model {
        name: 'User',
        table_name: 'users',
        fields: [
          db.field {
            name: 'id',
            field_type: db.types.UUID,
            primary_key: true,
            nullable: false,
          },
          db.field {
            name: 'email',
            field_type: db.types.String { max_length: 255 },
            unique: true,
            nullable: false,
          },
          db.field {
            name: 'password_hash',
            field_type: db.types.String { max_length: 255 },
            nullable: false,
          },
          db.field {
            name: 'name',
            field_type: db.types.String { max_length: 100 },
            nullable: true,
          },
          db.field {
            name: 'role',
            field_type: db.types.String { max_length: 50 },
            nullable: false,
            default_value: 'user',
          },
          db.field {
            name: 'created_at',
            field_type: db.types.DateTime,
            nullable: false,
            default_value: 'NOW()',
          },
          db.field {
            name: 'updated_at',
            field_type: db.types.DateTime,
            nullable: false,
            default_value: 'NOW()',
          },
        ],
        relationships: [
          db.relationship {
            name: 'posts',
            target_model: 'Post',
            relationship_type: 'OneToMany',
            foreign_key: 'user_id',
            on_delete: 'Cascade',
          },
        ],
        indexes: [
          db.index {
            name: 'idx_users_email',
            fields: ['email'],
            unique: true,
          },
          db.index {
            name: 'idx_users_created_at',
            fields: ['created_at'],
          },
        ],
      },

      db.model {
        name: 'Post',
        table_name: 'posts',
        fields: [
          db.field {
            name: 'id',
            field_type: db.types.UUID,
            primary_key: true,
            nullable: false,
          },
          db.field {
            name: 'title',
            field_type: db.types.String { max_length: 255 },
            nullable: false,
          },
          db.field {
            name: 'content',
            field_type: db.types.Text,
            nullable: false,
          },
          db.field {
            name: 'slug',
            field_type: db.types.String { max_length: 255 },
            unique: true,
            nullable: false,
          },
          db.field {
            name: 'user_id',
            field_type: db.types.UUID,
            nullable: false,
          },
          db.field {
            name: 'published',
            field_type: db.types.Boolean,
            nullable: false,
            default_value: false,
          },
          db.field {
            name: 'published_at',
            field_type: db.types.DateTime,
            nullable: true,
          },
          db.field {
            name: 'created_at',
            field_type: db.types.DateTime,
            nullable: false,
            default_value: 'NOW()',
          },
          db.field {
            name: 'updated_at',
            field_type: db.types.DateTime,
            nullable: false,
            default_value: 'NOW()',
          },
        ],
        relationships: [
          db.relationship {
            name: 'author',
            target_model: 'User',
            relationship_type: 'ManyToOne',
            foreign_key: 'user_id',
            on_delete: 'Restrict',
          },
        ],
        indexes: [
          db.index {
            name: 'idx_posts_slug',
            fields: ['slug'],
            unique: true,
          },
          db.index {
            name: 'idx_posts_user_id',
            fields: ['user_id'],
          },
          db.index {
            name: 'idx_posts_published',
            fields: ['published'],
          },
        ],
      },
    ],

    migrations: [
      db.migration {
        version: '001',
        description: 'Create users table',
        up_sql: |||
          CREATE TABLE users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            name VARCHAR(100),
            role VARCHAR(50) NOT NULL DEFAULT 'user',
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
          );

          CREATE INDEX idx_users_email ON users(email);
          CREATE INDEX idx_users_created_at ON users(created_at);
        |||,

        down_sql: |||
          DROP TABLE users;
        |||,

        dependencies: [],
      },

      db.migration {
        version: '002',
        description: 'Create posts table',
        up_sql: |||
          CREATE TABLE posts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            title VARCHAR(255) NOT NULL,
            content TEXT NOT NULL,
            slug VARCHAR(255) UNIQUE NOT NULL,
            user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
            published BOOLEAN NOT NULL DEFAULT FALSE,
            published_at TIMESTAMP WITH TIME ZONE,
            created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
          );

          CREATE INDEX idx_posts_slug ON posts(slug);
          CREATE INDEX idx_posts_user_id ON posts(user_id);
          CREATE INDEX idx_posts_published ON posts(published);
        |||,

        down_sql: |||
          DROP TABLE posts;
        |||,

        dependencies: ['001'],
      },
    ],
  },

  // 認証設定
  authentication: auth.jwt {
    provider: 'Local',
    jwt_secret: 'your-secret-key-change-in-production',
    session_timeout: 3600,  // 1 hour
    config: {
      bcrypt_rounds: 12,
      password_min_length: 8,
      require_email_verification: true,
    },
  },

  // セッション設定
  session: {
    store: 'Redis',
    cookie_name: 'kotoba_session',
    secure: false,  // 本番環境ではtrue
    http_only: true,
    same_site: 'Lax',
  },

  // ミドルウェア設定
  middlewares: [
    {
      name: 'cors',
      middleware_type: 'CORS',
      order: 1,
      config: {
        allowed_origins: ['http://localhost:3000', 'http://localhost:3001'],
        allowed_methods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
        allowed_headers: ['Content-Type', 'Authorization', 'X-Requested-With'],
        allow_credentials: true,
        max_age: 86400,
      },
    },

    {
      name: 'compression',
      middleware_type: 'Compression',
      order: 2,
      config: {
        level: 6,
        algorithms: ['gzip', 'deflate'],
        min_length: 1024,
      },
    },

    {
      name: 'rate_limiting',
      middleware_type: 'RateLimiting',
      order: 3,
      config: {
        requests_per_minute: 100,
        burst_size: 20,
        strategy: 'TokenBucket',
      },
    },

    {
      name: 'logging',
      middleware_type: 'Logging',
      order: 4,
      config: {
        level: 'Info',
        format: 'json',
        include_headers: true,
        exclude_paths: ['/health', '/favicon.ico'],
      },
    },

    {
      name: 'auth',
      middleware_type: 'Authentication',
      order: 5,
      config: {
        exclude_paths: ['/login', '/register', '/api/auth/*', '/health'],
        redirect_unauthenticated: '/login',
      },
    },

    {
      name: 'csrf',
      middleware_type: 'CSRF',
      order: 6,
      config: {
        cookie_name: 'csrf_token',
        header_name: 'X-CSRF-Token',
        exclude_paths: ['/api/*'],
      },
    },
  ],

  // 静的ファイル設定
  static_files: [
    {
      route: '/static',
      directory: './public',
      cache_control: 'public, max-age=31536000',
      gzip: true,
    },
    {
      route: '/assets',
      directory: './dist/assets',
      cache_control: 'public, max-age=31536000, immutable',
      gzip: true,
    },
  ],

  // APIルート設定（ファイルから読み込み）
  api_routes: [
    import 'app/api/users.libsonnet',
  ],

  // WebSocket設定
  web_sockets: [
    {
      path: '/ws/chat',
      handler: {
        on_connect: 'handleChatConnect',
        on_message: 'handleChatMessage',
        on_disconnect: 'handleChatDisconnect',
        on_error: 'handleChatError',
      },
      protocols: ['chat-protocol-v1'],
      heartbeat_interval: 30000,
    },
  ],

  // GraphQL設定（オプション）
  graph_ql: null,

  // 開発環境設定
  development: {
    hot_reload: true,
    source_maps: true,
    debug_logging: true,
    mock_data: true,
  },

  // 本番環境設定
  production: {
    hot_reload: false,
    source_maps: false,
    debug_logging: false,
    mock_data: false,
  },
}

// Authentication Configuration Library
// 認証設定用のJsonnetライブラリ

{
  // JWT認証設定
  jwt:: {
    provider: 'Local',

    // JWT設定
    jwt_secret: error 'jwt_secret must be specified',
    session_timeout: 3600,

    // 認証設定
    config: {
      bcrypt_rounds: 12,
      password_min_length: 8,
      require_email_verification: false,
      max_login_attempts: 5,
      lockout_duration_minutes: 15,
    },
  },

  // OAuth2認証設定
  oauth2:: {
    provider: 'OAuth2',
    config: {
      providers: [],
      client_id: error 'client_id must be specified',
      client_secret: error 'client_secret must be specified',
      redirect_uri: error 'redirect_uri must be specified',
      scopes: ['openid', 'profile', 'email'],
    },
  },

  // LDAP認証設定
  ldap:: {
    provider: 'LDAP',
    config: {
      server_url: error 'server_url must be specified',
      base_dn: error 'base_dn must be specified',
      bind_dn: error 'bind_dn must be specified',
      bind_password: error 'bind_password must be specified',
      user_filter: '(&(objectClass=person)(uid={username}))',
      group_filter: '(&(objectClass=groupOfNames)(member={dn}))',
    },
  },

  // SAML認証設定
  saml:: {
    provider: 'SAML',
    config: {
      idp_metadata_url: error 'idp_metadata_url must be specified',
      sp_entity_id: error 'sp_entity_id must be specified',
      assertion_consumer_service_url: error 'assertion_consumer_service_url must be specified',
      name_id_policy: 'urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress',
    },
  },

  // 便利な認証設定関数
  local: {
    jwt(secret, timeout=3600):: $.jwt {
      jwt_secret: secret,
      session_timeout: timeout,
    },

    oauth2Google(client_id, client_secret, redirect_uri):: $.oauth2 {
      config+: {
        providers: ['google'],
        client_id: client_id,
        client_secret: client_secret,
        redirect_uri: redirect_uri,
      },
    },

    oauth2GitHub(client_id, client_secret, redirect_uri):: $.oauth2 {
      config+: {
        providers: ['github'],
        client_id: client_id,
        client_secret: client_secret,
        redirect_uri: redirect_uri,
        scopes: ['user:email', 'read:user'],
      },
    },

    multiProvider(providers):: {
      provider: 'Multi',
      config: {
        providers: providers,
        allow_registration: true,
        default_provider: providers[0],
      },
    },
  },

  // 権限設定
  permissions: {
    // ロールベースアクセス制御
    roles: {
      admin: {
        permissions: ['*'],
        description: 'Full system access',
      },
      moderator: {
        permissions: ['users.read', 'users.write', 'posts.*', 'comments.*'],
        description: 'Content moderation access',
      },
      user: {
        permissions: ['users.read.self', 'posts.read', 'posts.write.self', 'comments.read', 'comments.write'],
        description: 'Basic user access',
      },
      guest: {
        permissions: ['posts.read', 'comments.read'],
        description: 'Read-only access',
      },
    },

    // リソースベースアクセス制御
    resources: {
      users: {
        operations: ['create', 'read', 'update', 'delete'],
        conditions: {
          self: 'user_id == current_user_id',
          admin: 'current_user_role == "admin"',
        },
      },
      posts: {
        operations: ['create', 'read', 'update', 'delete', 'publish'],
        conditions: {
          self: 'author_id == current_user_id',
          published: 'status == "published"',
          draft: 'status == "draft"',
        },
      },
      comments: {
        operations: ['create', 'read', 'update', 'delete'],
        conditions: {
          self: 'author_id == current_user_id',
          post_owner: 'post.author_id == current_user_id',
        },
      },
    },
  },

  // ポリシー設定
  policies: {
    // ABAC (Attribute-Based Access Control)
    abac: {
      rules: [
        {
          name: 'user_self_management',
          resource: 'users',
          operation: 'update',
          condition: 'resource.id == subject.id || subject.role == "admin"',
        },
        {
          name: 'post_publish_permission',
          resource: 'posts',
          operation: 'publish',
          condition: 'subject.role in ["admin", "editor"] || resource.author_id == subject.id',
        },
        {
          name: 'comment_moderation',
          resource: 'comments',
          operation: 'delete',
          condition: 'subject.role in ["admin", "moderator"] || resource.post.author_id == subject.id',
        },
      ],
    },

    // RBAC (Role-Based Access Control)
    rbac: {
      role_permissions: $.permissions.roles,
      role_hierarchy: {
        admin: ['moderator', 'user'],
        moderator: ['user'],
        user: ['guest'],
      },
    },
  },

  // セキュリティ設定
  security: {
    password_policy: {
      min_length: 8,
      require_uppercase: true,
      require_lowercase: true,
      require_numbers: true,
      require_symbols: false,
      prevent_common_passwords: true,
      max_consecutive_chars: 3,
    },

    session_policy: {
      max_concurrent_sessions: 5,
      session_timeout_minutes: 60,
      extend_on_activity: true,
      remember_me_days: 30,
    },

    brute_force_protection: {
      max_attempts: 5,
      lockout_duration_minutes: 15,
      progressive_delays: true,
      ip_whitelist: [],
      ip_blacklist: [],
    },

    two_factor_auth: {
      required: false,
      methods: ['totp', 'sms', 'email'],
      issuer_name: 'Kotoba App',
      backup_codes_count: 10,
    },
  },

  // ソーシャルログイン設定
  social: {
    google: {
      client_id: error 'Google client_id must be specified',
      client_secret: error 'Google client_secret must be specified',
      scopes: ['openid', 'profile', 'email'],
      user_info_url: 'https://www.googleapis.com/oauth2/v2/userinfo',
    },

    github: {
      client_id: error 'GitHub client_id must be specified',
      client_secret: error 'GitHub client_secret must be specified',
      scopes: ['user:email', 'read:user'],
      user_info_url: 'https://api.github.com/user',
      emails_url: 'https://api.github.com/user/emails',
    },

    twitter: {
      client_id: error 'Twitter client_id must be specified',
      client_secret: error 'Twitter client_secret must be specified',
      scopes: ['tweet.read', 'users.read'],
      user_info_url: 'https://api.twitter.com/2/users/me',
    },

    facebook: {
      client_id: error 'Facebook client_id must be specified',
      client_secret: error 'Facebook client_secret must be specified',
      scopes: ['email', 'public_profile'],
      user_info_url: 'https://graph.facebook.com/me?fields=id,name,email,picture',
    },
  },
}

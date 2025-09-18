# 🎯 組み込みハンドラー - Rustコード不要のサイト構築

Kotoba Pagesの**組み込みハンドラー**を使用すると、**Rustコードを書かずに**Jsonnetだけで完全なWebサイトを構築できます！

## 🌟 概要

従来の静的サイトジェネレーターでは、HTMLテンプレート、CSS、JavaScriptを別々に管理する必要がありましたが、Kotoba Pagesでは**すべてをJsonnetで定義**できます。

```jsonnet
{
  handlers: {
    "GET /": {
      handler_type: "Template",
      template: "<h1>Hello from pure Jsonnet!</h1>"
    },
    "GET /api": {
      handler_type: "Json",
      data: { message: "Hello from JSON API!" }
    }
  }
}
```

## 🚀 利用可能なハンドラー

### 1. Template ハンドラー

HTMLテンプレートを直接Jsonnet内で定義：

```jsonnet
handlers: {
  "GET /": {
    handler_type: "Template",
    content_type: "text/html",
    status: 200,
    template: |||
      <!DOCTYPE html>
      <html>
      <head>
        <title>My Site</title>
        <style>
          body { font-family: Arial, sans-serif; }
          .hero { text-align: center; padding: 2rem; }
        </style>
      </head>
      <body>
        <div class="hero">
          <h1>Welcome to Kotoba Pages!</h1>
          <p>Built with pure Jsonnet - no HTML files required!</p>
        </div>
      </body>
      </html>
    |||
  }
}
```

### 2. JSON ハンドラー

JSON APIレスポンスを生成：

```jsonnet
handlers: {
  "GET /api/users": {
    handler_type: "Json",
    content_type: "application/json",
    status: 200,
    data: {
      users: [
        { id: 1, name: "Alice", email: "alice@example.com" },
        { id: 2, name: "Bob", email: "bob@example.com" }
      ],
      total: 2,
      timestamp: std.toString(std.time())
    }
  }
}
```

### 3. Static ハンドラー

静的ファイルの提供：

```jsonnet
handlers: {
  "GET /robots.txt": {
    handler_type: "Static",
    file_path: "robots.txt",
    content_type: "text/plain"
  }
}
```

### 4. Form ハンドラー

HTMLフォームの生成：

```jsonnet
handlers: {
  "GET /contact": {
    handler_type: "Form",
    content_type: "text/html",
    fields: ["name", "email", "message"],
    template: |||
      <!DOCTYPE html>
      <html>
      <head><title>Contact</title></head>
      <body>
        <h1>Contact Us</h1>
        <form method="POST">
          <input name="name" placeholder="Name" required>
          <input name="email" type="email" placeholder="Email" required>
          <textarea name="message" placeholder="Message" required></textarea>
          <button type="submit">Send</button>
        </form>
      </body>
      </html>
    |||
  }
}
```

### 5. Redirect ハンドラー

リダイレクト処理：

```jsonnet
handlers: {
  "GET /old-page": {
    handler_type: "Redirect",
    status: 302,
    redirect_url: "/new-page"
  }
}
```

### 6. Custom ハンドラー

高度な処理が必要な場合：

```jsonnet
handlers: {
  "GET /dashboard": {
    handler_type: "Custom",
    custom_handler: "dashboard_handler",
    // カスタムロジックはRust側で実装
  }
}
```

## 🎨 高度な機能

### データ駆動型コンテンツ

Jsonnetの機能を活かして動的コンテンツを生成：

```jsonnet
{
  data: {
    posts: [
      { id: 1, title: "First Post", content: "..." },
      { id: 2, title: "Second Post", content: "..." }
    ],
    products: [
      { name: "Basic", price: 0 },
      { name: "Pro", price: 29 }
    ]
  },

  handlers: {
    "GET /blog": {
      handler_type: "Template",
      template: |||
        <h1>Blog Posts</h1>
        {% for post in data.posts %}
        <article>
          <h2>{{ post.title }}</h2>
          <p>{{ post.content }}</p>
        </article>
        {% endfor %}
      |||
    },

    "GET /products": {
      handler_type: "Template",
      template: |||
        <h1>Our Products</h1>
        <div class="products">
          {% for product in data.products %}
          <div class="product">
            <h3>{{ product.name }}</h3>
            <p>${{ product.price }}/month</p>
          </div>
          {% endfor %}
        </div>
      |||
    }
  }
}
```

### 条件付きレンダリング

Jsonnetの条件式を使って動的なコンテンツ生成：

```jsonnet
{
  data: {
    user: {
      logged_in: true,
      name: "Alice",
      role: "admin"
    },
    features: [
      { name: "Dashboard", requires_auth: true },
      { name: "Public Page", requires_auth: false }
    ]
  },

  handlers: {
    "GET /": {
      handler_type: "Template",
      template: |||
        <div class="header">
          {% if data.user.logged_in %}
          <p>Welcome back, {{ data.user.name }}!</p>
          {% else %}
          <p><a href="/login">Login</a></p>
          {% endif %}
        </div>

        <nav>
          {% for feature in data.features %}
          {% if !feature.requires_auth || data.user.logged_in %}
          <a href="/{{ feature.name | lower }}">{{ feature.name }}</a>
          {% endif %}
          {% endfor %}
        </nav>
      |||
    }
  }
}
```

### API統合

外部APIとの連携：

```jsonnet
{
  handlers: {
    "GET /weather": {
      handler_type: "Json",
      // 実際のAPI呼び出しはカスタムハンドラーで実装
      data: {
        location: "Tokyo",
        temperature: 22,
        condition: "Sunny",
        last_updated: std.toString(std.time())
      }
    },

    "POST /webhook": {
      handler_type: "Json",
      // Webhook処理
      data: {
        received: true,
        timestamp: std.toString(std.time()),
        status: "processed"
      }
    }
  }
}
```

## 📱 レスポンシブデザイン

組み込みのCSSフレームワーク：

```jsonnet
handlers: {
  "GET /": {
    handler_type: "Template",
    template: |||
      <!DOCTYPE html>
      <html>
      <head>
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <style>
          .container { max-width: 1200px; margin: 0 auto; padding: 0 1rem; }
          .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 2rem; }
          .card { background: white; padding: 2rem; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }

          @media (max-width: 768px) {
            .grid { grid-template-columns: 1fr; }
            .card { padding: 1rem; }
          }
        </style>
      </head>
      <body>
        <div class="container">
          <div class="grid">
            <div class="card">
              <h3>Mobile First</h3>
              <p>Responsive design that works on all devices</p>
            </div>
            <div class="card">
              <h3>Fast Loading</h3>
              <p>Optimized for performance</p>
            </div>
          </div>
        </div>
      </body>
      </html>
    |||
  }
}
```

## 🔧 ユーティリティ関数

Jsonnetの標準ライブラリを活用：

```jsonnet
{
  data: {
    // ユーティリティ関数
    format_date: function(timestamp) std.substr(std.toString(timestamp), 0, 10),
    capitalize: function(str) std.char(std.codepoint(str[0]) - 32) + std.substr(str, 1, std.length(str) - 1),

    // 計算されたデータ
    recent_posts: std.take(5, std.sort(self.posts, function(a, b) a.date > b.date)),
    categories: std.set(std.map(function(p) p.category, self.posts)),
    stats: {
      total_posts: std.length(self.posts),
      total_words: std.foldl(function(acc, p) acc + std.length(std.split(p.content, " ")), 0, self.posts)
    }
  },

  handlers: {
    "GET /stats": {
      handler_type: "Json",
      data: {
        posts: data.stats.total_posts,
        words: data.stats.total_words,
        categories: data.stats.categories,
        recent: std.map(function(p) {
          title: p.title,
          date: data.format_date(p.date),
          category: data.capitalize(p.category)
        }, data.recent_posts)
      }
    }
  }
}
```

## 🚀 デプロイメント

GitHub Pagesへの自動デプロイ：

```jsonnet
{
  deployment: {
    provider: "github_pages",
    branch: "gh-pages",
    build_command: "kotoba build site.kotoba",
    output_dir: "_site",
    optimize: true
  }
}
```

## 📊 例：完全なブログサイト

```jsonnet
{
  data: {
    site: {
      name: "My Blog",
      author: "Author Name",
      description: "A blog built with Kotoba Pages"
    },
    posts: [
      {
        id: "1",
        title: "Getting Started with Kotoba",
        slug: "getting-started",
        content: "Learn how to build websites with pure Jsonnet...",
        date: "2024-01-15",
        category: "tutorial",
        tags: ["kotoba", "jsonnet", "tutorial"]
      }
    ]
  },

  handlers: {
    "GET /": {
      handler_type: "Template",
      template: |||
        <!DOCTYPE html>
        <html>
        <head>
          <title>{{ data.site.name }}</title>
          <meta name="description" content="{{ data.site.description }}">
          <style>
            body { font-family: Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 2rem; }
            .post { margin-bottom: 2rem; padding: 1rem; border: 1px solid #ddd; border-radius: 8px; }
            .tags { display: flex; gap: 0.5rem; }
            .tag { background: #f0f0f0; padding: 0.25rem 0.5rem; border-radius: 12px; font-size: 0.8rem; }
          </style>
        </head>
        <body>
          <header>
            <h1>{{ data.site.name }}</h1>
            <p>{{ data.site.description }}</p>
          </header>

          <main>
            {% for post in data.posts %}
            <article class="post">
              <h2><a href="/posts/{{ post.slug }}">{{ post.title }}</a></h2>
              <p>{{ post.content | truncate: 100 }}</p>
              <div class="meta">
                <time>{{ post.date }}</time>
                <span>{{ post.category }}</span>
              </div>
              <div class="tags">
                {% for tag in post.tags %}
                <span class="tag">{{ tag }}</span>
                {% endfor %}
              </div>
            </article>
            {% endfor %}
          </main>

          <footer>
            <p>© 2024 {{ data.site.author }}</p>
          </footer>
        </body>
        </html>
      |||
    },

    "GET /posts/:slug": {
      handler_type: "Template",
      template: |||
        {% assign post = data.posts | find: "slug", params.slug %}
        <!DOCTYPE html>
        <html>
        <head>
          <title>{{ post.title }} - {{ data.site.name }}</title>
        </head>
        <body>
          <h1>{{ post.title }}</h1>
          <p>Posted on {{ post.date }} in {{ post.category }}</p>
          <div>{{ post.content }}</div>
          <a href="/">← Back to home</a>
        </body>
        </html>
      |||
    },

    "GET /api/posts": {
      handler_type: "Json",
      data: data.posts
    }
  },

  deployment: {
    provider: "github_pages",
    branch: "gh-pages"
  }
}
```

## 🎯 利点

### 開発効率
- **単一言語**: すべてをJsonnetで記述
- **型安全性**: Jsonnetの型チェック
- **再利用性**: コンポーネントの再利用
- **保守性**: 設定の一元管理

### パフォーマンス
- **高速ビルド**: Jsonnetの高速評価
- **最適化**: 自動アセット最適化
- **キャッシュ**: インテリジェントなキャッシュ

### 拡張性
- **モジュール化**: 設定の分割と再利用
- **動的生成**: データ駆動型のコンテンツ
- **API統合**: 外部サービスとの連携

---

**組み込みハンドラー**を使用することで、**Rustコードを書かずに**Jsonnetだけで本格的なWebサイトを構築できます。従来の静的サイトジェネレーターの複雑さを排除し、直感的で強力な開発体験を提供します！

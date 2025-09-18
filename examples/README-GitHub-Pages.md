# 🎉 Kotoba言語で実装されたGitHub Pages

このディレクトリでは、**Kotoba言語（Jsonnet）で完全に実装されたGitHub Pages**のデモンストレーションを紹介します。

## 🌟 概要

従来のGitHub PagesはJekyllなどの静的サイトジェネレーターを使っていましたが、この実装では：

- ✅ **純粋なKotoba言語**でサイト全体を定義
- ✅ **Jsonnetの機能をフル活用**（変数、関数、条件分岐など）
- ✅ **型安全な設定**（Jsonnetの型システム）
- ✅ **再利用可能なコンポーネント**
- ✅ **自動最適化とデプロイメント**

## 🚀 クイックスタート

### 1. シンプルなサイトを作成

```bash
# 基本的なサイト定義ファイルを作成
cat > my-site.kotoba << 'EOF'
{
  config: {
    name: "My Awesome Site",
    description: "Built with Kotoba Pages",
    base_url: "https://username.github.io/my-site",
  },

  pages: [
    {
      name: "index",
      title: "Welcome to My Site",
      template: "home",
      content: {
        message: "Hello from Kotoba Pages!",
        features: ["Fast", "Beautiful", "Easy to use"]
      }
    }
  ]
}
EOF

# サイトをビルド
kotoba build my-site.kotoba

# ローカルでプレビュー
kotoba serve _site

# GitHub Pagesにデプロイ
kotoba deploy my-site.kotoba
```

### 2. 高度な機能を使う

```jsonnet
// advanced-site.kotoba
local siteName = "Advanced Kotoba Site";
local author = "Your Name";

{
  config: {
    name: siteName,
    description: "A sophisticated site built with Kotoba",
    base_url: "https://username.github.io/advanced-site",
    author: author,
  },

  // 動的なページ生成
  pages: [
    {
      name: "index",
      title: "Home - " + siteName,
      template: "home",
      content: {
        hero: {
          title: "Welcome to " + siteName,
          subtitle: "Built with pure Jsonnet",
        },
        features: [
          {
            title: "Pure Jsonnet",
            description: "Write everything in Jsonnet syntax"
          },
          {
            title: "Type Safe",
            description: "Jsonnet's type system prevents errors"
          },
          {
            title: "Reusable",
            description: "Components and templates are reusable"
          }
        ]
      }
    },
    // 他のページ...
  ],

  // 再利用可能なコンポーネント
  components: [
    {
      name: "HeroSection",
      template: "hero.html.jsonnet"
    }
  ],

  // 条件付き設定
  deployment: {
    branch: if std.startsWith(std.extVar("BRANCH"), "release/") then "gh-pages" else "staging",
    cname: if std.extVar("ENV") == "production" then "example.com" else null,
  }
}
```

## 📁 ファイル構造

```
my-kotoba-site/
├── site.kotoba          # メインサイト定義ファイル
├── _templates/          # HTMLテンプレート
│   ├── default.html.jsonnet
│   └── home.html.jsonnet
├── assets/             # CSS, JS, 画像
│   ├── style.css
│   └── main.js
└── content/            # Markdownコンテンツ
    ├── index.md
    └── about.md
```

## 🎨 テンプレートシステム

### 基本的なHTMLテンプレート

```jsonnet
// _templates/page.html.jsonnet
{
  html: std.join("\n", [
    "<!DOCTYPE html>",
    "<html>",
    "<head>",
    "  <title>" + page.title + " - " + site.title + "</title>",
    "  <meta name='description' content='" + page.description + "'>",
    "  <link rel='stylesheet' href='/assets/style.css'>",
    "</head>",
    "<body>",
    "  <nav>",
    "    <a href='/'>" + site.title + "</a>",
    "  </nav>",
    "  <main>",
    "    <h1>" + page.title + "</h1>",
    "    <div class='content'>",
           page.content,
    "    </div>",
    "  </main>",
    "  <footer>",
    "    <p>© 2024 " + site.author + "</p>",
    "  </footer>",
    "</body>",
    "</html>"
  ])
}
```

### 再利用可能なコンポーネント

```jsonnet
// components/Button.html.jsonnet
local styles = {
  primary: "btn btn-primary",
  secondary: "btn btn-secondary",
};

function(props) {
  local className = styles[props.variant or "primary"],
  local text = props.text or "Click me",

  html: "<button class='" + className + "'>" + text + "</button>"
}
```

## 🔧 高度な機能

### 動的コンテンツ生成

```jsonnet
// ブログ記事の動的生成
local posts = [
  { title: "First Post", date: "2024-01-01", content: "..." },
  { title: "Second Post", date: "2024-01-02", content: "..." },
];

{
  pages: [
    // インデックスページ
    {
      name: "blog",
      title: "Blog",
      content: {
        posts: std.map(function(post) {
          title: post.title,
          date: post.date,
          url: "/blog/" + std.strReplace(post.title, " ", "-")
        }, posts)
      }
    }
  ] + [
    // 個別記事ページを動的生成
    {
      name: "blog/" + std.strReplace(post.title, " ", "-"),
      title: post.title,
      content: post.content,
      date: post.date
    }
    for post in posts
  ]
}
```

### 条件付き設定

```jsonnet
local environment = std.extVar("ENV");
local isProduction = environment == "production";

{
  config: {
    base_url: if isProduction
      then "https://example.com"
      else "https://staging.example.com",
  },

  build: {
    minify: isProduction,
    sourcemaps: !isProduction,
  },

  deployment: {
    branch: if isProduction then "gh-pages" else "staging",
    cname: if isProduction then "example.com" else null,
  }
}
```

### データ駆動型ページ

```jsonnet
local products = import "data/products.jsonnet";

{
  pages: [
    // 商品一覧ページ
    {
      name: "products",
      title: "Our Products",
      content: {
        products: std.map(function(product) {
          name: product.name,
          price: "$" + std.toString(product.price),
          description: product.description,
          image: product.image,
          url: "/products/" + product.id
        }, products)
      }
    }
  ] + [
    // 個別商品ページ
    {
      name: "products/" + product.id,
      title: product.name,
      content: product
    }
    for product in products
  ]
}
```

## 🚀 デプロイメント

### GitHub Pagesへの自動デプロイ

```bash
# サイトをビルドしてGitHub Pagesにデプロイ
kotoba deploy site.kotoba

# 特定のブランチにデプロイ
kotoba deploy site.kotoba --branch main

# ドメインを設定
kotoba deploy site.kotoba --cname example.com
```

### CI/CD統合

```yaml
# .github/workflows/deploy.yml
name: Deploy to GitHub Pages

on:
  push:
    branches: [ main ]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
        with:
          node-version: '18'

      - name: Install Kotoba
        run: cargo install kotoba

      - name: Build site
        run: kotoba build site.kotoba

      - name: Deploy to GitHub Pages
        run: kotoba deploy site.kotoba
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## 📊 利点

### 従来の静的サイトジェネレーターとの比較

| 機能 | Jekyll | Hugo | Kotoba Pages |
|------|--------|------|--------------|
| 言語 | Ruby/Liquid | Go | Jsonnet |
| 設定 | YAML | YAML/TOML | Jsonnet |
| テンプレート | Liquid | Go | Jsonnet |
| 型安全性 | ❌ | ❌ | ✅ |
| 再利用性 | 中 | 中 | 高 |
| 学習コスト | 中 | 中 | 低（Jsonnet既知の場合） |

### Kotoba Pagesの強み

1. **Jsonnetの全機能利用**
   - 変数、関数、条件分岐
   - オブジェクト合成と継承
   - 配列内包表記
   - 標準ライブラリ

2. **型安全性**
   - Jsonnetの型システムによる設定検証
   - コンパイル時エラー検出

3. **設定の再利用**
   - 共通設定の抽出と再利用
   - 環境別設定の管理

4. **動的生成**
   - データ駆動型のページ生成
   - 条件付きコンテンツ

## 🎯 使用例

### 1. 個人ブログ

```jsonnet
local blogConfig = {
  title: "My Blog",
  author: "John Doe",
  posts: import "posts.jsonnet",
};

{
  config: blogConfig,
  pages: [
    {
      name: "index",
      title: blogConfig.title,
      content: {
        recentPosts: std.take(5, blogConfig.posts)
      }
    }
  ] + [
    {
      name: "posts/" + post.slug,
      title: post.title,
      content: post
    }
    for post in blogConfig.posts
  ]
}
```

### 2. ドキュメントサイト

```jsonnet
local docs = import "docs-structure.jsonnet";

{
  config: {
    name: "My Project Docs",
    base_url: "https://docs.example.com",
  },

  pages: [
    // 自動生成された目次ページ
    {
      name: "index",
      title: "Documentation",
      content: {
        sections: std.groupBy(function(doc) doc.category, docs)
      }
    }
  ] + [
    // 各ドキュメントページ
    {
      name: doc.slug,
      title: doc.title,
      content: doc.content,
      category: doc.category
    }
    for doc in docs
  ]
}
```

### 3. ポートフォリオサイト

```jsonnet
local projects = import "projects.jsonnet";

{
  config: {
    name: "My Portfolio",
    description: "Showcase of my work",
  },

  pages: [
    {
      name: "index",
      title: "Portfolio",
      content: {
        featured: std.filter(function(p) p.featured, projects),
        categories: std.groupBy(function(p) p.category, projects)
      }
    },
    {
      name: "about",
      title: "About Me",
      content: import "about.jsonnet"
    }
  ] + [
    {
      name: "projects/" + project.slug,
      title: project.title,
      content: project
    }
    for project in projects
  ]
}
```

## 🔗 関連リンク

- [Kotoba言語ドキュメント](https://kotoba.dev/docs)
- [Jsonnet言語仕様](https://jsonnet.org/)
- [GitHub Pages公式ドキュメント](https://pages.github.com/)

---

**Kotoba Pages** - Jsonnetの力を借りて、静的サイト生成を再定義する

# Kotoba Pages Demo

This static site was generated from pure Jsonnet using Kotoba language and kotoba2tsx.

## 🚀 How it was built

1. **Kotoba Definition**: Website components defined in pure Jsonnet (.kotoba files)
2. **TSX Generation**: Converted to React TSX components using kotoba2tsx
3. **Static Build**: Generated static HTML/CSS/JS files for GitHub Pages
4. **Deploy**: Ready for deployment to GitHub Pages - no server required!

## 🎯 Features

- ✅ **Zero Boilerplate** - Write everything in Jsonnet
- ✅ **Type Safe** - Jsonnet prevents configuration errors
- ✅ **GitHub Pages Ready** - Deploy with one command
- ✅ **Modern Design** - Responsive and beautiful
- ✅ **Interactive** - JavaScript functionality included
- ✅ **SEO Optimized** - Meta tags and sitemap included

## 📁 File Structure

```
_site/
├── index.html              # Main page
├── docs/
│   └── index.html         # Documentation page
├── assets/
│   ├── css/
│   │   └── style.css      # Stylesheet
│   └── js/
│       └── main.js        # JavaScript
├── CNAME                  # GitHub Pages domain
├── .nojekyll             # Prevent Jekyll processing
├── robots.txt            # SEO
├── sitemap.xml           # SEO
└── README.md             # This file
```

## 🛠️ Technology Stack

- **Kotoba Language**: Configuration language for defining web components
- **kotoba2tsx**: Converts Kotoba to React TSX components
- **Static Generation**: No server required - pure static files
- **GitHub Pages**: Free hosting for static sites

## 🚀 Deployment

To deploy to GitHub Pages:

1. Push the contents of this `_site` directory to your GitHub repository
2. Go to repository Settings > Pages
3. Set source to "Deploy from a branch"
4. Select the branch containing these files
5. Set folder to "/ (root)"
6. Save and wait for deployment

Your site will be available at: `https://[username].github.io/[repository-name]`

## 📝 Example Kotoba Definition

```jsonnet
{
  name: "MySite",
  components: {
    App: {
      type: "component",
      component_type: "div",
      props: { className: "app" },
      children: ["Header", "Main", "Footer"]
    }
  }
}
```

---

Built with ❤️ using Kotoba ecosystem.

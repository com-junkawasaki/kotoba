# KotobaScript REPL

An interactive online REPL (Read-Eval-Print Loop) for KotobaScript, inspired by [Try PureScript](https://try.purescript.org/).

## 🌸 What is KotobaScript?

KotobaScript is a declarative programming language that extends Jsonnet to enable frontend application development. It provides a unified approach to defining React components, pages, state management, and API integrations using familiar Jsonnet syntax.

## 🚀 Features

- **Interactive Editor**: Monaco Editor with syntax highlighting for Jsonnet/KotobaScript
- **Real-time Evaluation**: Auto-run functionality for instant feedback
- **Shareable Code**: Generate shareable URLs for your code snippets
- **Code Formatting**: Automatic code formatting
- **Examples Gallery**: Built-in examples to get you started
- **Responsive Design**: Works on desktop and mobile devices

## 🛠️ Usage

### Basic Syntax

```jsonnet
{
  // Objects
  name: "KotobaScript",
  version: "0.1.0",

  // Arrays
  features: [
    "Declarative programming",
    "Type safety",
    "Functional programming"
  ],

  // Functions
  double: function(x) x * 2,

  // Local variables
  local multiplier = 3,
  result: multiplier * 10,

  // Standard library functions
  greeting: std.join(" ", ["Hello", "KotobaScript", "World!"])
}
```

### Examples

The REPL includes several built-in examples:

- **Basic Object**: Simple object creation
- **Arrays**: Working with arrays
- **Functions**: Defining and calling functions
- **Std Library**: Using Jsonnet's standard library functions

## 🔗 Links

- [KotobaScript Documentation](https://github.com/com-junkawasaki/kotoba/blob/main/KOTOBA_FRAMEWORK.md)
- [GitHub Repository](https://github.com/com-junkawasaki/kotoba)
- [More Examples](https://github.com/com-junkawasaki/kotoba/tree/main/examples)

## 🏗️ Architecture

The REPL is built with:

- **Monaco Editor**: For the code editing experience
- **Vanilla JavaScript**: No frameworks, lightweight and fast
- **CSS Grid & Flexbox**: Modern responsive layout
- **GitHub Pages**: For hosting and deployment

## 🚀 Deployment

The REPL is automatically deployed to GitHub Pages when changes are pushed to the main branch. The deployment workflow:

1. Copies REPL files to a `repl/` directory
2. Deploys to GitHub Pages using GitHub Actions
3. Available at `https://[username].github.io/kotoba/repl/`

## 🤝 Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.

## 📄 License

Licensed under MIT OR Apache-2.0. See the main [LICENSE](https://github.com/com-junkawasaki/kotoba/blob/main/LICENSE) file for details.

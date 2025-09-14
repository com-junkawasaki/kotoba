# Kotoba2TSX

Kotoba2TSX is a Rust crate that converts Kotoba configuration files (`.kotoba`) to React TypeScript components (`.tsx`). Kotoba is a Jsonnet-based configuration format for defining UI components declaratively.

## Features

- Parse `.kotoba` files (Jsonnet format)
- Generate React TypeScript components
- Support for functional and class components
- TypeScript type generation
- CLI interface for easy conversion
- Configurable output options

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
kotoba2tsx = "0.1.0"
```

Or install the CLI tool:

```bash
cargo install kotoba2tsx --features cli
```

## Usage

### Library Usage

```rust
use kotoba2tsx::{KotobaParser, TsxGenerator, TsxGenerationOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse a .kotoba file
    let parser = KotobaParser::new();
    let config = parser.parse_file("app.kotoba").await?;

    // Generate TSX code
    let options = TsxGenerationOptions::default();
    let generator = TsxGenerator::with_options(options);
    let tsx_code = generator.generate_tsx(&config)?;

    println!("{}", tsx_code);
    Ok(())
}
```

### CLI Usage

Convert a `.kotoba` file to `.tsx`:

```bash
kotoba2tsx convert --input app.kotoba --output App.tsx
```

Use with stdin/stdout:

```bash
cat app.kotoba | kotoba2tsx pipe > App.tsx
```

## Kotoba File Format

Kotoba files use Jsonnet syntax to define UI components declaratively:

```jsonnet
// app.kotoba
{
  config: {
    name: "MyApp",
    version: "1.0.0",
    theme: "light",
  },

  components: {
    App: {
      type: "component",
      name: "App",
      component_type: "div",
      props: {
        className: "app-container",
      },
      children: ["Header", "Main", "Footer"],
    },

    Header: {
      type: "component",
      name: "Header",
      component_type: "header",
      props: {
        className: "app-header",
      },
      children: ["Title"],
    },
  },
}
```

## Generated TSX Output

The above Kotoba file generates something like:

```tsx
import React, { FC } from 'react';

interface AppProps {
  className: string;
}

const App: FC<AppProps> = (props) => {
  return (
    <div className={props.className}>
      <Header />
      <Main />
      <Footer />
    </div>
  );
};

interface HeaderProps {
  className: string;
}

const Header: FC<HeaderProps> = (props) => {
  return (
    <header className={props.className}>
      <Title />
    </header>
  );
};

export default App;
```

## Configuration Options

- `--types`: Include TypeScript type definitions
- `--functional`: Generate functional components (default)
- `--prop-types`: Include prop type interfaces
- `--format`: Format the output code

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Building CLI

```bash
cargo build --release --features cli
```

## License

MIT OR Apache-2.0

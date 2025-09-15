# Kotoba Jsonnet

Pure Rust implementation of Jsonnet 0.21.0, fully compatible with Google Jsonnet.

## 🎯 Jsonnet 0.21.0 Complete Compatibility

This crate implements all features of [Google Jsonnet v0.21.0](https://github.com/google/jsonnet) in pure Rust.

### ✅ Implemented Features

#### **Core Language Features**
- ✅ Complete AST definition (Expr, Stmt, ObjectField, BinaryOp, UnaryOp)
- ✅ Full lexer with tokenization (identifiers, literals, operators, keywords)
- ✅ Recursive descent parser with precedence handling
- ✅ Expression evaluator with variable scoping
- ✅ Function definitions and calls
- ✅ Object and array literals
- ✅ **Bracket notation** - `obj["key"]` and `arr[index]` syntax ⭐
- ✅ **Array comprehensions** - `[x for x in arr if cond]` syntax ⭐
- ✅ Local variable bindings
- ✅ Conditional expressions (if/then/else)
- ✅ Import and ImportStr
- ✅ Error handling with try/catch
- ✅ Assertions

#### **Standard Library (80+ Functions)**
- ✅ **Array functions**: `length`, `makeArray`, `filter`, `map`, `foldl`, `foldr`, `range`, `member`, `count`, `uniq`, `sort`, `reverse`
- ✅ **String functions**: `length`, `substring`, `startsWith`, `endsWith`, `contains`, `toLower`, `toUpper`, `trim`, `split`, `join`, `format`
- ✅ **Object functions**: `objectFields`, `objectValues`, `objectHas`, `get`, `mergePatch`, `mapWithKey`, `prune`
- ✅ **Math functions**: `abs`, `sqrt`, `sin`, `cos`, `tan`, `log`, `exp`, `floor`, `ceil`, `round`, `pow`, `modulo`
- ✅ **Type functions**: `type`, `isArray`, `isBoolean`, `isFunction`, `isNumber`, `isObject`, `isString`
- ✅ **Encoding functions**: `base64`, `base64Decode`, `md5`, `escapeStringJson`, `escapeStringYaml`, `escapeStringPython`, `encodeUTF8`, `decodeUTF8`
- ✅ **Serialization**: `manifestJson`, `manifestJsonEx`, `manifestYaml`, `parseJson`, `toString`
- ✅ **Utility functions**: `assertEqual`, `clamp`, `max`, `min`

#### **API Compatibility**
- ✅ `evaluate()` - Evaluate Jsonnet code to JsonnetValue
- ✅ `evaluate_to_json()` - Evaluate to JSON string
- ✅ `evaluate_to_yaml()` - Evaluate to YAML string (with feature flag)
- ✅ `evaluate_with_filename()` - Evaluate with filename for error reporting
- ✅ Error types matching original Jsonnet behavior

### 📊 Architecture

```
Jsonnet Code → Lexer → Tokens → Parser → AST → Evaluator → JsonnetValue
                    ↓         ↓         ↓         ↓           ↓
                 Tokenize  Parse    Build     Eval     Evaluate
```

### 🔧 Components

- **`lib.rs`**: Public API (`evaluate`, `evaluate_to_json`, `evaluate_to_yaml`)
- **`error.rs`**: Error types (`JsonnetError`, `Result<T>`)
- **`value.rs`**: Value representation (`JsonnetValue`, `JsonnetFunction`)
- **`ast.rs`**: Abstract Syntax Tree definitions
- **`lexer.rs`**: Lexical analysis and tokenization
- **`parser.rs`**: Recursive descent parsing
- **`evaluator.rs`**: AST evaluation and execution
- **`stdlib.rs`**: 80+ standard library functions

### 🧪 Testing

Run the comprehensive test suite:
```bash
cargo test
```

Tests cover:
- ✅ Basic evaluation (literals, variables, functions)
- ✅ Complex expressions and operator precedence
- ✅ Standard library functions
- ✅ Error handling and edge cases
- ✅ JSON/YAML output formatting

### 📚 Usage

```rust
use kotoba_jsonnet::{evaluate, evaluate_to_json};

// Evaluate Jsonnet code
let result = evaluate(r#"
  local person = { name: "Alice", age: 30 };
  local greeting(name) = "Hello, " + name + "!";
  {
    message: greeting(person.name),
    data: person,
    doubled_age: person.age * 2,
  }
"#)?;

println!("Result: {:?}", result);

// Convert to JSON
let json = evaluate_to_json(r#"{ name: "World", count: 42 }"#)?;
println!("JSON: {}", json);
```

### 🔗 Integration with Kotoba

This Jsonnet implementation is integrated into the broader Kotoba ecosystem:

- Used for configuration parsing (`.kotoba` files)
- Powers the frontend framework's component definitions
- Enables deployment configuration templating
- Provides runtime configuration evaluation

### ⚡ Performance

- **Zero-copy evaluation** where possible
- **Efficient AST representation** with Box for recursive types
- **Lazy evaluation** for optimal performance
- **Memory-efficient** standard library implementations

### 🔄 Compatibility Matrix

| Feature | Google Jsonnet 0.21.0 | kotoba-jsonnet |
|---------|----------------------|----------------|
| Language spec | ✅ Complete | ✅ Complete |
| Standard library | ✅ 80+ functions | ✅ 80+ functions |
| Import system | ✅ import/importstr | ✅ Implemented |
| Error handling | ✅ try/catch/error | ✅ Implemented |
| JSON output | ✅ manifestJson | ✅ Implemented |
| YAML output | ✅ manifestYaml | ✅ Feature flag |
| Performance | C++ optimized | Rust zero-cost |

### 🤝 Contributing

This implementation aims for 100% compatibility with Google Jsonnet 0.21.0. If you find any discrepancies or missing features, please open an issue.

### 📄 License

MIT OR Apache-2.0 (matching Google Jsonnet)

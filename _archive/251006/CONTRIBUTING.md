# Contributing to KotobaDB

Thank you for your interest in contributing to KotobaDB! We welcome contributions from the community to help make KotobaDB better. This document outlines the guidelines and processes for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Contribution Guidelines](#contribution-guidelines)
- [Development Workflow](#development-workflow)
- [Testing](#testing)
- [Documentation](#documentation)
- [Review Process](#review-process)
- [Community](#community)

## Code of Conduct

This project follows a code of conduct to ensure a welcoming environment for all contributors. By participating, you agree to:

- Be respectful and inclusive
- Focus on constructive feedback
- Accept responsibility for mistakes
- Show empathy towards other community members
- Help create a positive environment

See our [Code of Conduct](.github/CODE_OF_CONDUCT.md) for details.

## Getting Started

### Prerequisites

- **Rust**: 1.70 or later
- **Cargo**: Latest stable version
- **Git**: 2.0 or later
- **System dependencies**: See [Installation Guide](docs/deployment/README.md#system-requirements)

### Quick Setup

```bash
# Fork and clone the repository
git clone https://github.com/your-username/kotoba.git
cd kotoba

# Build the project
cargo build

# Run tests
cargo test

# Start developing!
```

## Development Setup

### Environment Setup

1. **Install Rust toolchain**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Clone and setup**:
   ```bash
   git clone https://github.com/your-org/kotoba.git
   cd kotoba
   cargo build --release
   ```

3. **Verify installation**:
   ```bash
   ./target/release/kotoba --version
   ```

### IDE Setup

#### VS Code
- Install the "rust-analyzer" extension
- Install the "CodeLLDB" extension for debugging

#### IntelliJ IDEA / CLion
- Install the "Rust" plugin
- Configure Cargo project

### Development Commands

```bash
# Build
cargo build                    # Debug build
cargo build --release         # Release build

# Test
cargo test                    # Run all tests
cargo test --lib              # Run library tests only
cargo test --doc              # Run documentation tests

# Format and lint
cargo fmt                     # Format code
cargo clippy                  # Run linter

# Documentation
cargo doc --open              # Generate and open documentation

# Benchmark
cargo bench                   # Run benchmarks
```

## Contribution Guidelines

### Types of Contributions

We welcome various types of contributions:

- **🐛 Bug fixes**: Fix issues and improve stability
- **✨ New features**: Implement new functionality
- **📚 Documentation**: Improve docs, tutorials, examples
- **🧪 Tests**: Add tests, improve test coverage
- **⚡ Performance**: Optimize performance, reduce memory usage
- **🔧 Tools**: Build tools, improve developer experience
- **🎨 UI/UX**: Improve user interfaces and experiences

### Code Style

#### Rust Code Style

- Follow the official [Rust Style Guide](https://doc.rust-lang.org/1.0.0/style/README.html)
- Use `cargo fmt` to format code automatically
- Address all `cargo clippy` warnings
- Write idiomatic Rust code

#### Commit Messages

Follow conventional commit format:

```
type(scope): description

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Code style changes
- `refactor`: Code refactoring
- `test`: Testing
- `chore`: Maintenance

Examples:
```
feat: add graph traversal algorithms
fix: resolve memory leak in LSM compaction
docs: update deployment guide for Kubernetes
test: add integration tests for cluster mode
```

#### Branch Naming

Use descriptive branch names:

```
feature/add-graph-algorithms
bugfix/fix-memory-leak
docs/update-contributing-guide
test/add-cluster-tests
```

### Pull Request Guidelines

#### Before Submitting

1. **Update documentation** if your changes affect APIs or user-facing behavior
2. **Add tests** for new features and bug fixes
3. **Update examples** if relevant
4. **Run the full test suite** locally
5. **Check formatting and linting**

#### PR Template

Use the provided PR template and fill in all relevant sections:

- **Description**: What does this PR do?
- **Type of change**: Bug fix, feature, documentation, etc.
- **Breaking changes**: Does this introduce breaking changes?
- **Testing**: How was this tested?
- **Checklist**: Complete the provided checklist

#### PR Size

- **Small PRs**: Prefer smaller, focused changes
- **Large PRs**: Break down into logical chunks
- **Maximum size**: Try to keep PRs under 500 lines of code

## Development Workflow

### 1. Choose an Issue

- Check the [GitHub Issues](https://github.com/your-org/kotoba/issues) page
- Look for issues labeled `good first issue` or `help wanted`
- Comment on the issue to indicate you're working on it

### 2. Create a Branch

```bash
# Create and switch to a new branch
git checkout -b feature/your-feature-name

# Or for bug fixes
git checkout -b bugfix/issue-number-description
```

### 3. Make Changes

```bash
# Make your changes
# Add tests
# Update documentation

# Stage changes
git add .

# Commit with conventional format
git commit -m "feat: add new graph algorithm

- Implement Dijkstra's shortest path
- Add unit tests
- Update documentation"
```

### 4. Run Tests

```bash
# Run full test suite
cargo test

# Run specific tests
cargo test test_name

# Run benchmarks to ensure no regression
cargo bench
```

### 5. Update Documentation

```bash
# Generate documentation
cargo doc

# Check that docs build without errors
cargo doc --no-deps
```

### 6. Create Pull Request

```bash
# Push your branch
git push origin feature/your-feature-name

# Create PR on GitHub
# Fill in the PR template
# Request review from maintainers
```

### 7. Address Review Comments

```bash
# Make requested changes
git add .
git commit -m "fix: address review comments

- Fix clippy warnings
- Add more test cases
- Update documentation"

# Push updates
git push origin feature/your-feature-name
```

## Testing

### Test Structure

```
tests/
├── integration/     # Integration tests
├── load/           # Load and performance tests
└── fuzz/           # Fuzzing tests
```

### Running Tests

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration

# Load tests
cargo run --bin load_test_runner -- workload ycsb-a 60

# Fuzz tests (requires nightly)
cargo +nightly fuzz run data_structures -- -max_total_time=30
```

### Test Coverage

- **Target**: Maintain >80% code coverage
- **Tools**: Use `cargo-tarpaulin` for coverage reports
- **CI**: Coverage reports are generated automatically

### Writing Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Arrange
        let input = "test";

        // Act
        let result = process_input(input);

        // Assert
        assert_eq!(result, "expected_output");
    }

    #[tokio::test]
    async fn test_async_functionality() {
        // Async test
        let result = async_operation().await;
        assert!(result.is_ok());
    }
}
```

## Documentation

### Documentation Standards

- **API docs**: All public APIs must have documentation
- **Code comments**: Complex logic should be explained
- **Examples**: Provide code examples in documentation
- **README**: Keep README up to date

### Documentation Structure

```
docs/
├── api/           # API reference
├── deployment/    # Deployment guides
├── tutorials/     # Tutorials and examples
└── README.md      # Main documentation
```

### Writing Documentation

```rust
/// Calculate the shortest path between two nodes using Dijkstra's algorithm.
///
/// This function finds the minimum-cost path between `start` and `end` nodes
/// in a weighted graph.
///
/// # Arguments
///
/// * `graph` - The graph to search in
/// * `start` - The starting node
/// * `end` - The target node
///
/// # Returns
///
/// Returns `Some(path)` where `path` is a vector of node IDs representing
/// the shortest path, or `None` if no path exists.
///
/// # Examples
///
/// ```
/// use kotoba_graph::Graph;
///
/// let mut graph = Graph::new();
/// // ... populate graph ...
///
/// let path = dijkstra_shortest_path(&graph, start_node, end_node);
/// assert!(path.is_some());
/// ```
///
/// # Complexity
///
/// Time complexity: O((V + E) log V) where V is vertices, E is edges.
/// Space complexity: O(V).
///
/// # Panics
///
/// Panics if `start` or `end` nodes don't exist in the graph.
pub fn dijkstra_shortest_path(
    graph: &Graph,
    start: NodeId,
    end: NodeId
) -> Option<Vec<NodeId>> {
    // Implementation...
}
```

## Review Process

### Code Review Guidelines

#### Reviewers
- Check for correctness and safety
- Ensure tests are adequate
- Verify documentation is complete
- Confirm code follows project standards
- Consider performance implications

#### Authors
- Address all review comments
- Explain decisions when needed
- Add tests for edge cases
- Update documentation as needed

### Review Checklist

- [ ] **Functionality**: Does the code work as intended?
- [ ] **Tests**: Are there adequate tests?
- [ ] **Documentation**: Is documentation updated?
- [ ] **Style**: Does code follow project standards?
- [ ] **Performance**: Any performance concerns?
- [ ] **Security**: Any security implications?
- [ ] **Breaking Changes**: Does this break existing APIs?

### Automated Checks

PRs automatically run:
- **Build**: `cargo build --all-targets`
- **Test**: `cargo test`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt --check`
- **Security**: `cargo audit`
- **Coverage**: Code coverage report

## Community

### Communication Channels

- **GitHub Issues**: Bug reports and feature requests
- **GitHub Discussions**: General discussion and Q&A
- **Discord**: Real-time chat for contributors
- **Twitter**: Announcements and updates

### Getting Help

- **Documentation**: Check the [docs](docs/) first
- **Issues**: Search existing issues before creating new ones
- **Discussions**: Ask questions in GitHub Discussions
- **Discord**: Get real-time help from the community

### Recognition

Contributors are recognized through:
- **GitHub Contributors**: Listed in repository contributors
- **Changelog**: Mentioned in release notes
- **Hall of Fame**: Special recognition for major contributions
- **Swag**: T-shirts, stickers for significant contributors

### Governance

- **Maintainers**: Core team responsible for project direction
- **Contributors**: Community members who contribute regularly
- **Users**: Everyone who uses KotobaDB

### Decision Making

- **Technical decisions**: Discussed in GitHub Issues/Discussions
- **Code changes**: Via pull request review process
- **Releases**: Coordinated by maintainers with community input

## Recognition and Rewards

### Contributor Recognition

We recognize contributions in several ways:

1. **GitHub Recognition**
   - Listed in repository contributors
   - Mentioned in release notes

2. **Community Recognition**
   - Featured in community newsletter
   - Highlighted in social media

3. **Rewards Program**
   - Digital badges for contributions
   - Priority support for contributors
   - Exclusive access to beta features

### Hall of Fame

Outstanding contributors are inducted into our Hall of Fame:

- **Code Contributors**: Major feature implementations
- **Documentation Heroes**: Comprehensive documentation improvements
- **Community Builders**: Help build and nurture the community
- **Bug Hunters**: Find and fix critical bugs

## License

By contributing to KotobaDB, you agree that your contributions will be licensed under the same license as the project (Apache 2.0).

## Questions?

If you have questions about contributing:

1. Check this document first
2. Search existing GitHub Issues
3. Ask in GitHub Discussions
4. Join our Discord community

We appreciate your interest in contributing to KotobaDB! 🚀

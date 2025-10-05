# Kotoba Makefile - Core Graph Processing System
# ================================================

.PHONY: all test test-unit test-integration coverage coverage-html coverage-xml clean build release docs

# Default target
all: test build

# Testing targets
test: test-unit test-integration
	@echo "✅ All tests completed"

test-unit:
	@echo "🧪 Running unit tests..."
	cargo test --lib --quiet
	@echo "✅ Unit tests passed"

test-integration:
	@echo "🔗 Running integration tests..."
	cargo test --test '*' --quiet
	@echo "✅ Integration tests passed"

# Coverage targets (80% target)
coverage: coverage-html

coverage-html:
	@echo "📊 Generating HTML coverage report (Target: 80%)..."
	./scripts/coverage_report.sh html

coverage-xml:
	@echo "📊 Generating XML coverage report..."
	./scripts/coverage_report.sh xml

coverage-lcov:
	@echo "📊 Generating LCOV coverage report..."
	./scripts/coverage_report.sh lcov

# Build targets
build:
	@echo "🔨 Building Kotoba..."
	cargo build --quiet
	@echo "✅ Build completed"

build-release:
	@echo "🚀 Building release version..."
	cargo build --release --quiet
	@echo "✅ Release build completed"

# Development targets
check:
	@echo "🔍 Running cargo check..."
	cargo check --quiet
	@echo "✅ Code check passed"

fmt:
	@echo "🎨 Formatting code..."
	cargo fmt --quiet
	@echo "✅ Code formatted"

lint:
	@echo "🔎 Running clippy..."
	cargo clippy --quiet -- -D warnings
	@echo "✅ Linting passed"

dev-setup: fmt lint check

# Documentation
docs:
	@echo "📚 Generating documentation..."
	cargo doc --no-deps --quiet
	@echo "✅ Documentation generated at target/doc/kotoba/index.html"

# Clean up
clean:
	@echo "🧹 Cleaning up..."
	cargo clean
	rm -rf target/coverage
	@echo "✅ Cleanup completed"

# Core Graph Processing System specific targets
test-core:
	@echo "🔄 Testing Core Graph Processing components..."
	cargo test --package kotoba-core --quiet
	cargo test --package kotoba-storage --quiet
	cargo test --package kotoba-memory --quiet
	cargo test --package kotoba-graphdb --quiet
	@echo "✅ Core Graph Processing tests completed"

test-event-sourcing:
	@echo "📊 Testing Event Sourcing components..."
	cargo test --package kotoba-event-stream --quiet
	cargo test --package kotoba-projection-engine --quiet
	@echo "✅ Event Sourcing tests completed"

test-graph-ops:
	@echo "🔍 Testing Graph Operations..."
	cargo test --package kotoba-query-engine --quiet
	cargo test --package kotoba-execution --quiet
	cargo test --package kotoba-rewrite --quiet
	@echo "✅ Graph Operations tests completed"

test-app-layer:
	@echo "🎯 Testing Application Layer..."
	cargo test --package kotoba-routing --quiet
	cargo test --package kotoba-state-graph --quiet
	cargo test --package kotoba-handler --quiet
	@echo "✅ Application Layer tests completed"

# Integration test targets
test-storage-integration:
	@echo "💾 Running Storage Integration Tests..."
	cargo test --test integration storage_adapter --quiet
	@echo "✅ Storage Integration tests completed"

test-event-integration:
	@echo "📊 Running Event Sourcing Integration Tests..."
	cargo test --test integration event_sourcing --quiet
	@echo "✅ Event Sourcing Integration tests completed"

test-graph-integration:
	@echo "🔄 Running Core Graph Processing Integration Tests..."
	cargo test --test integration core_graph_processing --quiet
	@echo "✅ Core Graph Processing Integration tests completed"

# Performance testing
bench:
	@echo "⚡ Running benchmarks..."
	cargo bench --quiet
	@echo "✅ Benchmarks completed"

# CI/CD targets
ci: check test coverage-html
	@echo "✅ CI pipeline completed"

# Development workflow
dev: fmt lint test-unit build
	@echo "✅ Development workflow completed"

# Help target
help:
	@echo "Kotoba Makefile - Core Graph Processing System"
	@echo "==============================================="
	@echo ""
	@echo "Main Targets:"
	@echo "  all              - Run tests and build"
	@echo "  test             - Run all tests (unit + integration)"
	@echo "  test-unit        - Run unit tests only"
	@echo "  test-integration - Run integration tests only"
	@echo "  coverage         - Generate HTML coverage report (80% target)"
	@echo "  coverage-xml     - Generate XML coverage report"
	@echo "  coverage-lcov    - Generate LCOV coverage report"
	@echo "  build            - Build the project"
	@echo "  build-release    - Build release version"
	@echo "  clean            - Clean build artifacts"
	@echo ""
	@echo "Development Targets:"
	@echo "  check            - Run cargo check"
	@echo "  fmt              - Format code with rustfmt"
	@echo "  lint             - Run clippy linter"
	@echo "  dev-setup        - Format, lint, and check code"
	@echo "  docs             - Generate documentation"
	@echo ""
	@echo "Component-Specific Tests:"
	@echo "  test-core        - Test core graph processing components"
	@echo "  test-event-sourcing - Test event sourcing components"
	@echo "  test-graph-ops   - Test graph operations"
	@echo "  test-app-layer   - Test application layer"
	@echo ""
	@echo "Integration Tests:"
	@echo "  test-storage-integration  - Storage adapter integration tests"
	@echo "  test-event-integration    - Event sourcing integration tests"
	@echo "  test-graph-integration    - Core graph processing integration tests"
	@echo ""
	@echo "Performance & CI:"
	@echo "  bench            - Run benchmarks"
	@echo "  ci               - Run full CI pipeline"
	@echo "  dev              - Run development workflow"
	@echo ""
	@echo "Usage Examples:"
	@echo "  make test              # Run all tests"
	@echo "  make coverage          # Generate 80% coverage report"
	@echo "  make build-release     # Build optimized release"
	@echo "  make dev-setup         # Setup for development"
	@echo "  make clean && make all # Clean and rebuild everything"

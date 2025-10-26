#!/bin/bash

# Kotoba Coverage Report Generator
# Generates comprehensive coverage reports targeting 80% coverage

set -e

echo "🚀 Kotoba Coverage Report Generator"
echo "==================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if cargo-tarpaulin is installed
check_tarpaulin() {
    if ! command -v cargo-tarpaulin &> /dev/null; then
        print_error "cargo-tarpaulin is not installed."
        print_status "Install with: cargo install cargo-tarpaulin"
        exit 1
    fi
}

# Create coverage directory
setup_coverage_dir() {
    mkdir -p target/coverage
    print_status "Coverage directory created: target/coverage"
}

# Run unit tests first
run_unit_tests() {
    print_status "Running unit tests..."
    if cargo test --lib --quiet; then
        print_success "Unit tests passed"
    else
        print_error "Unit tests failed"
        exit 1
    fi
}

# Run integration tests
run_integration_tests() {
    print_status "Running integration tests..."
    if cargo test --test '*' --quiet; then
        print_success "Integration tests passed"
    else
        print_error "Integration tests failed"
        exit 1
    fi
}

# Generate coverage report
generate_coverage_report() {
    local report_type=$1
    local output_format=""
    local output_file=""

    case $report_type in
        "html")
            output_format="Html"
            output_file="target/coverage/coverage.html"
            ;;
        "lcov")
            output_format="Lcov"
            output_file="target/coverage/lcov.info"
            ;;
        "xml")
            output_format="Xml"
            output_file="target/coverage/coverage.xml"
            ;;
        *)
            output_format="Html"
            output_file="target/coverage/coverage.html"
            ;;
    esac

    print_status "Generating $report_type coverage report..."

    if cargo tarpaulin \
        --out $output_format \
        --output-dir target/coverage \
        --exclude-files "**/tests/**" \
        --exclude-files "**/benches/**" \
        --exclude-files "**/examples/**" \
        --line \
        --branch \
        --fail-under 80 \
        --verbose; then

        print_success "Coverage report generated: $output_file"

        # Show coverage summary
        if [ -f "target/coverage/tarpaulin-report.html" ]; then
            echo ""
            echo "📊 Coverage Summary:"
            echo "=================="

            # Extract coverage percentage from HTML report (simplified)
            if command -v grep &> /dev/null && command -v sed &> /dev/null; then
                coverage_line=$(grep -o "Total Coverage: [0-9.]*%" target/coverage/tarpaulin-report.html | head -1)
                if [ ! -z "$coverage_line" ]; then
                    echo "🎯 $coverage_line"
                fi
            fi
        fi

    else
        print_error "Coverage report generation failed"
        print_warning "Coverage might be below 80% threshold"
        exit 1
    fi
}

# Generate coverage badges
generate_badges() {
    print_status "Generating coverage badges..."

    # Create simple coverage badge (you can enhance this)
    echo "![Coverage](https://img.shields.io/badge/coverage-80%25-brightgreen)" > target/coverage/COVERAGE_BADGE.md

    print_success "Coverage badge generated"
}

# Show coverage targets
show_coverage_targets() {
    echo ""
    echo "🎯 Coverage Targets (80%+):"
    echo "=========================="
    echo "✅ Core Graph Processing:"
    echo "   - kotoba-core (foundation types)"
    echo "   - kotoba-storage (KeyValueStore trait)"
    echo "   - kotoba-memory (in-memory adapter)"
    echo "   - kotoba-graphdb (RocksDB adapter)"
    echo ""
    echo "✅ Event Sourcing:"
    echo "   - kotoba-event-stream (event management)"
    echo "   - kotoba-projection-engine (materialized views)"
    echo ""
    echo "✅ Graph Operations:"
    echo "   - kotoba-query-engine (GQL queries)"
    echo "   - kotoba-execution (graph operations)"
    echo "   - kotoba-rewrite (graph rewriting)"
    echo ""
    echo "✅ Application Layer:"
    echo "   - kotoba-routing (request routing)"
    echo "   - kotoba-state-graph (state management)"
    echo "   - kotoba-handler (request handling)"
}

# Main execution
main() {
    local report_type=${1:-"html"}

    print_status "Starting Kotoba Coverage Analysis (Target: 80%)"

    # Pre-flight checks
    check_tarpaulin
    setup_coverage_dir

    # Run tests
    run_unit_tests
    run_integration_tests

    # Generate coverage report
    generate_coverage_report "$report_type"

    # Generate badges
    generate_badges

    # Show targets
    show_coverage_targets

    echo ""
    print_success "Coverage analysis completed!"
    print_status "Open target/coverage/index.html to view detailed coverage report"

    # Final summary
    echo ""
    echo "📋 Next Steps:"
    echo "=============="
    echo "1. Review coverage report in target/coverage/"
    echo "2. Identify uncovered code sections"
    echo "3. Add additional tests for uncovered areas"
    echo "4. Run 'cargo coverage' to verify 80% target"
    echo ""
    echo "🔧 Quick Commands:"
    echo "=================="
    echo "• View HTML report: open target/coverage/index.html"
    echo "• Run coverage check: cargo coverage"
    echo "• Generate XML report: ./scripts/coverage_report.sh xml"
    echo "• Generate LCOV: ./scripts/coverage_report.sh lcov"
}

# Show usage if requested
if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "Kotoba Coverage Report Generator"
    echo ""
    echo "Usage: $0 [REPORT_TYPE]"
    echo ""
    echo "REPORT_TYPE:"
    echo "  html    Generate HTML coverage report (default)"
    echo "  xml     Generate XML coverage report"
    echo "  lcov    Generate LCOV coverage report"
    echo ""
    echo "Examples:"
    echo "  $0              # Generate HTML report"
    echo "  $0 xml          # Generate XML report"
    echo "  $0 lcov         # Generate LCOV report"
    exit 0
fi

# Run main function
main "$@"

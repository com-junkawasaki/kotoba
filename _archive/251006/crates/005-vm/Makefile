.PHONY: all build run check test clean release

all: build

build:
	@echo "Building the VM..."
	@cargo build

run:
	@echo "Running the VM..."
	@cargo run

check:
	@echo "Checking the VM code..."
	@cargo check

test:
	@echo "Testing the VM..."
	@cargo test

clean:
	@echo "Cleaning the project..."
	@cargo clean

release: check test
	@echo "Building for release..."
	@cargo build --release

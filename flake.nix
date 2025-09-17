# Kotoba - Distributed Graph Database
{
  description = "GP2-based Graph Rewriting Language - ISO GQL-compliant queries, MVCC+Merkle persistence, and distributed execution";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Use a specific Rust version for reproducibility
        rustVersion = "1.82.0";

        rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default.override {
          extensions = [
            "rust-src"      # For IDE support
            "rust-analyzer" # For IDE support
            "clippy"        # Linter
            "rustfmt"       # Formatter
          ];
          targets = [
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
            "wasm32-unknown-unknown",
          ];
        };

        # Build dependencies
        buildInputs = with pkgs; [
          # Rust toolchain
          rustToolchain

          # System dependencies
          pkg-config
          openssl
          libclang

          # Development tools
          git
          jq
          yq
          curl
          wget

          # For cross-compilation (optional)
          gcc
          gnumake
          cmake
          clang

          # For documentation
          graphviz
          plantuml

          # For testing
          cargo-nextest
          cargo-watch
          cargo-expand

          # For benchmarking
          hyperfine
          criterion

          # For JSON processing
          jsonnet
          go-jsonnet
        ] ++
        # Platform-specific tools (may not be available on all systems)
        (pkgs.lib.optionals (pkgs.stdenv.isLinux) [
          docker
          kind
          kubectl
          helm
          kubernetes-helm
        ]);

        # Runtime dependencies
        runtimeInputs = with pkgs; [
          openssl
        ] ++
        (pkgs.lib.optionals (pkgs.stdenv.isLinux) [
          ca-certificates
        ]);

      in
      {
        # Development shell
        devShells.default = pkgs.mkShell {
          inherit buildInputs runtimeInputs;

          # Environment variables
          RUST_BACKTRACE = "1";
          RUST_LOG = "info";
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

          # Shell hook for development setup
          shellHook = ''
            echo "🚀 Welcome to Kotoba development environment!"
            echo "📦 Rust version: $(rustc --version)"
            echo "🔧 Cargo version: $(cargo --version)"
            echo "🐳 Docker version: $(docker --version 2>/dev/null || echo 'Docker not available')"
            echo "☸️  kubectl version: $(kubectl version --client --short 2>/dev/null || echo 'kubectl not available')"
            echo "⎈  helm version: $(helm version --short 2>/dev/null || echo 'helm not available')"
            echo ""
            echo "Available commands:"
            echo "  cargo build          - Build the project"
            echo "  cargo test           - Run tests"
            echo "  cargo clippy         - Run linter"
            echo "  cargo fmt            - Format code"
            ${if pkgs.stdenv.isLinux then ''
            echo "  ./k8s/kind/deploy-local.sh - Deploy locally with Kind"
            echo "  docker build -t kotoba . - Build Docker image"
            '' else ''
            echo "  # Note: Docker/Kind deployment requires Linux"
            echo "  # For local development, use cargo build/test"
            ''}
            echo ""
          '';
        };

        # Package definition for Kotoba
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "kotoba";
          version = "0.1.2";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              # Add any git dependencies here if needed
              # "some-crate-0.1.0" = "sha256-...";
            };
          };

          nativeBuildInputs = [
            rustToolchain
            pkgs.pkg-config
          ];

          buildInputs = runtimeInputs ++ [
            pkgs.openssl.dev
          ];

          # Environment variables for build
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

          # Build features
          cargoBuildFeatures = [ "full" ];

          # Skip tests that require network or external services
          doCheck = false;

          meta = with pkgs.lib; {
            description = "GP2-based Graph Rewriting Language - ISO GQL-compliant queries, MVCC+Merkle persistence, and distributed execution";
            homepage = "https://github.com/com-junkawasaki/kotoba";
            license = licenses.mit;
            maintainers = [ maintainers.jun784 ];
            platforms = platforms.all;
          };
        };

        # Docker image using Nix
        packages.dockerImage = pkgs.dockerTools.buildImage {
          name = "kotoba";
          tag = "latest";

          copyToRoot = pkgs.buildEnv {
            name = "kotoba-root";
            paths = [ self.packages.${system}.default ] ++ runtimeInputs;
            pathsToLink = [ "/bin" ];
          };

          config = {
            Cmd = [ "${self.packages.${system}.default}/bin/kotoba" ];
            WorkingDir = "/app";
            Env = [
              "RUST_LOG=info"
            ];
            ExposedPorts = {
              "3000/tcp" = {};
              "8080/tcp" = {};
            };
          };
        };

        # Apps for easy access
        apps = {
          kotoba = flake-utils.lib.mkApp {
            drv = self.packages.${system}.default;
          };

          docker-build = flake-utils.lib.mkApp {
            drv = pkgs.writeShellScriptBin "docker-build" ''
              ${pkgs.docker}/bin/docker load < ${self.packages.${system}.dockerImage}
            '';
          };
        };

        # Formatter
        formatter = pkgs.nixpkgs-fmt;
      }
    );
}

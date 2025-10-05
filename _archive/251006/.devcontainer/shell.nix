# Fallback shell.nix for systems without flake support
# This is automatically generated from flake.nix

{ system ? builtins.currentSystem }:

let
  lock = builtins.fromJSON (builtins.readFile ./flake.lock);
  nixpkgsSrc = fetchTarball {
    url = "https://github.com/NixOS/nixpkgs/archive/${lock.nodes.nixpkgs.locked.rev}.tar.gz";
    sha256 = lock.nodes.nixpkgs.locked.narHash;
  };

  rust-overlay = fetchTarball {
    url = "https://github.com/oxalica/rust-overlay/archive/${lock.nodes.rust-overlay.locked.rev}.tar.gz";
    sha256 = lock.nodes.rust-overlay.locked.narHash;
  };

  overlays = [
    (import rust-overlay)
  ];

  pkgs = import nixpkgsSrc {
    inherit system overlays;
  };

  rustVersion = "1.82.0";

  rustToolchain = pkgs.rust-bin.stable.${rustVersion}.default.override {
    extensions = [
      "rust-src"
      "rust-analyzer"
      "clippy"
      "rustfmt"
    ];
    targets = [
      "x86_64-unknown-linux-gnu"
      "aarch64-unknown-linux-gnu"
      "x86_64-apple-darwin"
      "aarch64-apple-darwin"
    ];
  };

  buildInputs = with pkgs; [
    rustToolchain
    pkg-config
    openssl
    libclang
    git
    docker
    kind
    kubectl
    helm
    jq
    yq
    curl
    wget
    gcc
    gnumake
    cmake
    clang
    graphviz
    plantuml
    cargo-nextest
    cargo-watch
    cargo-expand
    hyperfine
    criterion
    jsonnet
    go-jsonnet
  ];

  runtimeInputs = with pkgs; [
    openssl
    ca-certificates
  ];

in
pkgs.mkShell {
  inherit buildInputs runtimeInputs;

  RUST_BACKTRACE = "1";
  RUST_LOG = "info";
  LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
  PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

  shellHook = ''
    echo "🚀 Welcome to Kotoba development environment!"
    echo "📦 Rust version: $(rustc --version)"
    echo "🔧 Cargo version: $(cargo --version)"
    echo "🐳 Docker version: $(docker --version 2>/dev/null || echo 'Docker not available')"
    echo "☸️  kubectl version: $(kubectl version --client --short 2>/dev/null || echo 'kubectl not available')"
    echo ""
    echo "Available commands:"
    echo "  cargo build          - Build the project"
    echo "  cargo test           - Run tests"
    echo "  cargo clippy         - Run linter"
    echo "  cargo fmt            - Format code"
    echo "  ./k8s/kind/deploy-local.sh - Deploy locally with Kind"
    echo "  docker build -t kotoba . - Build Docker image"
    echo ""
  '';
}

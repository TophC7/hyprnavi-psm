{
  description = "hyprnavi-psm - Smart navigation tool for Hyprland with edge-detection";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Use stable Rust toolchain
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
          ];
        };

        # Build inputs needed for hyprland-rs
        buildInputs = with pkgs; [
          openssl
        ];

        # Native build inputs
        nativeBuildInputs = with pkgs; [
          pkg-config
          rustToolchain
        ];

      in
      {
        # Development shell
        devShells.default = pkgs.mkShell {
          inherit buildInputs nativeBuildInputs;

          # Environment variables for OpenSSL (needed by some Rust crates)
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";
          PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";

          shellHook = ''
            echo "hyprnavi-psm development shell"
            echo "Rust: $(rustc --version)"
            echo ""
            echo "Available commands:"
            echo "  cargo build        - Build the project"
            echo "  cargo build -r     - Build release binary"
            echo "  cargo clippy       - Run linter"
            echo "  cargo fmt          - Format code"
            echo ""
          '';
        };

        # Package definition
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "hyprnavi-psm";
          version = "0.3.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            # Allow fetching git dependencies
            allowBuiltinFetchGit = true;
          };

          inherit buildInputs nativeBuildInputs;

          # Environment for build
          OPENSSL_DIR = "${pkgs.openssl.dev}";
          OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";
          OPENSSL_INCLUDE_DIR = "${pkgs.openssl.dev}/include";

          meta = with pkgs.lib; {
            description = "Smart navigation tool for Hyprland with edge-detection";
            homepage = "https://github.com/tophc7/hyprnavi-psm";
            license = licenses.mit;
            maintainers = [ "tophc7" ];
            platforms = platforms.linux;
            mainProgram = "hyprnavi";
          };
        };

        # Convenient alias
        packages.hyprnavi-psm = self.packages.${system}.default;
      }
    );
}

{
  description = "lmd Rust development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix.url = "github:nix-community/fenix";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      fenix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ fenix.overlays.default ];
        };

        toolchain = pkgs.fenix.complete.withComponents [
          "cargo"
          "clippy"
          "rust-src"
          "rustc"
          "rustfmt"
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            toolchain
            pkgs.fenix.rust-analyzer
            pkgs.lalrpop
          ];

          RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            export CARGO_INSTALL_ROOT="$PWD/.cargo-tools"
            export PATH="$CARGO_INSTALL_ROOT/bin:$PATH"

            if ! command -v lalrpop-lsp >/dev/null 2>&1; then
              echo "Installing lalrpop-lsp from GitHub into $CARGO_INSTALL_ROOT ..."
              cargo install --locked --git https://github.com/LighghtEeloo/lalrpop-lsp lalrpop-lsp
            fi
          '';
        };
      }
    );
}

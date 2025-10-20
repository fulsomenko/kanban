{ pkgs ? import <nixpkgs> {}, rustToolchain ? pkgs.rustc }:

let
  changeset = pkgs.writeShellScriptBin "changeset" ''
    ${builtins.readFile ./scripts/create-changeset.sh}
  '';
in

pkgs.mkShell {
  name = "kanban-rust-shell";

  buildInputs = with pkgs; [
    # Rust toolchain
    rustToolchain
    cargo-watch
    cargo-edit
    cargo-audit
    cargo-tarpaulin

    # Development utilities
    bacon
    changeset
  ];

  shellHook = ''
    export RUST_BACKTRACE=1
    echo "Kanban Development Environment"
    echo "📦 Cargo: $(cargo --version)"
    echo "🦀 Rustc: $(rustc --version)"
  '';
}


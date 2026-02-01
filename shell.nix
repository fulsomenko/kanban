{
  pkgs ? import <nixpkgs> {},
  rustToolchain ? pkgs.rustc,
  changeset ? null,
  aggregateChangelog ? null,
  bumpVersion ? null,
  publishCrates ? null,
  validateRelease ? null,
}:

let
  scripts = builtins.filter (x: x != null) [
    changeset
    aggregateChangelog
    bumpVersion
    publishCrates
    validateRelease
  ];
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

    asciinema_3
    asciinema-agg
  ] ++ scripts;

  shellHook = ''
    export RUST_BACKTRACE=1
    echo "Kanban Development Environment"
    echo "📦 Cargo: $(cargo --version)"
    echo "🦀 Rustc: $(rustc --version)"
  '';
}

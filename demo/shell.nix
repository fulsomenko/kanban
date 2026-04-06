{ pkgs ? import <nixpkgs> {} }:

let
  demoDir = builtins.toString ./.;
in

pkgs.mkShell {
  name = "kanban-demo-shell";

  buildInputs = with pkgs; [
    vhs
    cargo
    neovim
  ];

  shellHook = ''
    export EDITOR="${demoDir}/nvim-editor.sh"
    echo "Kanban Demo Environment"
    echo "📹 VHS: $(vhs --version)"
    echo "✎ Editor: nvim via ${demoDir}/nvim-editor.sh"
    echo ""
    echo "To build and record the demo:"
    echo "  1. cargo build --release"
    echo "  2. nix-shell demo/shell.nix --run 'bash demo/record.sh'"
  '';
}

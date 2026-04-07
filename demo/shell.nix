{ pkgs }:

let
  demoDir = builtins.toString ./.;
in

pkgs.mkShell {
  name = "kanban-demo-shell";

  buildInputs = [ pkgs.vhs pkgs.neovim ];

  shellHook = ''
    export EDITOR="${demoDir}/nvim-editor.sh"
    echo "Kanban Demo Environment"
    echo "📹 VHS: $(vhs --version)"
    echo "✎ Editor: nvim via ${demoDir}/nvim-editor.sh"
    echo ""
    echo "To build and record the demo:"
    echo "  1. cargo build --release"
    echo "  2. nix develop .#demo --command bash demo/record.sh"
  '';
}

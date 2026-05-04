{ pkgs, kanban }:

let
  demoDir = builtins.toString ./.;
in

pkgs.mkShell {
  name = "kanban-demo-shell";

  buildInputs = [ pkgs.vhs pkgs.neovim kanban ];

  shellHook = ''
    export EDITOR="${demoDir}/nvim-editor.sh"
    echo "Kanban Demo Environment"
    echo "📹 VHS: $(vhs --version)"
    echo "✎ Editor: nvim via ${demoDir}/nvim-editor.sh"
    echo ""
    echo "To record the demo:"
    echo "  nix develop .#demo --command bash demo/record.sh"
  '';
}

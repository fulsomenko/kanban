{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "rust-analyzer" "clippy" "rustfmt"];
        };

        changeset = pkgs.writeShellScriptBin "changeset" ''
          ${builtins.readFile ./scripts/create-changeset.sh}
        '';

        bumpVersion = pkgs.writeShellScriptBin "bump-version" ''
          ${builtins.readFile ./scripts/bump-version.sh}
        '';

        publishCrates = pkgs.writeShellScriptBin "publish-crates" ''
          ${builtins.readFile ./scripts/publish-crates.sh}
        '';

        validateRelease = pkgs.writeShellScriptBin "validate-release" ''
          ${builtins.readFile ./scripts/validate-release.sh}
        '';
      in {
        devShells.default = import ./shell.nix {
          inherit pkgs rustToolchain;
        };

        packages = {
          default = pkgs.callPackage ./default.nix {};
          bump-version = bumpVersion;
          publish-crates = publishCrates;
          validate-release = validateRelease;
          changset = changeset;
        };
      }
    );
}


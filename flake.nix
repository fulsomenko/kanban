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

        changeset = pkgs.writeShellApplication {
          name = "changeset";
          runtimeInputs = with pkgs; [coreutils];
          text = builtins.readFile ./scripts/create-changeset.sh;
        };

        bumpVersion = pkgs.writeShellApplication {
          name = "bump-version";
          runtimeInputs = with pkgs; [rustToolchain cargo coreutils gnugrep gnused git findutils];
          text = builtins.readFile ./scripts/bump-version.sh;
        };

        publishCrates = pkgs.writeShellApplication {
          name = "publish-crates";
          runtimeInputs = [rustToolchain pkgs.cargo pkgs.coreutils validateRelease];
          text = builtins.readFile ./scripts/publish-crates.sh;
        };

        validateRelease = pkgs.writeShellApplication {
          name = "validate-release";
          runtimeInputs = with pkgs; [rustToolchain cargo coreutils gnugrep gnused];
          text = builtins.readFile ./scripts/validate-release.sh;
        };
      in {
        devShells.default = import ./shell.nix {
          inherit pkgs rustToolchain;
        };

        packages = {
          default = pkgs.callPackage ./default.nix {};
          bump-version = bumpVersion;
          publish-crates = publishCrates;
          validate-release = validateRelease;
          changeset = changeset;
        };
      }
    );
}


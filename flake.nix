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
          extensions = ["rust-src" "rust-analyzer"];
        };
      in {
        devShells.default = import ./shell.nix {
          inherit pkgs rustToolchain;
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "kanban";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            openssl
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
          ];

          meta = with pkgs.lib; {
            description = "A terminal-based kanban board";
            homepage = "https://github.com/fulsomenko/kanban";
            license = licenses.asl20;
            maintainers = [ ];
            mainProgram = "kanban";
            platforms = platforms.all;
          };
        };
      }
    );
}


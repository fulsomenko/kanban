{
  lib,
  pkgs,
  rustPlatform,
  src,
}:
let
  cargoToml = lib.importTOML ../Cargo.toml;
in
rustPlatform.buildRustPackage {
  pname = "kanban-tests";
  inherit (cargoToml.workspace.package) version;

  inherit src;

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs = [ pkgs.pkg-config ];
  buildInputs =
    lib.optionals pkgs.stdenv.isLinux [
      pkgs.wayland
      pkgs.xorg.libxcb
    ]
    ++ lib.optionals pkgs.stdenv.isDarwin [
      pkgs.apple-sdk
    ];

  cargoBuildFlags = [ "--workspace" "--all-features" ];
  cargoTestFlags = [ "--workspace" "--all-features" ];
  doCheck = true;

  # This derivation exists solely to exercise the test suite hermetically;
  # nothing needs to be installed.
  installPhase = ''
    runHook preInstall
    touch $out
    runHook postInstall
  '';

  meta = {
    description = "Hermetic test runner for the kanban workspace";
    platforms = lib.platforms.all;
  };
}

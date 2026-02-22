{
  lib,
  pkgs,
  craneLib,
}:

let
  cargoToml = lib.importTOML ./Cargo.toml;

  src = lib.cleanSourceWith {
    src = ./.;
    filter = path: type:
      (lib.hasSuffix "\.rs" path) ||
      (lib.hasSuffix "\.toml" path) ||
      (lib.hasInfix "/Cargo.lock" path) ||
      (type == "directory");
  };

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src;
    pname = "kanban";
    version = cargoToml.workspace.package.version;

    nativeBuildInputs = [ pkgs.pkg-config ];
    buildInputs = lib.optionals pkgs.stdenv.isLinux [
      pkgs.wayland
      pkgs.xorg.libxcb
    ];
  };

in
craneLib.buildPackage {
  inherit src cargoArtifacts;
  pname = "kanban";
  version = cargoToml.workspace.package.version;

  nativeBuildInputs = [ pkgs.pkg-config ];
  buildInputs = lib.optionals pkgs.stdenv.isLinux [
    pkgs.wayland
    pkgs.xorg.libxcb
  ];

  cargoExtraArgs = "--package kanban-cli";
  doCheck = false;

  meta = {
    inherit (cargoToml.workspace.package) description homepage;
    license = lib.licenses.asl20;
    maintainers = with lib.maintainers; [ fulsomenko ];
    mainProgram = "kanban";
    platforms = lib.platforms.all;
  };
}

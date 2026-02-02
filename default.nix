{
  lib,
  rustPlatform,
}:

let
  cargoToml = lib.importTOML ./Cargo.toml;
in
rustPlatform.buildRustPackage {
  pname = "kanban";
  inherit (cargoToml.workspace.package) version;

  src = lib.cleanSource ./.;

  cargoLock.lockFile = ./Cargo.lock;

  cargoBuildFlags = [ "--package" "kanban-cli" ];
  doCheck = false;

  meta = {
    inherit (cargoToml.workspace.package) description homepage;
    license = lib.licenses.asl20;
    maintainers = with lib.maintainers; [ fulsomenko ];
    mainProgram = "kanban";
    platforms = lib.platforms.all;
  };
}

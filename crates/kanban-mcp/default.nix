{ lib
, rustPlatform
}:

let
  cargoToml = lib.importTOML ../../Cargo.toml;
in
rustPlatform.buildRustPackage {
  inherit (cargoToml.workspace.package) version;
  pname = "kanban-mcp";

  src = lib.cleanSource ../..;

  cargoLock = {
    lockFile = ../../Cargo.lock;
  };

  # Only build the kanban-mcp binary
  cargoBuildFlags = [ "--package" "kanban-mcp" ];
  cargoTestFlags = [ "--package" "kanban-mcp" ];

  meta = {
    inherit (cargoToml.workspace.package) description homepage;
    license = lib.licenses.asl20;
    maintainers = with lib.maintainers; [ fulsomenko ];
    mainProgram = "kanban-mcp";
    platforms = lib.platforms.all;
  };
}

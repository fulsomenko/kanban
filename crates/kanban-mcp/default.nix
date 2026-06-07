{ lib
, rustPlatform
, src
, gitRev ? null
}:

let
  cargoToml = lib.importTOML ../../Cargo.toml;
in
rustPlatform.buildRustPackage {
  inherit (cargoToml.workspace.package) version;
  pname = "kanban-mcp";

  inherit src;

  cargoLock = {
    lockFile = ../../Cargo.lock;
  };

  preBuild = lib.optionalString (gitRev != null) ''
    export GIT_COMMIT_HASH="${gitRev}"
  '';

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

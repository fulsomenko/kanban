{ lib
, rustPlatform
, makeWrapper
, kanban
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

  nativeBuildInputs = [ makeWrapper ];
  nativeCheckInputs = [ kanban ];

  # Only build the kanban-mcp binary
  cargoBuildFlags = [ "--package" "kanban-mcp" ];
  cargoTestFlags = [ "--package" "kanban-mcp" ];

  # Point integration tests to the Nix-built kanban binary
  KANBAN_BIN = lib.getExe kanban;

  # Wrap the binary to include kanban CLI in PATH
  postInstall = ''
    wrapProgram $out/bin/kanban-mcp \
      --prefix PATH : ${lib.makeBinPath [ kanban ]}
  '';

  meta = {
    inherit (cargoToml.workspace.package) description homepage;
    license = lib.licenses.asl20;
    maintainers = with lib.maintainers; [ fulsomenko ];
    mainProgram = "kanban-mcp";
    platforms = lib.platforms.all;
  };
}

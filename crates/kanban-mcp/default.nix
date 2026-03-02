{
  lib,
  craneLib,
  makeWrapper,
  kanban,
}:

let
  cargoToml = lib.importTOML ../../Cargo.toml;

  src = lib.cleanSourceWith {
    src = ../..;
    filter = path: type:
      (lib.hasSuffix "\.rs" path) ||
      (lib.hasSuffix "\.toml" path) ||
      (lib.hasInfix "/Cargo.lock" path) ||
      (type == "directory");
  };

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src;
    pname = "kanban-mcp";
    version = cargoToml.workspace.package.version;
  };

in
craneLib.buildPackage {
  inherit src cargoArtifacts;
  pname = "kanban-mcp";
  version = cargoToml.workspace.package.version;

  nativeBuildInputs = [ makeWrapper ];
  nativeCheckInputs = [ kanban ];

  cargoExtraArgs = "--package kanban-mcp";
  cargoTestExtraArgs = "--package kanban-mcp";

  KANBAN_BIN = lib.getExe kanban;

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

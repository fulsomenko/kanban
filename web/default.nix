{
  lib,
  stdenv,
}:

let
  cargoVersion = (lib.importTOML ../Cargo.toml).workspace.package.version;
  demoSrc = ../demo;
in
stdenv.mkDerivation {
  pname = "kanban-web";
  version = cargoVersion;

  src = lib.cleanSource ./.;

  buildPhase = ''
    substitute index.html index.html.out \
      --replace-fail "@VERSION@" "${cargoVersion}"
  '';

  installPhase = ''
    mkdir -p $out/demo
    cp index.html.out $out/index.html
    cp styles.css $out/
    cp ${demoSrc}/demo.svg $out/demo/demo.svg
  '';

  meta = {
    description = "Kanban landing page - keyboard-driven project management for developers";
    homepage = "https://kanban.yoon.se";
    license = lib.licenses.asl20;
    maintainers = with lib.maintainers; [ fulsomenko ];
    platforms = lib.platforms.all;
  };
}

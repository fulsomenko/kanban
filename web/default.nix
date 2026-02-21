{
  lib,
  stdenv,
}:

stdenv.mkDerivation {
  pname = "kanban-web";
  version = "1.0.0";

  src = lib.cleanSource ./.;

  dontBuild = true;

  installPhase = ''
    mkdir -p $out
    cp index.html $out/
    cp styles.css $out/
    cp demo.gif $out/
  '';

  meta = {
    description = "Kanban landing page - keyboard-driven project management for developers";
    homepage = "https://kanban.yoon.se";
    license = lib.licenses.asl20;
    maintainers = with lib.maintainers; [ fulsomenko ];
    platforms = lib.platforms.all;
  };
}

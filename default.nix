{
  lib,
  fetchFromGitHub,
  rustPlatform,
  nix-update-script,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "kanban";
  version = "0.1.15";

  src = fetchFromGitHub {
    owner = "fulsomenko";
    repo = "kanban";
    rev = "f81983111a0279958e8e6b407cc8d05c78015aeb";
    hash = "sha256-N5e+yzM8thYGmIcVpmF4aSE5ZscONwGgH0k1u68VS2U=";
  };

  GIT_COMMIT_HASH = finalAttrs.src.rev;

  cargoHash = "sha256-Q/o5MHjVRrJpfhkzNNJ6j4oASV5wDg/0Zi43zPlp5p8=";

  passthru.updateScript = nix-update-script { };

  meta = {
    description = "Terminal-based project management solution";
    longDescription = ''
      A terminal-based kanban/project management tool inspired by lazygit,
      built with Rust. Features include file persistence, keyboard-driven
      navigation, multi-select capabilities, and sprint management.
    '';
    homepage = "https://github.com/fulsomenko/kanban";
    license = lib.licenses.asl20;
    maintainers = with lib.maintainers; [ fulsomenko ];
    mainProgram = "kanban";
    platforms = lib.platforms.all;
  };
})

{ lib, buildGoModule, fetchFromGitHub }:

buildGoModule {
  pname = "vhs";
  version = "0.11.1-agentstation";

  src = fetchFromGitHub {
    owner = "agentstation";
    repo = "vhs";
    rev = "v0.11.1";
    hash = "sha256-VqNTRRFk/kZPGn/mpejXFR7ui+878Z3gx+TUlRtn8/0=";
  };

  vendorHash = "sha256-WiCSn84cr42yQFgg36H/NrVsfiBA/ZDAGd0WmC6LAa4=";

  ldflags = [ "-s" "-w" "-X main.Version=v0.11.1" ];
}

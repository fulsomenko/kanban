{ lib, buildGoModule, fetchFromGitHub }:

buildGoModule {
  pname = "vhs";
  version = "0.11.1-agentstation";

  src = fetchFromGitHub {
    owner = "agentstation";
    repo = "vhs";
    rev = "v0.11.1";
    hash = lib.fakeHash;
  };

  vendorHash = lib.fakeHash;
}

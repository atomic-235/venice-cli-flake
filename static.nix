{
  lib,
  pkgs,
  ...
}:

# Built by CI, not by consumers: the resulting binary is published as the
# release asset that package.nix fetches.
#
# pkgsStatic gives a fully static musl binary with no /nix/store references,
# so a plain fixed-output fetch is enough on the consumer side.
let
  data = builtins.fromJSON (builtins.readFile ./version.json);
in
pkgs.pkgsStatic.rustPlatform.buildRustPackage {
  pname = "venice";
  version = data.version;

  src = lib.cleanSourceWith {
    src = ./.;
    filter = path: _: baseNameOf path != "target";
  };
  cargoLock.lockFile = ./Cargo.lock;

  doCheck = false;

  meta = with lib; {
    description = "Venice AI CLI";
    homepage = "https://github.com/atomic-235/venice-cli-flake";
    license = licenses.mit;
    platforms = [ "x86_64-linux" ];
    mainProgram = "venice";
  };
}

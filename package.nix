{
  lib,
  rustPlatform,
  makeWrapper,
  mpv,
  imv,
  xdg-utils,
}:

rustPlatform.buildRustPackage {
  pname = "venice";
  version = "0.2.0";

  src = lib.cleanSourceWith {
    src = ./.;
    filter = path: _: baseNameOf path != "target";
  };

  cargoLock.lockFile = ./Cargo.lock;

  nativeBuildInputs = [ makeWrapper ];

  postInstall = ''
    wrapProgram $out/bin/venice \
      --prefix PATH : ${lib.makeBinPath [ mpv imv xdg-utils ]}
  '';

  meta = with lib; {
    description = "Venice AI CLI";
    license = licenses.mit;
    mainProgram = "venice";
    platforms = platforms.unix;
  };
}

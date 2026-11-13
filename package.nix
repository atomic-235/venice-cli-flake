{
  lib,
  stdenvNoCC,
  fetchurl,
  makeWrapper,
  mpv,
  imv,
  xdg-utils,
}:

let
  data = builtins.fromJSON (builtins.readFile ./version.json);
  system = stdenvNoCC.hostPlatform.system;

  asset =
    {
      x86_64-linux = "venice-linux-x86_64";
    }
    .${system} or (throw "venice: unsupported system ${system}");
in
stdenvNoCC.mkDerivation {
  pname = "venice";
  version = data.version;

  # Static musl binary with zero /nix/store references, so a plain
  # fixed-output fetch is enough.
  src = fetchurl {
    url = "https://github.com/atomic-235/venice-cli-flake/releases/download/v${data.version}/${asset}";
    hash = data.assets.${system};
  };

  dontUnpack = true;
  nativeBuildInputs = [ makeWrapper ];

  installPhase = ''
    runHook preInstall
    install -Dm755 "$src" "$out/bin/venice"
    wrapProgram $out/bin/venice \
      --prefix PATH : ${lib.makeBinPath [ mpv imv xdg-utils ]}
    runHook postInstall
  '';

  meta = with lib; {
    description = "Venice AI CLI (static release binary)";
    homepage = "https://github.com/atomic-235/venice-cli-flake";
    license = licenses.mit;
    platforms = [ "x86_64-linux" ];
    mainProgram = "venice";
  };
}

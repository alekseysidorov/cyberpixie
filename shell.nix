# Shell for the platform independent code parts
{ localSystem ? builtins.currentSystem
, pkgs ? import ./nix { inherit localSystem; }
}:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustToolchain
    rustBuildHostDependencies
    # Dependencies for the code formatting utility
    dprint
    nixpkgs-fmt
    qt6.full
  ] ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.OpenGL
  ];

  shellHook = "${pkgs.crossBashPrompt}";
}

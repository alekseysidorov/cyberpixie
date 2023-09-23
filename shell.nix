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
  ]
  # Additional frameworks for the Qt application
  ++ lib.optionals stdenv.isDarwin [
    darwin.apple_sdk.frameworks.AppKit
    darwin.apple_sdk.frameworks.OpenGL
    darwin.apple_sdk.frameworks.AGL
    darwin.apple_sdk.frameworks.CoreBluetooth
  ];

  shellHook = "${pkgs.crossBashPrompt}";
}

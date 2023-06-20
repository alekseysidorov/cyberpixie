# Shell for the platform independent code parts
{ localSystem ? builtins.currentSystem
, pkgs ? import ./../../../nix { inherit localSystem; }
}:
let
  # Read a cargo build configuration toml
  shellPrompt = pkgs.mkBashPrompt "esp32c3";
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustToolchain
    rustBuildHostDependencies
    # Utilites to flash firmware to the device
    espflash
    cargo-espflash
  ];

  shellHook = ''
    # Setup nice bash prompt
    ${shellPrompt}
  '';
}

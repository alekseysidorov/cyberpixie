# Shell for the platform independent code parts
{ localSystem ? builtins.currentSystem
, pkgs ? import ./../../../nix { inherit localSystem; }
}:
let
  # Read a cargo build configuration toml
  shellPrompt = pkgs.mkBashPrompt "esp32s3";
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustBuildHostDependencies
    # Special case for the xtensa toolchain
    espup
    rustup
    # Utilites to flash firmware to the device
    espflash
    cargo-espflash
  ];

  shellHook =
    let
      setupToolchain = builtins.readFile ./scripts/setup-rust-toolchain.sh;
    in
    ''
      ${setupToolchain}
      # Setup nice bash prompt
      ${shellPrompt}
    '';
}

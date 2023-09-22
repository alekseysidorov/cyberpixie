# Shell for the platform independent code parts
{ localSystem ? builtins.currentSystem
, pkgs ? import ./../../../nix { inherit localSystem; }
}:
let
  boardTarget = "riscv32imc-unknown-none-elf";
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustToolchain
    rustBuildHostDependencies
    zlib
    # Utilites to flash firmware to the device
    espflash
    cargo-espflash
  ];

  # Force cargo build target to make sure that the vscode will use it as well
  env.CARGO_BUILD_TARGET = boardTarget;

  shellHook = ''
    # Setup nice bash prompt
    ${pkgs.mkBashPrompt boardTarget}
  '';
}

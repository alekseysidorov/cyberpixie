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
    # Utilites to flash firmware to the device
    cargo-espflash
  ];

  # Force cargo build target to make sure that the vscode will use it as well
  env.CARGO_BUILD_TARGET = boardTarget;
  # Force cargo workspace to enable build-std feature for this crate
  env.CARGO_UNSTABLE_BUILD_STD = "core";

  shellHook = ''
    # Setup nice bash prompt
    ${pkgs.mkBashPrompt boardTarget}
  '';
}

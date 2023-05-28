# Shell for the platform independent code parts
{ localSystem ? builtins.currentSystem
, pkgs ? import ./../../nix { inherit localSystem; }
}:
let
  # Read a cargo build configuration toml
  cargoConfigUtils = pkgs.cargoConfigUtils.fromFile ./.cargo/config.toml;
  shellPrompt = pkgs.mkBashPrompt cargoConfigUtils.target;
in
pkgs.mkShell {

  env = cargoConfigUtils.env;
  shellHook = ''
      # Setup nice bash prompt
    ${shellPrompt}
  '';
}

# Shell for the platform independent code parts
{ localSystem ? builtins.currentSystem
, pkgs ? import ./../../nix { inherit localSystem; }
, target ? null
}:
let
  # Read a cargo build configuration toml
  cargoConfigUtils = pkgs.cargoConfigUtils.fromFile ./.cargo/config.toml;
  # Set a cargo build target
  cargoTarget =
    if target == null then cargoConfigUtils.target
    else target;

  # Extra shell hook for the xtensa targets
  extraShellHook =
    if target == "xtensa-esp32s3-espidf"
    then
      ''
        # Install esp toolchain
        espup install -f /tmp/export-esh.sh -t esp32s3
        . /tmp/export-esh.sh
        # Override rustup toolchain to esp
        export RUSTUP_TOOLCHAIN=esp
      ''
    else '''';
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustBuildHostDependencies
    # For compiling the esp-idf library
    python310Packages.virtualenv
    python310Packages.pip
    ldproxy
    # Utilites to flash firmware to the device
    espflash
    cargo-espflash
  ];
  env = cargoConfigUtils.env // {
    # Override cargo build target
    CARGO_BUILD_TARGET = cargoTarget;
  };
  shellHook = ''
    # Disable Native compiler in shell
    unset CC; unset CXX
    # Invoke target specific shell hook
    ${extraShellHook}
    # Setup nice bash prompt
    ${pkgs.mkBashPrompt cargoTarget}
  '';
}

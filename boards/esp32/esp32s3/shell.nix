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

  shellHook = ''
    # Install esp toolchain
    ESP_TARGET_DIR="../target/esp"
    EXPORT_FILE="$ESP_TARGET_DIR/export-esh.sh"
    if [ ! -f "$EXPORT_FILE" ]; then
        mkdir -p "$ESP_TARGET_DIR"
        espup install -f $EXPORT_FILE -t esp32s3
    else
        espup update
    fi
    # Export variables
    . $EXPORT_FILE
    export RUSTUP_TOOLCHAIN="esp"
    # Force cargo build target to make sure that the vscode will use it as well
    export CARGO_BUILD_TARGET="xtensa-esp32s3-none-elf"
    # Setup nice bash prompt
    ${shellPrompt}
  '';
}

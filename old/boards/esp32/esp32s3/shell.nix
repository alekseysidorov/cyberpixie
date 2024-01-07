# Shell for the platform independent code parts
{ localSystem ? builtins.currentSystem
, pkgs ? import ./../../../nix { inherit localSystem; }
}:
let
  boardTarget = "xtensa-esp32s3-none-elf";
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustBuildHostDependencies
    # Special case for the xtensa toolchain
    rustup
    espup
    # Utilites to flash firmware to the device
    cargo-espflash
  ];

  # Force cargo build target to make sure that the vscode will use it as well
  env.CARGO_BUILD_TARGET = boardTarget;
  # Force cargo workspace to enable build-std feature for this crate
  env.CARGO_UNSTABLE_BUILD_STD = "core";
  # Override Rust toolchain target
  env.RUSTUP_TOOLCHAIN = "esp";

  shellHook = ''
    # Install esp toolchain
    ESP_TARGET_DIR=$(realpath "../../../target/esp")
    EXPORT_FILE="$ESP_TARGET_DIR/export-esh.sh"
    if [ -f "$EXPORT_FILE" ]; then
      espup update
    else
      mkdir -p "$ESP_TARGET_DIR"
      espup install -f $EXPORT_FILE -t esp32s3
    fi
    # Export variables
    . $EXPORT_FILE
    # Setup nice bash prompt
    ${pkgs.mkBashPrompt boardTarget}
  '';
}

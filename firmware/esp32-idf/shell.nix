# Shell for the platform independent code parts
{ localSystem ? builtins.currentSystem
, pkgs ? import ./../../nix { inherit localSystem; }
, target ? "esp32c3"
}:
let
  # Partitions file.
  partitions = ./partitions.csv;
  # Firmware runner command.
  firmwareRunner = "espflash flash --monitor --partition-table ${partitions}";
  # Target specific variables and shell hooks.
  targetConfig = {
    esp32c3 = {
      shellHook = ''
        # Disable Native compiler in shell
        unset CC; unset CXX
      '';
      env = {
        # Setup the target specific build configuration.
        CARGO_BUILD_TARGET = "riscv32imc-esp-espidf";
        CARGO_TARGET_RISCV32IMC_ESP_ESPIDF_LINKER = "ldproxy";
        CARGO_TARGET_RISCV32IMC_ESP_ESPIDF_RUSTFLAGS = "-C default-linker-libraries";
        CARGO_TARGET_RISCV32IMC_ESP_ESPIDF_RUNNER = firmwareRunner;
        # It's impossible to use the rustPlatform.bindgenHook, 
        # but we have to provide the path to the libclang anyway.
        LIBCLANG_PATH = "${pkgs.llvmPackages_14.libclang.lib}/lib";
      };
    };

    esp32s3 = {
      shellHook = ''
        # Disable Native compiler in shell
        unset CC; unset CXX
        # Install esp toolchain
        espup install -f /tmp/export-esh.sh
        . /tmp/export-esh.sh
      '';
      env = {
        CARGO_BUILD_TARGET = "xtensa-esp32s3-espidf";
        CARGO_TARGET_XTENSA-ESP32S3_LINKER = "ldproxy";
        CARGO_TARGET_XTENSA-ESP32S3_RUNNER = firmwareRunner;
        RUSTUP_TOOLCHAIN="esp";
      };
    };
  };

  shellPrompt = pkgs.mkBashPrompt target;
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

  env = {
    # Enable unstable cargo features
    CARGO_UNSTABLE_BUILD_STD = "std,panic_abort";
    # Builds against ESP-IDF stable (v4.4)
    ESP_IDF_VERSION = "release/v4.4";
  } // targetConfig.${target}.env;

  shellHook = ''
    # Disable Native compiler in shell
    unset CC; unset CXX
    # Invoke target specific shell hook
    ${targetConfig.${target}.shellHook}
    # Setup nice bash prompt
    ${shellPrompt}
  '';
}

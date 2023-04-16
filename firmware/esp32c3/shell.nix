# Shell for the platform independent code parts
{ localSystem ? builtins.currentSystem
, pkgs ? import ./../../nix { inherit localSystem; }
}:
let
  partitionTable = ./partitionTable.csv;
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustToolchain
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
    # Setup the target specific build configuration
    CARGO_BUILD_TARGET = "riscv32imc-esp-espidf";
    CARGO_TARGET_RISCV32IMC_ESP_ESPIDF_LINKER = "ldproxy";
    CARGO_TARGET_RISCV32IMC_ESP_ESPIDF_RUSTFLAGS = "-C default-linker-libraries";
    # Setup the target runnner
    CARGO_TARGET_RISCV32IMC_ESP_ESPIDF_RUNNER = "espflash flash --monitor --partition-table ${partitionTable}";
    # Builds against ESP-IDF stable (v4.4)
    ESP_IDF_VERSION = "release/v4.4";
    # It's impossible to use the rustPlatform.bindgenHook, 
    # but we have to provide the path to the libclang anyway.
    LIBCLANG_PATH = "${pkgs.llvmPackages_14.libclang.lib}/lib";
  };

  shellHook = ''
    # Disable Native compiler in shell
    unset CC; unset CXX
    ${pkgs.mkBashPrompt "esp32c3"}
  '';
}

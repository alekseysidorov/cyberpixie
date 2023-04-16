# Shell for the platform independent code parts
{ localSystem ? builtins.currentSystem
, pkgs ? import ./../../nix { inherit localSystem; }
}:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    rustToolchain
    rustBuildHostDependencies
    rustPlatform.bindgenHook
    # Dependencies for the code formatting utility
    dprint
    # For compiling the esp-idf library
    cmake
  ];

  # Enable unstable cargo features
  CARGO_UNSTABLE_BUILD_STD = "std,panic_abort";
  # Setup the target specific build configuration
  CARGO_BUILD_TARGET = "riscv32imc-esp-espidf";
  CARGO_TARGET_RISCV32IMC_ESP_ESPIDF_LINKER = "ldproxy";
  CARGO_TARGET_RISCV32IMC_ESP_ESPIDF_RUSTFLAGS = "-C default-linker-libraries";
  # Setup the target runnner
  CARGO_TARGET_RISCV32IMC_ESP_ESPIDF_RUNNER = "espflash flash --monitor --partition-table $PWD/partitionTable.csv";
  # Builds against ESP-IDF stable (v4.4)
  ESP_IDF_VERSION = "release/v4.4";

  shellHook = ''
    # Disable Native compiler in shell
    unset CC; unset CXX
    PS1="\[\033[38;5;39m\]\w \[\033[38;5;35m\](esp32c3) \[\033[0m\]\$ "
  '';
}

{ pkgs }:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    (rust-bin.fromRustupToolchainFile ./rust-toolchain.toml)
    rustBuildHostDependencies
    cmake
    sccache
    # Esp32 development packages
    cargo-espflash
    espflash
    ldproxy
    espup
  ];

  RUSTC_WRAPPER = "sccache";

  shellHook = ''
    # Disable Native compiler in shell
    unset CC; unset CXX
  '';
}

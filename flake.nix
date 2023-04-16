{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    rust-esp32 = {
      url = "github:alekseysidorov/nixpkgs-rust-esp32";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
    nixpkgs-cross-overlay = {
      url = "github:alekseysidorov/nixpkgs-cross-overlay/dev";
      inputs = {
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
      };
    };
  };

  outputs = { flake-utils, nixpkgs, rust-esp32, nixpkgs-cross-overlay, ... }: { } // flake-utils.lib.eachDefaultSystem
    (localSystem:
      let
        pkgs = import nixpkgs {
          inherit localSystem;
          overlays = [
            rust-esp32.overlays.default
            nixpkgs-cross-overlay.overlays.default
            (final: prev: {
              rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
            })
          ];
        };

      in
      {
        devShells = {
          default = pkgs.callPackage ./shell.nix { };
          esp32c3 = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [
              rustToolchain
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

            CARGO_BUILD_TARGET = "riscv32imc-esp-espidf";
            CARGO_UNSTABLE_BUILD_STD = "std,panic_abort";
            ESP_IDF_VERSION = "release/v4.4";

            shellHook = ''
              # Disable Native compiler in shell
              unset CC; unset CXX
              PS1="\[\033[38;5;39m\]\w \[\033[38;5;35m\](esp32c3) \[\033[0m\]\$ "
              echo "Hello"
            '';
          };
        };
      }
    );
}

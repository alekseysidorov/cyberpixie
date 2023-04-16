{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
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
    nixpkgs-cross-overlay = {
      url = "github:alekseysidorov/nixpkgs-cross-overlay/dev";
      inputs = {
        flake-utils.follows = "flake-utils";
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
      };
    };
  };

  outputs = { flake-utils, ... }: { } // flake-utils.lib.eachDefaultSystem
    (localSystem:
      {
        devShells = {
          default = import ./shell.nix { inherit localSystem; };
          esp32c3 = import ./firmware/esp32c3/shell.nix { inherit localSystem; };
        };
      }
    );
}

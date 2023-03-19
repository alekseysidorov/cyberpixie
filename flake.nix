{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
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
          ];
        };
      in
      {
        devShells = {
          default = import ./shell.nix { inherit pkgs; };
        };
      }
    );
}

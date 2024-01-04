{
  inputs = {
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
    nixpkgs-cross-overlay = {
      url = "github:alekseysidorov/nixpkgs-cross-overlay/dev";
    };

    treefmt-nix.url = "github:numtide/treefmt-nix";
    flake-root.url = "github:srid/flake-root";
  };

  outputs = { flake-utils, ... }: { } // flake-utils.lib.eachDefaultSystem
    (localSystem:
      {
        devShells = {
          default = import ./shell.nix { inherit localSystem; };
          esp32c3 = import ./boards/esp32/esp32c3/shell.nix { inherit localSystem; };
          esp32s3 = import ./boards/esp32/esp32s3/shell.nix { inherit localSystem; };
        };
      }
    );
}

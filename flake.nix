{
  inputs = {
    flake-utils = {
      url = "github:numtide/flake-utils";
    };
    nixpkgs-cross-overlay = {
      url = "github:alekseysidorov/nixpkgs-cross-overlay/dev";
    };
  };

  outputs = { flake-utils, ... }: { } // flake-utils.lib.eachDefaultSystem
    (localSystem:
      {
        devShells = {
          default = import ./shell.nix { inherit localSystem; };
          esp32c3 = import ./boards/esp32c3/shell.nix { inherit localSystem; };
        };
      }
    );
}

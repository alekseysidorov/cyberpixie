# Definition of Nix packages compatible with flakes and traditional workflow.
let
  lockFile = import ./flake-lock.nix { src = ./..; };
in
{ localSystem ? builtins.currentSystem
, crossSystem ? null
, src ? lockFile.nixpkgs
, config ? { }
, overlays ? [ ]
}:
let
  # Import local packages.
  pkgs = import src {
    inherit localSystem config;
    # Setup overlays
    overlays = [
      (import lockFile.nixpkgs-cross-overlay)
      (import lockFile.rust-overlay)
      # Local overlay with an additional methods.
      (final: prev: {
        # Setup Rust toolchain in according with the toolchain file 
        rustToolchain = prev.rust-bin.fromRustupToolchainFile ./../rust-toolchain.toml;
      })
    ] ++ overlays;
  };
in
pkgs

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
        # Extracts cargo target infix from the given target triple
        toEnvInfix = cargoBuildTarget: builtins.replaceStrings [ "-" "." ] [ "_" "_" ] (final.lib.toUpper cargoBuildTarget);
        # Converts a flags array to the env variable
        mkEnvFromFlags = flags: builtins.concatStringsSep " " flags;
        unstableFlagsToEnv = flags: cargoTarget: final.lib.attrsets.foldlAttrs
          (a: name: value: {
            "CARGO_UNSTABLE_${final.toEnvInfix name}" = final.mkEnvFromFlags value;
          } // a)
          { }
          flags;

        cargoConfigUtils = {
          fromFile = filePath:
            let
              lib = prev.lib;
              # Read the cargo config variables
              config = builtins.fromTOML (builtins.readFile filePath);
              # Get the build target triple(it should be set)
              target = config.build.target;

              mkEnvFromTargetConfig = config: targetInfix:
                let
                  asIs = foo: foo;
                  mkEnvEntry = name: set: fn:
                    let
                      entrySuffix = final.toEnvInfix name;

                      entryKey = "CARGO${targetInfix}_${entrySuffix}";
                      entryValue = (fn set.${name});
                    in
                    lib.optionalAttrs (builtins.hasAttr name set) { "${entryKey}" = entryValue; }
                  ;
                in
                mkEnvEntry "linker" config asIs
                  // mkEnvEntry "runner" config asIs
                  // mkEnvEntry "rustflags" config final.mkEnvFromFlags;

              targetConfig = config.target.${target};
              targetInfix = "_" + (final.toEnvInfix target);
              # Combine the entire env variables
              env = {
                # Set the cargo build target
                CARGO_BUILD_TARGET = target;
              }
              # Append target specific variables
              // (mkEnvFromTargetConfig targetConfig targetInfix)
              # Append unstable features
              // lib.optionalAttrs (builtins.hasAttr "unstable" config) (final.unstableFlagsToEnv config.unstable target)
              // lib.optionalAttrs (builtins.hasAttr "env" config) config.env
              ;
            in
            {
              inherit target config env;
            };
        };
      })
    ] ++ overlays;
  };
in
pkgs

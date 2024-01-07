{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs-cross-overlay = {
      url = "github:alekseysidorov/nixpkgs-cross-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    treefmt-nix.url = "github:numtide/treefmt-nix";
    flake-root.url = "github:srid/flake-root";
  };

  outputs = inputs@{ flake-parts, nixpkgs, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.treefmt-nix.flakeModule
        inputs.flake-root.flakeModule
      ];

      systems = nixpkgs.lib.systems.flakeExposed;

      flake = { };

      perSystem = { config, self', inputs', system, nixpkgs, pkgs, ... }: {
        # Setup nixpkgs with overlays.
        _module.args.pkgs = import inputs.nixpkgs {
          inherit system;
          overlays = [
            inputs.rust-overlay.overlays.default
            inputs.nixpkgs-cross-overlay.overlays.default
            # Setup rust toolchain
            (final: prev: {
              rustToolchain = prev.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
            })
          ];
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; let
            # Scripts used in CI
            ci-run-tests = writeShellApplication {
              name = "ci-run-tests";
              runtimeInputs = [
                rustToolchain
              ];
              text = ''
                cargo test --all-features --all-targets
                # TODO Add cargo publish test with the cargo workspaces
              '';
            };

            ci-run-lints = writeShellApplication {
              name = "ci-run-lints";
              runtimeInputs = [
                rustToolchain
              ];
              text = ''
                cargo clippy --all-features --all --all-targets
                cargo doc --all-features  --no-deps
              '';
            };

            # Run them all together
            ci-run-all = writeShellApplication {
              name = "ci-run-all";
              runtimeInputs = [
                ci-run-tests
                ci-run-lints
              ];
              text = ''
                ci-run-tests
                ci-run-lints
              '';
            };
          in
          [
            rustToolchain
            # Useful utilities
            cargo-espflash
            taplo-cli

            ci-run-tests
            ci-run-lints
            ci-run-all
          ];

          shellHook = ''
            # Setup nice bash prompt
            ${pkgs.mkBashPrompt "esp32c3"}
          '';
        };

        treefmt.config = {
          inherit (config.flake-root) projectRootFile;

          programs = {
            nixpkgs-fmt.enable = true;
            rustfmt = {
              enable = true;
              package = pkgs.rustToolchain;
            };
            beautysh.enable = true;
            deno.enable = true;
            taplo.enable = true;
          };
        };

        formatter = config.treefmt.build.wrapper;
      };
    };
}

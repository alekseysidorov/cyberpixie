# treefmt.nix
{ pkgs, ... }:
{
  # Used to find the project root
  projectRootFile = "flake.nix";

    programs.rustfmt = {
    enable = true;
    package = pkgs.rustToolchain;
  };
  programs.nixpkgs-fmt.enable = true;

  programs.beautysh.enable = true;
  programs.deno.enable = true;
  programs.taplo.enable = true;
}
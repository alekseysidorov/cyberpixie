# Shell for the platform independent code parts
{ localSystem ? builtins.currentSystem
, pkgs ? import ./nix { inherit localSystem; }
}:
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    # Dependencies for the code formatting utility
    dprint
    nixpkgs-fmt
  ];

  shellHook = "${pkgs.crossBashPrompt}";
}

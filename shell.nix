# Shell for the platform independent code parts
{ pkgs
, rustToolchain
, rustBuildHostDependencies
, dprint
}:
pkgs.mkShell {
  nativeBuildInputs = [
    rustToolchain
    rustBuildHostDependencies
    # Dependencies for the code formatting utility
    dprint
  ];

  RUSTC_WRAPPER = "sccache";

  shellHook = "${pkgs.crossBashPrompt}";
}

{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;
        libPath = with pkgs; lib.makeLibraryPath [
          libGL
          libxkbcommon
          wayland
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
        ];
      in
    {
      packages.default = craneLib.buildPackage {
        src = craneLib.cleanCargoSource ./.;

        # Add extra inputs here or any other derivation settings
        # doCheck = true;
        buildInputs = [
          pkgs.libxcb
        ];
        nativeBuildInputs = [
          pkgs.makeWrapper
        ];
        postInstall = ''
            wrapProgram "$out/bin/org-roam-nvim-ui" --prefix LD_LIBRARY_PATH : "${libPath}"
          '';
      };

      devShell = craneLib.devShell {
        packages = [
          pkgs.xorg.libxcb
        ];
        RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
        LD_LIBRARY_PATH = libPath;
      };

      # devShell = with pkgs; mkShell {
      #   buildInputs = [
      #     cargo
      #     rust-analyzer
      #     rustPackages.clippy
      #     rustc
      #     rustfmt
      #     xorg.libxcb
      #   ];
      #   RUST_SRC_PATH = rustPlatform.rustLibSrc;
      #   LD_LIBRARY_PATH = libPath;
      # };
    });
}

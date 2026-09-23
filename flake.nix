{
  description = "nyano - a simple nano replacement for quick file edits";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        nyano = pkgs.rustPlatform.buildRustPackage {
          pname = "nyano";
          version = "1.0.0";

          src = pkgs.lib.cleanSource ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ ];
          doCheck = false;

          meta = {
            description = "a simple nano replacement for quick file edits";
            homepage = "https://github.com/shareui/nyano";
            license = pkgs.lib.licenses.mit;
            mainProgram = "nyano";
          };
        };
      in
      {
        packages.default = nyano;
        packages.nyano = nyano;

        apps.default = {
          type = "app";
          program = "${nyano}/bin/nyano";
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ nyano ];
          packages = [ pkgs.cargo pkgs.rustc pkgs.rust-analyzer pkgs.clippy ];
        };
      });
}

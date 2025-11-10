{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };
    crane.url = "github:ipetkov/crane";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        lib = nixpkgs.lib;
        craneLib = crane.mkLib pkgs;

        src = lib.cleanSourceWith { src = craneLib.path ./.; };

        envVars =
          { }
          // (lib.attrsets.optionalAttrs pkgs.stdenv.isLinux {
            RUSTFLAGS = "-Clinker=clang -Clink-arg=--ld-path=${pkgs.mold}/bin/mold";
            LD_LIBRARY_PATH = "${pkgs.libiio.lib}/lib";
          });

        commonArgs = (
          {
            inherit src;
            nativeBuildInputs = with pkgs; [
              rust-bin.stable.latest.default
              cargo
              clang
              rust-analyzer
              rustc
            ];
            buildInputs = with pkgs; [ libiio.lib libiio.dev ] ++ lib.optionals stdenv.isDarwin [ libiconv ];
          }
          // envVars
        );
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        bin = craneLib.buildPackage (
          commonArgs
          // {
            inherit cargoArtifacts;
          }
          // {
            preFixup = lib.optionalString pkgs.stdenv.isLinux ''
              patchelf \
                --add-needed "${pkgs.libiio.lib}/lib/libiio.so.0" \
                $out/bin/iio_ambient_brightness
            '';
          }
        );
      in
      with pkgs;
      {
        packages = {
          default = bin;
        };

        devShells.default = mkShell (
          {
            packages = [
              rust-bin.stable.latest.default
              cargo
              cargo-watch
              rust-analyzer
              rustc
              rustfmt
              libiio.lib
              libiio.dev
              pkg-config
            ];
          }
          // envVars
        );

        formatter = nixpkgs-fmt;
      }
    );
}

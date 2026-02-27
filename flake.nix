{
  description = "KBNT Development Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        mingw = pkgs.pkgsCross.mingwW64;

        applications = with pkgs; [
          nil
          nixd

          toolchain
        ];

        libraries = with pkgs; [
          pkg-config
          mingw.stdenv.cc
          mingw.windows.pthreads
        ];
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = applications ++ libraries;

          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath libraries;
          PKG_CONFIG_PATH = pkgs.lib.makeSearchPath "lib/pkgconfig" libraries;
          CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "${mingw.stdenv.cc}/bin/x86_64-w64-mingw32-gcc";
          PKG_CONFIG_ALLOW_CROSS = "1";
        };
      }
    );
}

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
  outputs = { nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchainFor = p: p.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" "rustc-codegen-cranelift-preview" "rust-analyzer" ];
          targets = [ "x86_64-unknown-linux-gnu" ];
        });
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchainFor;
        my-crate = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./.;
          buildInputs = [
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];
        };
      in
      {
        packages.default = my-crate;
        devShells.default = craneLib.devShell {
          inputsFrom = [ my-crate ];
          packages = [ ];
            shellHook = ''
                export RUST_SRC_PATH=${pkgs.rustPlatform.rustLibSrc}
            '';
        };
      });
}

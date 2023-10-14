{
  description = "A tool that generates Anki or PDF flashcards from openstreetmap data";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
    utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, utils, rust-overlay }: (
    utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in rec {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [
            (rust-bin.fromRustupToolchainFile ./rust-toolchain)
            cargo-bootimage
            pre-commit
            qemu
          ];
        };
      }
    )
  );
}

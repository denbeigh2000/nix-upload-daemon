{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, flake-utils, naersk, nixpkgs, rust-overlay }:
    {
      overlays.default = import ./overlay.nix { inherit naersk; };
      inherit (import ./modules) nixosModules darwinModules;
    } // flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) {
          overlays = [ rust-overlay.overlays.default self.overlays.default ];
          inherit system;
        };

        # rustPkgs = with pkgs.rust-bin.stable.latest; [ default rust-analyzer rust-src ];
        rust = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in
      rec {
        packages = {
          default = pkgs.nix-upload-daemon;
          inherit (pkgs) nix-upload-daemon;
        };

        # For `nix develop`:
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [ rust ];
        };
      }
    );
}

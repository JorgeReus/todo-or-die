{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, rust-overlay, ... }:
    let
      system = "aarch64-darwin";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      rust = pkgs.rust-bin.stable."1.98.0".default.override {
        extensions = [ "rustfmt" "clippy" ];
      };
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = [ rust pkgs.tree-sitter ];
      };
    };
}

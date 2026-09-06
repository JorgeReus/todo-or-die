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
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forEachSystem = nixpkgs.lib.genAttrs systems;
      version = "0.2.1";
      assets = {
        x86_64-linux = {
          url = "https://github.com/JorgeReus/todo-or-die/releases/download/v${version}/todo-or-die-x86_64-unknown-linux-gnu.tar.gz";
          hash = "sha256-EAuBI4b9JLwCn/hpqjpQDUG8YCkw56SuIt2LsdM+3gg=";
        };
        aarch64-linux = {
          url = "https://github.com/JorgeReus/todo-or-die/releases/download/v${version}/todo-or-die-aarch64-unknown-linux-gnu.tar.gz";
          hash = "sha256-MiEuScJklVDAJzNb+GjSZwVavZ9VI2ZkeO3Ovc1PMDY=";
        };
        x86_64-darwin = {
          url = "https://github.com/JorgeReus/todo-or-die/releases/download/v${version}/todo-or-die-x86_64-apple-darwin.tar.gz";
          hash = "sha256-cmeDTTUiZXwJ/C5ase2Jyaz1IAz7r0NXToL2zB80HeY=";
        };
        aarch64-darwin = {
          url = "https://github.com/JorgeReus/todo-or-die/releases/download/v${version}/todo-or-die-aarch64-apple-darwin.tar.gz";
          hash = "sha256-cmeDTTUiZXwJ/C5ase2Jyaz1IAz7r0NXToL2zB80HeY=";
        };
      };
      mkBinary = system: let
        pkgs = import nixpkgs { inherit system; };
        asset = assets.${system};
      in pkgs.stdenvNoCC.mkDerivation {
        pname = "todo-or-die";
        inherit version;
        src = pkgs.fetchurl asset;
        dontConfigure = true;
        dontBuild = true;
        dontUnpack = true;
        installPhase = ''
          mkdir -p $out/bin
          tar -xzf $src -C $out/bin
          chmod +x $out/bin/todo-or-die
        '';
      };
      pkgs = import nixpkgs {
        system = "aarch64-darwin";
        overlays = [ rust-overlay.overlays.default ];
      };
      rust = pkgs.rust-bin.stable."1.98.0".default.override {
        extensions = [ "rustfmt" "clippy" ];
      };
    in {
      packages = forEachSystem (system: {
        default = mkBinary system;
      });
      apps = forEachSystem (system: {
        default = {
          type = "app";
          program = "${mkBinary system}/bin/todo-or-die";
        };
      });
      devShells.aarch64-darwin.default = pkgs.mkShell { packages = [ rust ]; };
    };
}

{
  description = "Holy Furries Studio Website";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.devshell.flakeModule ];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
      ];
      perSystem =
        { pkgs, ... }:
        let
          rust_toolchain = (inputs.rust-overlay.lib.mkRustBin { } pkgs).stable.latest.default.override {
            extensions = [
              "rustfmt"
              "clippy"
              "rust-src"
            ];
          };
          crane_lib = (inputs.crane.mkLib pkgs).overrideToolchain rust_toolchain;
          package = pkgs.callPackage ./nix/package.nix { inherit crane_lib; };
        in
        {
          packages.default = package;
          checks.service = package;
          devshells.default = {
            packages = with pkgs; [
              rust_toolchain
              nats-server
              natscli
              curl
              jq
              ripgrep
              nixfmt
              opentofu
              nodejs
              biome
            ];
          };
          formatter = pkgs.nixfmt;
        };
      flake.nixosModules = {
        default = import ./nix/service.nix inputs.self;
        atlas = import ./nix/atlas.nix inputs.self;
      };
    };
}

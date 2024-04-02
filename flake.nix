{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.nci.url = "github:yusdacra/nix-cargo-integration";
  inputs.nci.inputs.nixpkgs.follows = "nixpkgs";
  inputs.parts.url = "github:hercules-ci/flake-parts";
  inputs.parts.inputs.nixpkgs-lib.follows = "nixpkgs";
  inputs.nix-input.url = "github:nixos/nix";

  outputs = inputs @ {
    parts,
    nci,
    nix-input,
    ...
  }:
    parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];
      imports = [
        nci.flakeModule
        ./crates.nix
      ];
      perSystem = {
        pkgs,
        config,
        ...
      }: let
        crateOutputs = config.nci.outputs."nix-inspect";
      in {
        devShells.default = crateOutputs.devShell.overrideAttrs (old: {
          packages =
            (old.packages or [])
            ++ (with pkgs; [
              rust-analyzer
              clang-tools
              pkg-config
              ninja
              boost
              meson
              nlohmann_json
              nix-input.packages.${system}.default
            ]);
        });
        packages.default = crateOutputs.packages.release;
      };
    };
}

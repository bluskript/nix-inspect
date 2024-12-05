{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  inputs.nci.url = "github:yusdacra/nix-cargo-integration";
  inputs.nci.inputs.nixpkgs.follows = "nixpkgs";
  inputs.parts.url = "github:hercules-ci/flake-parts";
  inputs.parts.inputs.nixpkgs-lib.follows = "nixpkgs";

  outputs = inputs @ {
    parts,
    nci,
    ...
  }:
    parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "aarch64-darwin" "aarch64-linux" "i686-linux" "x86_64-darwin"];
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
          WORKER_BINARY_PATH = "./worker/build/nix-inspect";
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
              nixVersions.nix_2_24.dev
            ]);
        });
        packages.default = crateOutputs.packages.release;
      };
    };
}

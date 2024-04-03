{inputs, ...}: {
  perSystem = {
    pkgs,
    config,
    ...
  }: let
    crateName = "nix-inspect";
    workerPackage = pkgs.stdenv.mkDerivation {
      name = "worker";
      src = ./worker;

      nativeBuildInputs = with pkgs; [
        meson
        ninja
        pkg-config
      ];

      buildInputs = with pkgs; [
        boost
        nlohmann_json
        inputs.nix-input.packages.${system}.default
      ];

      configurePhase = "meson setup build";
      buildPhase = "ninja -C build";

      installPhase = ''
        mkdir -p $out/bin
        cp build/nix-inspect $out/bin/
      '';
    };
  in {
    # declare projects
    nci.projects."nix-inspect".path = ./.;
    # configure crates
    nci.crates.${crateName} = {
      drvConfig = {
        env.WORKER_BINARY_PATH = "${workerPackage}/bin/nix-inspect";
      };
    };
  };
}

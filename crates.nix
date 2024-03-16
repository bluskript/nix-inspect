{...}: {
  perSystem = {
    pkgs,
    config,
    ...
  }: let
    # TODO: change this to your crate's name
    crateName = "nix-inspect";
  in {
    # declare projects
    nci.projects."simple".path = ./.;
    # configure crates
    nci.crates.${crateName} = {};
  };
}

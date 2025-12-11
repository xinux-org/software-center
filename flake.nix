{
  description = "A beginning of an awesome project bootstrapped with github:bleur-org/templates";

  inputs = {
    # Stable for keeping thins clean
    # # Fresh and new for testing
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    crane.url = "github:ipetkov/crane";
    
    utils.url = "github:numtide/flake-utils";
    nixos-appstream-data = {
      url = "github:korfuri/nixos-appstream-data/flake";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
    # The flake-utils library
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
    ...
  } @ inputs:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {inherit system;};
    in {
      # Nix script formatter
      formatter = pkgs.alejandra;

      # Development environment
      devShells.default = import ./shell.nix {inherit pkgs inputs;};

      # Output package
      packages.default = pkgs.callPackage ./. {inherit crane pkgs inputs;};
    });
  # // {
  #   # Hydra CI jobs
  #   hydraJobs = {
  #     packages = self.packages.x86_64-linux.default;
  #   };
  # };
}

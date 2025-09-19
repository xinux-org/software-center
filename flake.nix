{
  inputs = {
    nixpkgs.url = "github:xinux-org/nixpkgs/nixos-25.05";
    utils.url = "github:numtide/flake-utils";
    nixos-appstream-data = {
      url = "github:korfuri/nixos-appstream-data/flake";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "utils";
    };
    xinux-lib = {
      url = "github:xinux-org/lib";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs = inputs:
    inputs.xinux-lib.mkFlake {
      inherit inputs;
      alias.packages.default = "nix-software-center";
      alias.shells.default = "nix-software-center";
      src = ./.;
      # this should be implemented because it's left over previous flake.nix setup
      # hydraJobs = self.packages.${system}; 
  };
}
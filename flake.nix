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
      alias.packages.default = "nixos-conf-editor";
      alias.shells.default = "nixos-conf-editor";
      src = ./.;
  };
}




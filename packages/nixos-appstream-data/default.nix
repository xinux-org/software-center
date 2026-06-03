{ inputs, pkgs, ... }:
inputs.nixos-appstream-data.packages."${pkgs.stdenv.hostPlatform.system}".nixos-appstream-data

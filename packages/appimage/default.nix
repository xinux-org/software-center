# example: https://github.com/bpftrace/bpftrace/blob/336bb4a2042767942f1b36368270c7b68a126e58/flake.nix#L267
# source: https://github.com/ralismark/nix-appimage/blob/main/mkAppImage.nixs
{
  pkgs,
  inputs,
  ...
}:
let
  pkg = pkgs.callPackage ../nix-software-center { };
in
inputs.nix-appimage.bundlers.${pkgs.stdenv.hostPlatform.system}.default pkg

{inputs, ...}:

final: prev: {
  nixos-appstream-data = inputs.nixos-appstream-data.packages."${prev.stdenv.hostPlatform.system}".nixos-appstream-data;
}

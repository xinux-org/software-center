{
  pkgs,
  inputs,
  stdenv,
  lib,
  cargo,
  desktop-file-utils,
  rustc,
  gdk-pixbuf,
  gtk4,
  gtksourceview5,
  libadwaita,
  meson,
  ninja,
  openssl,
  pkg-config,
  polkit,
  wrapGAppsHook4,
  rustPlatform,
}:
let
  nixos-appstream-data =
    inputs.self.packages."${pkgs.stdenv.hostPlatform.system}".nixos-appstream-data;
in
stdenv.mkDerivation {
  pname = "nix-software-center";
  version = "0.1.4";

  src = [ ../.. ];

  cargoDeps = rustPlatform.fetchCargoVendor {
    src = ../..;
    hash = "sha256-07CROBXMPoOMSZ3362m24erwA1d/Ndb4d6v7CfSS+Qs=";
  };

  nativeBuildInputs =
    with pkgs;
    [
      appstream-glib
      polkit
      gettext
      desktop-file-utils
      meson
      ninja
      pkg-config
      git
      wrapGAppsHook4
    ]
    ++ (with pkgs.rustPlatform; [
      cargoSetupHook
      cargo
      rustc
    ]);

  buildInputs = with pkgs; [
    gdk-pixbuf
    glib
    gtk4
    gtksourceview5
    libadwaita
    libxml2
    openssl
    wayland
    adwaita-icon-theme
    desktop-file-utils
    nixos-appstream-data
  ];

  patchPhase = ''
    substituteInPlace ./src/lib.rs \
        --replace "/usr/share/app-info" "${nixos-appstream-data}/share/app-info"
  '';

  postInstall = ''
    mkdir -p $out/share/app-info/
    cp	-r ${nixos-appstream-data}/share/app-info/* $out/share/app-info/
    
    wrapProgram $out/bin/nix-software-center --prefix PATH : '${
      lib.makeBinPath [
        pkgs.gnome-console
        pkgs.gtk3 # provides gtk-launch
        pkgs.sqlite
      ]
    }'
  '';
}

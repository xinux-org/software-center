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
  version = "0.2.0";

  src = [ ../.. ];

  cargoDeps = rustPlatform.fetchCargoVendor {
    src = ../..;
    hash = "sha256-gjcBsl4N4CCsiSxtZBu8nrcTHhyJPepBF4lJytS4AtY=";
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
      libxdg_basedir
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
        --replace "./result/share/app-info" "${nixos-appstream-data}/share/app-info"
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

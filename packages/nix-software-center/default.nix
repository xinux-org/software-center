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
  pandoc,
  pkg-config,
  polkit,
  wrapGAppsHook4,
  system,
  rustPlatform,
}: let
  nixos-appstream-data = inputs.nixos-appstream-data.packages."${system}".nixos-appstream-data;
in
  stdenv.mkDerivation rec {
    pname = "nix-software-center";
    version = "0.1";

    src = [../..];

    cargoDeps = rustPlatform.importCargoLock {
      lockFile = ../../Cargo.lock;
      outputHashes = {
        "nix-data-0.0.3" = "sha256-7JUMDnFMQUWr7XM2ZWhbXBnFZNAmnc49JLzXURSv15o=";
      };
    };

    nativeBuildInputs = with pkgs;
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
      wrapProgram $out/bin/nix-software-center --prefix PATH : '${lib.makeBinPath [
        pkgs.gnome-console
        pkgs.gtk3 # provides gtk-launch
        pkgs.sqlite
      ]}'
    '';
  }

{
  pkgs,
  inputs,
  ...
}: let
  nixos-appstream-data = inputs.nixos-appstream-data.packages."${pkgs.stdenv.hostPlatform.system}".nixos-appstream-data;
in
  pkgs.mkShell {
    nativeBuildInputs = with pkgs; [
      nixd
      cargo
      clippy
      rust-analyzer
      rustc
      rustup
      rustfmt
      rustPlatform.bindgenHook
    ];
    buildInputs = with pkgs;
      [
        desktop-file-utils
        cairo
        gdk-pixbuf
        gobject-introspection
        graphene
        gtk4
        gtksourceview5
        libadwaita
        libxml2
        meson
        ninja
        openssl
        pandoc
        pango
        pkg-config
        polkit
        sqlite
        wrapGAppsHook4
      ]
      ++ [nixos-appstream-data];
    # Set Environment Variables
    RUST_BACKTRACE = "full";
    RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  }

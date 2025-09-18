{
  pkgs,
  inputs,
  mkShell,
  cairo,
  cargo,
  clippy,
  desktop-file-utils,
  gdk-pixbuf,
  gettext,
  gobject-introspection,
  graphene,
  gtk4,
  gtksourceview5,
  libadwaita,
  meson,
  ninja,
  openssl,
  pandoc,
  pango,
  pkg-config,
  polkit,
  sqlite,
  rust,
  rust-analyzer,
  rustc,
  rustfmt,
  wrapGAppsHook4,
  libxml2,
  system,
  rustPlatform,
}: let
  nixos-appstream-data = inputs.nixos-appstream-data.packages."${system}".nixos-appstream-data;
in {
  mkShell = {
    cargoDeps = rustPlatform.importCargoLock {
      lockFile = ../../Cargo.lock;
      outputHashes = {
        "nix-data-0.0.3" = "sha256-7JUMDnFMQUWr7XM2ZWhbXBnFZNAmnc49JLzXURSv15o=";
      };
    };
    buildInputs = with pkgs; [
      cargo
      clippy
      desktop-file-utils
      rust-analyzer
      rustc
      rustfmt
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
      nixos-appstream-data.packages."${system}".nixos-appstream-data
    ];
    # Set Environment Variables
    RUST_BACKTRACE = "full";
    RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  };
}

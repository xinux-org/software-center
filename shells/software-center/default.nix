{  mkShell
  ,cairo
  ,cargo
  ,clippy
  ,desktop-file-utils
  ,gdk-pixbuf
  ,gettext
  ,gobject-introspection
  ,graphene
  ,gtk4
  ,gtksourceview5
  ,libadwaita
  ,meson
  ,ninja
  ,openssl
  ,pandoc
  ,pango
  ,pkg-config
  ,polkit
  ,sqlite
  ,rust
  ,rust-analyzer
  ,rustc
  ,rustfmt
  ,wrapGAppsHook4
  ,libxml2
  ,inputs
  ,system
  ,rustPlatform
}:

let
  appstr = inputs.nixos-appstream-data.packages."${system}".nixos-appstream-data;
in {
  mkShell = {
    buildInputs = [
      appstr
      cairo
      cargo
      clippy
      desktop-file-utils
      gdk-pixbuf
      gettext
      gobject-introspection
      graphene
      gtk4
      gtksourceview5
      libadwaita
      meson
      ninja
      openssl
      pandoc
      pango
      pkg-config
      polkit
      sqlite
      rust
      rust-analyzer
      rustc
      rustfmt
      wrapGAppsHook4
      libxml2
    ];
    RUST_SRC_PATH = "${rust.packages.stable.rustPlatform.rustLibSrc}";

  };
  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ../../Cargo.lock;
    outputHashes = {
      "nix-data-0.0.3" = "sha256-7JUMDnFMQUWr7XM2ZWhbXBnFZNAmnc49JLzXURSv15o=";
    };
  };
}
{  stdenv
  ,lib
  ,cargo
  ,desktop-file-utils
  ,rustc
  ,gdk-pixbuf
  ,gtk4
  ,gtksourceview5
  ,libadwaita
  ,meson
  ,ninja
  ,openssl
  ,pandoc
  ,pkg-config
  ,polkit
  ,wrapGAppsHook4
  # ,nixos-appstream-data
  # ,system
  ,rustPlatform
}:
stdenv.mkDerivation rec {
  pname = "nixos-conf-editor";
  version = "0.1";

  src = [ ../.. ];

  cargoDeps = rustPlatform.importCargoLock {
    lockFile = ../../Cargo.lock;
    outputHashes = {
      "nix-data-0.0.3" = "sha256-7JUMDnFMQUWr7XM2ZWhbXBnFZNAmnc49JLzXURSv15o=";
    };
  };
  

  nativeBuildInputs = [
    # nixos-appstream-data.packages."${system}".nixos-appstream-data
    desktop-file-utils
    gdk-pixbuf
    gtk4
    gtksourceview5
    libadwaita
    meson
    ninja
    openssl
    pandoc
    pkg-config
    polkit
    wrapGAppsHook4
  ] ++ (with rustPlatform; [
    cargo
    cargoSetupHook
    rustc
  ]);

  buildInputs = [
    gdk-pixbuf
    gtk4
    gtksourceview5
    libadwaita
    openssl
  ];

  postInstall = ''
    wrapProgram $out/bin/nixos-conf-editor --prefix PATH : '${lib.makeBinPath [ pandoc ]}'
  '';
}
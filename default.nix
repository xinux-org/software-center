{
  pkgs,
  inputs,
  crane,
  ...
}: let
  # Manifest via Cargo.toml
  manifest = (pkgs.lib.importTOML ./Cargo.toml).package;

  craneLib = crane.mkLib pkgs;

  nixos-appstream-data = inputs.nixos-appstream-data.packages."${pkgs.stdenv.hostPlatform.system}".nixos-appstream-data;
  commonBuildInputs = with pkgs; [
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

  commonNativeBuildInputs = with pkgs; [
    appstream-glib
    polkit
    gettext
    desktop-file-utils
    meson
    ninja
    pkg-config
    git
    wrapGAppsHook4
  ];

  cargoArtifacts = craneLib.buildDepsOnly {
    src = craneLib.cleanCargoSource ./.;
    strictDeps = true;

    nativeBuildInputs = commonNativeBuildInputs;
    buildInputs = commonBuildInputs;
  };
in
  craneLib.buildPackage {
    pname = manifest.name;
    version = manifest.version;
    strictDeps = true;

    src = pkgs.lib.cleanSource ./.;

    inherit cargoArtifacts;

    nativeBuildInputs = commonNativeBuildInputs;
    buildInputs = commonBuildInputs;

    patchPhase = ''
      substituteInPlace ./src/lib.rs \
          --replace-fail "/usr/share/app-info" "${nixos-appstream-data}/share/app-info"
    '';
    postInstall = ''
      wrapProgram $out/bin/nix-software-center --prefix PATH : '${pkgs.lib.makeBinPath [
        pkgs.gnome-console
        pkgs.gtk3 # provides gtk-launch
        pkgs.sqlite
      ]}'
    '';

    configurePhase = ''
      mesonConfigurePhase
      runHook postConfigure
    '';

    buildPhase = ''
      runHook preBuild
      ninjaBuildPhase
      runHook postBuild
    '';

    installPhase = ''
      runHook preInstall
      mesonInstallPhase
      runHook postInstall
    '';

    doNotPostBuildInstallCargoBinaries = true;
    checkPhase = false;
  }

{
  pkgs,
  inputs,
  ...
}:
let
  nixos-appstream-data =
    inputs.nixos-appstream-data.packages."${pkgs.stdenv.hostPlatform.system}".nixos-appstream-data;

  treefmtEval = inputs.treefmt-nix.lib.evalModule pkgs {
    projectRootFile = "flake.nix";
    programs.nixfmt.enable = true;
    programs.rustfmt.enable = true;
  };

  preCommitCheck = inputs.git-hooks.lib."${pkgs.stdenv.hostPlatform.system}".run {
    src = ./.;
    hooks.treefmt.enable = true;
    hooks.treefmt.package = treefmtEval.config.build.wrapper;
  };

in
pkgs.mkShell {
  packages =
    with pkgs;
    [
      nixd
      cargo
      clippy
      rust-analyzer
      rustc
      rustfmt
      rustPlatform.bindgenHook
      nixfmt
      just
      just-lsp
      appstream
      desktop-file-utils
      cairo
      gdk-pixbuf
      gobject-introspection
      graphene
      gtk4
      gtksourceview5
      libadwaita
      librsvg
      libxml2
      meson
      ninja
      openssl
      pandoc
      pango
      pkg-config
      sqlite
      wrapGAppsHook4
    ]
    ++ [ nixos-appstream-data ];

  shellHook = preCommitCheck.shellHook;

  # Set Environment Variables
  RUST_BACKTRACE = "full";
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";
  PKG_CONFIG_PATH = "${pkgs.polkit.dev}/lib/pkgconfig";
}

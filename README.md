<div align="center">

<img src="data/icons/uz.xinux.NixSoftwareCenter.svg"/>

# Nix Software Center

[![Built with Nix][builtwithnix badge]][builtwithnix]
[![License: GPLv3][GPLv3 badge]][GPLv3]
[![Chat on Matrix][matrix badge]][matrix]

A graphical app store for Nix built with [libadwaita](https://gitlab.gnome.org/GNOME/libadwaita), [GTK4](https://www.gtk.org/), and [Relm4](https://relm4.org/). Heavily inspired by [GNOME Software](https://gitlab.gnome.org/GNOME/gnome-software).

<img src="data/screenshots/overview-light.png#gh-light-mode-only"/>
<img src="data/screenshots/overview-dark.png#gh-dark-mode-only"/>

</div>

# Features

- Install packages to `configuration.nix`
  - Flakes support can be enabled in the preferences menu
- Install packages with `nix profile`
- Show updates for all installed packages
- Search for packages
- Launch applications without installing via `nix shell` and `nix run`

## NixOS Flakes Installation

`flake.nix`

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nix-software-center.url = "git+https://git.oss.uzinfocom.uz/xinux/software-center";
  };

  outputs = inputs@{ self, nixpkgs, nix-software-center }: {
    nixosConfigurations = {
      workstation = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [ ./configuration.nix ];
        specialArgs = { inherit inputs; };
      };
    };
  };
}
```

`configuration.nix`

```nix
{ inputs, config, lib, pkgs, ... }: # add inputs here
{
...
  environment.systemPackages = with pkgs; [
    inputs.nix-software-center.packages.${stdenv.hostPlatform.system}.default
    # rest of your packages
  ];
...
}
```

## NixOS Installation

Head of `configuration.nix`

if you are on unstable channel or any version after 22.11:

```nix
{ config, pkgs, lib, ... }:
let
  nix-software-center = fetchFromForgejo {
      domain = "git.oss.uzinfocom.uz";
      owner = "xinux";
      repo = "software-center";
      tag = finalAttrs.version;
      hash = ""; # add shaa
    };
in

...

environment.systemPackages =
with pkgs; [
  nix-software-center
  # rest of your packages
];
```

For any other method of installation, when rebuilding you might be prompted to authenticate twice in a row by `pkexec`

## Single run on an flakes enabled system:

```bash
nix run git+https://git.oss.uzinfocom.uz/xinux/software-center
```

## Build & run

This application has Linux-only dependencies.

```bash
# download dependencies
nix develop

just install

just run

# or with nix when ready for release
nix build . --show-trace
./settings/result/bin/settings

# Optional. Generate translation words from /po/POTFILES.in if needed.
cd ./po
xgettext --directory=.. --files-from=POTFILES.in --from-code=UTF-8 -kgettext -o translations.pot
```

## Screenshots
<!--
<p align="middle">
  <img src="data/screenshots/frontpage-light.png#gh-light-mode-only"/>
  <img src="data/screenshots/frontpage-dark.png#gh-dark-mode-only"/>
</p>

<p align="middle">
  <img src="data/screenshots/application-light.png#gh-light-mode-only"/>
  <img src="data/screenshots/application-dark.png#gh-dark-mode-only"/>
</p>

<p align="middle">
  <img src="data/screenshots/searchpage-light.png#gh-light-mode-only"/>
  <img src="data/screenshots/searchpage-dark.png#gh-dark-mode-only"/>
</p>-->

## Licenses

Some icons in [data/icons](data/icons/) contains assets from the [NixOS logo](https://github.com/NixOS/nixos-artwork/tree/master/logo) and are licensed under a [CC-BY license](https://creativecommons.org/licenses/by/4.0/).

Some icons in [data/icons](data/icons/) contains assets from [GNOME Software](https://gitlab.gnome.org/GNOME/gnome-software/-/tree/main/data/icons/hicolor/scalable) and are licensed under [CC0-1.0](https://creativecommons.org/publicdomain/zero/1.0/).

[builtwithnix badge]: https://img.shields.io/badge/Built%20With-Nix-41439A?style=for-the-badge&logo=nixos&logoColor=white
[builtwithnix]: https://builtwithnix.org/
[GPLv3 badge]: https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge
[GPLv3]: https://opensource.org/licenses/GPL-3.0
[matrix badge]: https://img.shields.io/badge/matrix-join%20chat-0cbc8c?style=for-the-badge&logo=matrix&logoColor=white
[matrix]: https://matrix.to/#/#xinux-distro:uchar.uz

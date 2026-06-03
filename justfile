# https://github.com/snowfallorg/nix-software-center/blob/next/justfile
builddir := "builddir"
profile := "development"
bin := "nix-software-center"
local := "~/.local"

meson_flags := "-Dprofile=" + profile + " -Dprefix=" + local

# Configure meson build directory
setup:
    @if [ ! -f {{builddir}}/build.ninja ]; then \
        meson setup {{builddir}} {{meson_flags}}; \
    elif ! meson configure {{builddir}} | grep -q "profile.*{{profile}}"; then \
        meson setup {{builddir}} --reconfigure {{meson_flags}}; \
    fi

# Reconfigure existing build directory
reconfigure:
    meson setup {{builddir}} --reconfigure {{meson_flags}}

# Build the project
build: setup
    meson compile -C {{builddir}}

# Install to local prefix
install: build
    meson install -C {{builddir}}

# Build, install, and run the app
run: install
    RUST_LOG={{bin}}=DEBUG \
    ~/.local/bin/{{bin}}

# Clean build directory
clean:
    rm -rf {{builddir}} ~/.local/bin/{{bin}}

# Watch for changes and rebuild
watch:
    bacon

# Run clippy lints
lint:
    cargo clippy --manifest-path Cargo.toml

# Format code
fmt:
    cargo fmt --manifest-path Cargo.toml

# Fix lints and format code
fix:
    cargo clippy --manifest-path Cargo.toml --fix --allow-dirty --allow-staged
    cargo fmt --manifest-path Cargo.toml

# Clean and reconfigure from scratch
rebuild: clean setup build

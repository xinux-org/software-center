global_conf = configuration_data()
# cargo_options = [ '--manifest-path', meson.project_source_root() / 'e-helper' /  'Cargo.toml' ]
# cargo_options = [ '-r', meson.project_source_root() / 'e-helper' /  'Cargo.toml' ]
# cargo_options += [ '--target-dir', meson.project_build_root() / 'e-helper' /  'src' ]

if get_option('profile') == 'default'
  cargo_options += [ '--release' ]
  rust_target = 'release'
  message('Building in release mode')
else
  rust_target = 'debug'
  message('Building in debug mode')
endif

#cargo_env = [ 'CARGO_HOME=' + '/build/source' / 'nsc-helper' / 'cargo-home' ]
cargo_env = [ 'CARGO_HOME=/build/source/.cargo-home' ]

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: 'nsc-helper',
  console: true,
  install: true,
  install_dir: get_option('libexecdir'),
  depends: resources,
  # env: cargo_env,
  command: [
    # 'env',
    # cargo_env,
    cargo, 'build',
    ['--release', '--manifest-path', meson.project_source_root() / 'Cargo.toml'],
    '&&',
    'cp', '/build/source/target/' / rust_target / 'nsc-helper', '@OUTPUT@',
  ]
)

global_conf = configuration_data()
#cargo_options = ['--manifest-path', meson.project_source_root() / 'nsc-helper' / 'Cargo.toml']

if get_option('profile') == 'default'
  cargo_options = ['--release']
  rust_target = 'release'
  message('Building in release mode')
else
  rust_target = 'debug'
  message('Building in debug mode')
endif

cargo_build = custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: 'nsc-helper',
  console: true,
  install: true,
  install_dir: get_option('libexecdir'),
  depends: resources,
  command: [
    cargo,
    'build',
    cargo_options,
    '&&',
    'cp',
    '/build/source/target/' / rust_target / 'nsc-helper',
    '@OUTPUT@',
  ],
)

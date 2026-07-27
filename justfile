# Name of the application's project.
name := 'sticky-notes'

# Name of the application's applet binary.
applet := 'notes-applet'

# Name of the application's service binary.
service := 'notes-service'

# The unique ID of the application.
appid := 'dev.aae.sticky_notes'

# Path to root file system, which defaults to `/`.
rootdir := ''
# The prefix for the `/usr` directory.
prefix := '/usr'
# The location of the cargo target directory.
cargo-target-dir := env('CARGO_TARGET_DIR', 'target')

# Application's appstream metadata
appdata := appid + '.metainfo.xml'
# Application's desktop entry
desktop := appid + '.desktop'
# Application's service desktop entry to launch standalone
desktop-svc := appid + '_service' + '.desktop'
# Application's icon.
icon-svg := appid + '.svg'

# Install destinations
base-dir := absolute_path(clean(rootdir / prefix))
appdata-dst := base-dir / 'share' / 'appdata' / appdata
bin-dst-app := base-dir / 'bin' / applet
bin-dst-svc := base-dir / 'bin' / service
desktop-dst := base-dir / 'share' / 'applications' / desktop
desktop-svc-dst := base-dir / 'share' / 'applications' / desktop-svc
icons-dst := base-dir / 'share' / 'icons' / 'hicolor'
icon-svg-dst := icons-dst / 'scalable' / 'apps' / icon-svg

# Logging configuration
log-tracing := 'warn,sticky_notes=trace'

# Default recipe ('just'): do the same as 'just run' does
default: (run)

# Runs `cargo clean`
clean:
    cargo clean

# Removes vendored dependencies
#clean-vendor:
#    rm -rf .cargo vendor vendor.tar

# `cargo clean` and removes vendored dependencies
#clean-dist: clean clean-vendor

# Compiles release profile with vendored dependencies
#build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

# Run a code formatting then run a clippy check
check:
    cargo fmt
    cargo clippy --tests --all-features --all-targets -- -W clippy::pedantic

# -----------------------------------------
# Build release version
# -----------------------------------------

# Build release version of the main application targeting Cosmic DE
build *args:
    cargo build --release --bin {{service}} {{args}}

# Build release version of both main application and applet targeting Cosmic DE
build-cosmic *args: (build '--bin' applet args)

# Build release version of the main application targeting Wayland-based environment
build-wayland *args: (build '--no-default-features' '--features=wayland' args)

# Build release version of the main application targeting X11-based environment
build-x *args: (build '--no-default-features' '--features=x11' args)

# -----------------------------------------
# Run release version
# -----------------------------------------

# Build and run release version of main application targeting Cosmic DE
run *args:
    env RUST_BACKTRACE=full RUST_LOG={{log-tracing}} cargo run --release --bin {{service}} {{args}}

# Build and run release version of applet targeting Cosmic DE
run-applet *args:
    env RUST_BACKTRACE=full RUST_LOG={{log-tracing}} cargo run --release --bin {{applet}} {{args}}

# Do the same as 'just run' does
run-cosmic *args: (run args)

# Build and run release version of main application targeting Wayland-based environment
run-wayland *args: (run '--no-default-features' '--features=wayland' args)

# Build and run release version of main application targeting X11-based environment
run-x *args: (run '--no-default-features' '--features=x11' args)

# -----------------------------------------
# Install release version
# -----------------------------------------

# (sudo is required) Install sticky-notes main application
install:
    install -Dm0755 {{ cargo-target-dir / 'release' / service }} {{bin-dst-svc}}
    install -Dm0644 {{ 'resources' / desktop }} {{desktop-dst}}
    install -Dm0644 {{ 'resources' / desktop-svc }} {{desktop-svc-dst}}
    install -Dm0644 {{ 'resources' / appdata }} {{appdata-dst}}
    install -Dm0644 {{ 'resources' / 'icons' / 'hicolor' / 'scalable' / 'apps' / icon-svg }} {{icon-svg-dst}}

# (sudo is required) Install sticky-notes applet
install-applet:
    install -Dm0755 {{ cargo-target-dir / 'release' / applet }} {{bin-dst-app}}

# (sudo is required) Uninstall previously installed files
uninstall:
    rm {{bin-dst-app}} {{bin-dst-svc}} {{desktop-dst}} {{appdata-dst}} {{icon-svg-dst}}

# -----------------------------------------
# Build and run debug version
# -----------------------------------------

# Build and run the main application for debugging
dbg *args:
    env RUST_BACKTRACE=full RUST_LOG={{log-tracing}} cargo run --bin {{service}} {{args}}

# Build and run the applet for debugging
dbg-applet *args:
    cargo build --bin {{service}} --bin {{applet}} {{args}}
    env RUST_BACKTRACE=full RUST_LOG={{log-tracing}} cargo run --bin {{applet}} {{args}}


# # Vendor dependencies locally
# vendor:
#     mkdir -p .cargo
#     cargo vendor | head -n -1 > .cargo/config.toml
#     echo 'directory = "vendor"' >> .cargo/config.toml
#     tar pcf vendor.tar vendor
#     rm -rf vendor

# # Extracts vendored dependencies
# vendor-extract:
#     rm -rf vendor
#     tar pxf vendor.tar

# # Bump cargo version, create git commit, and create tag
# tag version:
#     find -type f -name Cargo.toml -exec sed -i '0,/^version/s/^version.*/version = "{{version}}"/' '{}' \; -exec git add '{}' \;
#     cargo check
#     cargo clean
#     git add Cargo.lock
#     git commit -m 'release: {{version}}'
#     git commit --amend
#     git tag -a {{version}} -m ''

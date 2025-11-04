set -e

export PKG_CONFIG_PATH=/data/h445xu/opt/ssl/lib64/pkgconfig

cargo build --workspace
cargo test --workspace
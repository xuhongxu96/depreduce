set -e

cargo build
cargo build --features dev
cargo build --tests
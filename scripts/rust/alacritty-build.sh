set -e

cargo build
cargo build --tests
cargo test
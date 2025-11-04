set -e

cargo build --exclude affine_mobile_native --workspace
cargo build --exclude affine_mobile_native --exclude affine_nbstore --workspace --tests
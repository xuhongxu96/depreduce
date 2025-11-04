export RUST_BACKTRACE=1
./target/release/depreduce \
    --build-system cargo \
    -w /data/h445xu/repo/AFFiNE \
    -c $PWD/scripts/rust/affine-build.sh \
    --config $PWD/scripts/rust/affine.toml \
    --output affine-output > affine.stdout 2>affine.stderr
# https://github.com/toeverything/AFFiNE/pull/13854
export RUST_BACKTRACE=1
./target/release/depreduce \
    --build-system cargo \
    -w /data/h445xu/repo/uv \
    -c $PWD/scripts/rust/uv-build.sh \
    --config $PWD/scripts/rust/uv.toml \
    --output uv-output > uv.stdout 2>uv.stderr
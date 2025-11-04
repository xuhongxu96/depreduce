export RUST_BACKTRACE=1
./target/release/depreduce \
    --build-system cargo \
    -w /data/h445xu/repo/alacritty \
    -c $PWD/scripts/rust/alacritty-build.sh \
    --config $PWD/scripts/rust/alacritty.toml \
    --output alacritty-output > alacritty.stdout 2>alacritty.stderr
# nothing reduced
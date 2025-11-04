export RUST_BACKTRACE=1
./target/release/depreduce \
    --build-system cargo \
    -w /data/h445xu/repo/typst \
    -c $PWD/scripts/rust/typst-build.sh \
    --config $PWD/scripts/rust/typst.toml \
    --output typst-output > typst.stdout 2>typst.stderr
# nothing reduced
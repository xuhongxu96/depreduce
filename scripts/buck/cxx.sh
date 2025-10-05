export RUST_BACKTRACE=1
./target/release/depreduce \
    --build-system buck \
    -w /data/h445xu/repo/cxx \
    -c $PWD/scripts/buck/cxx-build.sh \
    --config $PWD/scripts/buck/cxx.toml \
    --output cxx-output > cxx.stdout 2>cxx.stderr
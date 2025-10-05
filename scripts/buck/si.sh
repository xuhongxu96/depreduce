export RUST_BACKTRACE=1
./target/release/depreduce \
    --build-system buck \
    -w /data/h445xu/repo/si \
    -c $PWD/scripts/buck/si-build.sh \
    --config $PWD/scripts/buck/si.toml \
    --output si-output > si.stdout 2>si.stderr
# https://github.com/systeminit/si/pull/7436
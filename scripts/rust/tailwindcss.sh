export RUST_BACKTRACE=1
./target/release/depreduce \
    --build-system cargo \
    -w /data/h445xu/repo/tailwindcss \
    -c $PWD/scripts/rust/tailwindcss-build.sh \
    --config $PWD/scripts/rust/tailwindcss.toml \
    --output tailwindcss-output > tailwindcss.stdout 2>tailwindcss.stderr
# https://github.com/tailwindlabs/tailwindcss/pull/19256
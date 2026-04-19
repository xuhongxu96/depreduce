export RUST_BACKTRACE=1
export USE_BAZEL_VERSION=7.4.1
./target/release/depreduce \
    -w /data/h445xu/repo/perses \
    -c $PWD/scripts/experiments/perses-build.sh \
    --config $PWD/scripts/experiments/perses.toml \
    --output perses-output > perses.stdout 2>perses.stderr
# https://github.com/uw-pluverse/perses/pull/42
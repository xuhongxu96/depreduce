export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/google__copybara \
    -c $PWD/scripts/experiments/copybara-build.sh \
    --config $PWD/scripts/experiments/copybara.toml \
    --output copybara-output > copybara.stdout 2>copybara.stderr
# https://github.com/google/copybara/pull/329
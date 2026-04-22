export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/buildbuddy-io__buildbuddy \
    -c $PWD/scripts/experiments/buildbuddy-build.sh \
    --config $PWD/scripts/experiments/buildbuddy.toml \
    --output buildbuddy-output > buildbuddy.stdout 2>buildbuddy.stderr
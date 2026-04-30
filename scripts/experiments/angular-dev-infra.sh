export RUST_BACKTRACE=1
export USE_BAZEL_VERSION=8.6.0
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/angular__dev-infra \
    -c $PWD/scripts/experiments/angular-dev-infra-build.sh \
    --config $PWD/scripts/experiments/angular-dev-infra.toml \
    --output angular-dev-infra-output > angular-dev-infra.stdout 2>angular-dev-infra.stderr
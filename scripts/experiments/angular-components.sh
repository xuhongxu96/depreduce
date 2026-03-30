export RUST_BACKTRACE=1
export USE_BAZEL_VERSION=8.6.0
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/angular__components \
    -c $PWD/scripts/experiments/angular-components-build.sh \
    --config $PWD/scripts/experiments/angular-components.toml \
    --output angular-components-output > angular-components.stdout 2>angular-components.stderr
# https://github.com/angular/components/pull/33004
export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/CodeIntelligenceTesting__jazzer \
    -c $PWD/scripts/experiments/jazzer-build.sh \
    --deps-only \
    --config $PWD/scripts/experiments/jazzer.toml \
    --output jazzer-output > jazzer.stdout 2>jazzer.stderr
# https://github.com/CodeIntelligenceTesting/jazzer/pull/949
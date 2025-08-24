export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/zml__zml \
    -c $PWD/scripts/experiments/zml-build.sh \
    --deps-only \
    --config $PWD/scripts/experiments/zml.toml \
    --output zml-output > zml.stdout 2>zml.stderr
# no removal
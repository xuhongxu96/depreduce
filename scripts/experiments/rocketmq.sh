export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/rocketmq \
    -c $PWD/scripts/experiments/rocketmq-build.sh \
    --deps-only \
    --disable-dependency-lifting \
    --config $PWD/scripts/experiments/rocketmq.toml \
    --output rocketmq-output > rocketmq.stdout 2>rocketmq.stderr
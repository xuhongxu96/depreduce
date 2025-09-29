export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/rocketmq \
    -c $PWD/scripts/experiments/rocketmq-build.sh \
    --config $PWD/scripts/experiments/rocketmq.toml \
    --output rocketmq-output > rocketmq.stdout 2>rocketmq.stderr
# https://github.com/apache/rocketmq/pull/9610
export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/typedb__typedb \
    -c $PWD/scripts/experiments/typedb-build.sh \
    --deps-only \
    --config $PWD/scripts/experiments/typedb.toml \
    --output typedb-output > typedb.stdout 2>typedb.stderr
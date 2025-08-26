export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/gerrit \
    -c $PWD/scripts/experiments/gerrit-build.sh \
    --deps-only \
    --config $PWD/scripts/experiments/gerrit.toml \
    --output gerrit-output > gerrit.stdout 2>gerrit.stderr
export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/JetBrains__hirschgarten \
    -c $PWD/scripts/experiments/hirschgarten-build.sh \
    --deps-only \
    --config $PWD/scripts/experiments/hirschgarten.toml \
    --output hirschgarten-output > hirschgarten.stdout 2>hirschgarten.stderr
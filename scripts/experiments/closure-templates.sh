export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/google__closure-templates \
    -c $PWD/scripts/experiments/closure-templates-build.sh \
    --deps-only \
    --config $PWD/scripts/experiments/closure-templates.toml \
    --output closure-templates-output > closure-templates.stdout 2>closure-templates.stderr
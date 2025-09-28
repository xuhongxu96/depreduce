export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/unicode-org__icu \
    -c $PWD/scripts/build.sh \
    --disable-dependency-lifting \
    --config $PWD/scripts/experiments/icu.toml \
    --output icu-output > icu.stdout 2>icu.stderr

# no result -- `cc_library`` generates static libraries, of which `deps` are not actually linked.
# if we do dependency reduction on `cc_library`, all `deps` except header-only deps will be removed.
export RUST_BACKTRACE=1
./target/release/depreduce \
  -w /data/h445xu/repo/bazel-repos/cloned_repos/risc0__zirgen \
  -c $PWD/scripts/experiments/zirgen-build.sh \
  --config $PWD/scripts/experiments/zirgen.toml \
  --output zirgen-output > zirgen.stdout 2>zirgen.stderr
# https://github.com/risc0/zirgen/pull/325
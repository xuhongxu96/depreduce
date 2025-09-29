export RUST_BACKTRACE=1
./target/release/depreduce \
  -w /data/h445xu/repo/trpc-cpp \
  -c $PWD/scripts/experiments/trpc-cpp-build.sh \
  --config $PWD/scripts/experiments/trpc-cpp.toml \
  --output trpc-cpp-output > trpc-cpp.stdout 2>trpc-cpp.stderr
# https://github.com/trpc-group/trpc-cpp/pull/220
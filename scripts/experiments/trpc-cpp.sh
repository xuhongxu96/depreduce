export RUST_BACKTRACE=1
./target/release/depreduce -w /data/h445xu/repo/trpc-cpp -c $PWD/scripts/experiments/trpc-cpp-build.sh --deps-only --output trpc-cpp-output > trpc-cpp.stdout 2>trpc-cpp.stderr
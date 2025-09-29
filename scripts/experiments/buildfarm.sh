export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/buildfarm__buildfarm \
    -c $PWD/scripts/experiments/buildfarm-build.sh \
    --config $PWD/scripts/experiments/buildfarm.toml \
    --output buildfarm-output > buildfarm.stdout 2>buildfarm.stderr
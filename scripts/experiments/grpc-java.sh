export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/grpc__grpc-java \
    -c $PWD/scripts/experiments/grpc-java-build.sh \
    --deps-only \
    --config $PWD/scripts/experiments/grpc-java.toml \
    --output grpc-java-output > grpc-java.stdout 2>grpc-java.stderr
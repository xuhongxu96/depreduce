export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/tensorflow/ \
    -c $PWD/scripts/experiments/tensorflow-build.sh \
    --deps-only \
    --config $PWD/scripts/experiments/tensorflow.toml \
    --output tensorflow-output > tensorflow.stdout 2>tensorflow.stderr
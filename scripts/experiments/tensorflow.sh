export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/tensorflow/ \
    -c $PWD/scripts/experiments/tensorflow-build.sh \
    --target //tensorflow/tools/pip_package:wheel \
    --disable-dependency-flattening \
    --disable-dependency-lifting \
    --config $PWD/scripts/experiments/tensorflow.toml \
    --output tensorflow-output > tensorflow.stdout 2>tensorflow.stderr
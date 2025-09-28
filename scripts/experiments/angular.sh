export RUST_BACKTRACE=1
./target/release/depreduce \
    -w /data/h445xu/repo/bazel-repos/cloned_repos/angular__angular \
    -c $PWD/scripts/experiments/angular-build.sh \
    --config $PWD/scripts/experiments/angular.toml \
    --output angular-output > angular.stdout 2>angular.stderr
# https://github.com/angular/angular/pull/63348
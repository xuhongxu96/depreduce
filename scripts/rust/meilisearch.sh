export RUST_BACKTRACE=1
./target/release/depreduce \
    --build-system cargo \
    -w /data/h445xu/repo/meilisearch \
    -c $PWD/scripts/rust/meilisearch-build.sh \
    --config $PWD/scripts/rust/meilisearch.toml \
    --output meilisearch-output > meilisearch.stdout 2>meilisearch.stderr
# https://github.com/meilisearch/meilisearch/pull/5969
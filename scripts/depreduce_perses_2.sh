nohup ./target/release/depreduce \
  -w /data/h445xu/repo/perses-2 \
  -c scripts/build_perses.sh \
  --disable-dependency-lifting \
  --output perses-2-output > perses-2-stdout.log 2>perses-2-stderr.log &
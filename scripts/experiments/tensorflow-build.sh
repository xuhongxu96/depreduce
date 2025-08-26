set -e

sh=$(tr '\n' ' ' </data/h445xu/repo/depreduce/scripts/experiments/tensorflow-docker-build.sh)

docker exec -w /tensorflow 1381a743e108 bash -c "$sh"
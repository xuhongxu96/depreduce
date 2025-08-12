set -e

# ARGS=(
#   "--define trpc_include_rpcz=true"
#   "--define trpc_include_overload_control=true"
# )

bazel build //... ${ARGS[@]}
bazel test --spawn_strategy=local --notest_keep_going //... ${ARGS[@]}
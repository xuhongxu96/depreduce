set -e

ARGS=(
  "--define trpc_include_ssl=true"
  "--define trpc_enable_profiler=true"
  "--define trpc_include_rpcz=true"
  "--define trpc_include_prometheus=true"
  "--define include_metrics_prometheus=true"
  "--define trpc_include_overload_control=true"
  "--define trpc_enable_http_transinfo_base64=true"
)

bazel build --spawn_strategy=standalone //... ${ARGS[@]}
bazel test --spawn_strategy=standalone --notest_keep_going //... ${ARGS[@]}
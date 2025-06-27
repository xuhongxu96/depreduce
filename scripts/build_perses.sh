set -e

source "scripts/constants.sh" || exit 1

bazelisk test --notest_keep_going --test_output=errors ${ALL_BAZEL_BUILD_TARGETS_STRING} \
  //benchmark:run_benchmark_test \
  //benchmark:convert_memory_log_to_csv_test \
  //benchmark:convert_time_query_to_csv_test \
  //:all || exit 1

bazelisk build ${ALL_BAZEL_BUILD_TARGETS_STRING} || exit 1
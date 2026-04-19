set -e

source "scripts/constants.sh" || exit 1

bazelisk test --notest_keep_going --test_output=errors ${ALL_BAZEL_BUILD_TARGETS_STRING} \
  //benchmark:run_benchmark_test \
  //benchmark:convert_memory_log_to_csv_test \
  //benchmark:convert_time_query_to_csv_test \
  //:all \
  -- \
  || exit 1

  # -//test/org/perses/util/markdown:MarkdownToHTMLConverterTest \
  # -//test/org/perses/reduction:profile_query_cache_memory_usage \
  # -//test/org/perses/reduction:test_query_cache_memory_usage_is_not_empty \
  # -//test/org/perses/benchmark_toys/scala_print:golden_test_reduce_scala_print_with_token_slicer_progress_test \

bazelisk build ${ALL_BAZEL_BUILD_TARGETS_STRING} || exit 1

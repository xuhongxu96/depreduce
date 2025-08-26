set -e

bazel build //...
bazel test --notest_keep_going -- //... -//tests/... -//storage/tests:test_recovery
# bazel test //tests/behaviour/connection:test_connection --test_output=streamed  --test_arg="--test-threads=1"
# bazel test //tests/behaviour/concept:test_concept --test_output=errors --test_arg="--test-threads=1"
# bazel test //tests/behaviour/query:test_query --test_output=streamed --test_arg="--test-threads=1" --test_arg="test_read"
# bazel test //tests/behaviour/query:test_query --test_output=streamed --test_arg="--test-threads=1" --test_arg="test_write"
# bazel test //tests/behaviour/query:test_query --test_output=streamed --test_arg="--test-threads=1" --test_arg="test_definable"
# bazel test //tests/behaviour/query:test_query --test_output=streamed --test_arg="--test-threads=1" --test_arg="functions::"
# bazel test //tests/assembly:test_assembly
# bazel test //tests/behaviour/service/... --test_output=streamed --jobs=1
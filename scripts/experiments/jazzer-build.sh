set -e

bazel build //...
bazel test --notest_keep_going -- //... -//:buildifier_test
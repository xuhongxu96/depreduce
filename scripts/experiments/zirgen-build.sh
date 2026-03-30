set -e

bazel build //zirgen/dsl:zirgen
bazel test --notest_keep_going //...
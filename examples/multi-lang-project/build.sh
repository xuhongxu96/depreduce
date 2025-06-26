set -e

bazel build //...
bazel run //main/py:main
bazel run //main/java:core_java
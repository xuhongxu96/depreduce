set -e

bazel build //...
bazel test --notest_keep_going -- //... -//javatests/com/google/gerrit/acceptance/server/util:server_util
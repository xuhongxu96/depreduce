set -e
bazel build --noincompatible_disallow_empty_glob -- //...
bazel test --noincompatible_disallow_empty_glob --notest_keep_going -- //... \
    -//bazel/spec-bundling/test:test_async_await \
    -//ng-dev/utils/test:test \
    -//bazel/api-golden/test:test_npm_package_no_exports_field
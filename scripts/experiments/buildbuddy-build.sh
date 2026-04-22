set -e

bazel build //...
bazel test --notest_keep_going -- //... \
    -//enterprise/server/remote_execution/overlayfs:overlayfs_test \
    -//server/util/disk:disk_test \
    -//enterprise/server/remote_execution/filecache:filecache_test \
    -//enterprise/server/oci/ocifetcher:ocifetcher_test \
    -//enterprise/server/scheduling/scheduler_server:scheduler_server_test \
    -//server/test/webdriver/... \
    -//enterprise/server/remote_execution/copy_on_write:copy_on_write_test \
    -//server/backends/disk_cache:disk_cache_test \
    -//enterprise/server/raft/store:store_test \
    -//enterprise/server/backends/migration_cache:migration_cache_test
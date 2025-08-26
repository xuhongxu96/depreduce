set -e

bazel build //...
bazel test --spawn_strategy=local --notest_keep_going -- //... \
    -//javatests/com/google/copybara:util/DiffUtilTest_diff_util_test \
    -//javatests/com/google/copybara:util/AutoPatchUtilTest_auto_patch_util_test \
    -//javatests/com/google/copybara/regenerate:RegenerateCmdTest_all_tests \
    -//javatests/com/google/copybara:util/ConsistencyFileTest_consistency_file_tests \
    -//javatests/com/google/copybara:WorkflowTest_workflow_test \
    -//javatests/com/google/copybara/transform/patch:QuiltTransformationTest_patch_tests \
    -//javatests/com/google/copybara/git:GitRepositoryTest_all_tests \
    -//javatests/com/google/copybara/git:GitOriginTest_all_tests
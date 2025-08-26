export HERMETIC_PYTHON_VERSION=3.9 && 
bazel build //tensorflow/tools/pip_package:wheel 
--repo_env=USE_PYWRAP_RULES=1 
--repo_env=WHEEL_NAME=tensorflow_cpu 
--config=opt && 
bazel test 
--test_tag_filters=-no_oss,-oss_excluded,-oss_serial,-gpu,-tpu,-benchmark-test 
-k 
--build_tests_only 
--config=opt 
--test_output=errors 
--test_size_filters=small,medium 
-- 
//tensorflow/... 
-//tensorflow/compiler/... 
-//tensorflow/lite/...  
-//tensorflow/core/kernels/mkl:onednn_fused_matmul_ops_test 
-//tensorflow/python/debug/examples/... 
-//tensorflow/java/... 
-//tensorflow/python/tools:large_matmul_no_multithread_test 
-//tensorflow/python/tools:large_matmul_yes_multithread_test
-//tensorflow/python/distribute:moving_averages_test_2gpu
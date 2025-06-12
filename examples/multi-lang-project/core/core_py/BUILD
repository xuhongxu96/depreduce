load("@pybind11_bazel//:build_defs.bzl", "pybind_extension")
load("@rules_python//python:defs.bzl", "py_library")

pybind_extension(
    name = "core_py",
    srcs = ["core_pybind.cc"],
    deps = [
        "//core/core_cxx",
    ],
)

py_library(
    name = "core_py_lib",
    data = [":core_py"],
    imports = ["."],
    visibility = ["//visibility:public"],
)

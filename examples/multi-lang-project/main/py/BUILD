load("@rules_python//python:defs.bzl", "py_binary")

py_binary(
    name = "main",
    srcs = ["main.py"],
    deps = [
        "//core/core_cxx",
        "//core/core_py:core_py_lib",
    ],
)

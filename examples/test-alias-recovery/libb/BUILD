load("@rules_cc//cc:cc_library.bzl", "cc_library")

cc_library(
    name = "libb",
    srcs = ["b.cpp"],
    hdrs = ["b.h"],
    includes = ["."],
    visibility = ["//visibility:public"],
    deps = [
        "//liba",
    ],
)

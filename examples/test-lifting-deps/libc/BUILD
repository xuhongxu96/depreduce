load("@rules_cc//cc:cc_library.bzl", "cc_library")

cc_library(
    name = "libc",
    srcs = ["c.cpp"],
    hdrs = ["c.h"],
    includes = ["."],
    visibility = ["//visibility:public"],
    deps = [
        "//libb",
    ],
)

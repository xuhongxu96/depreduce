load("@rules_cc//cc:cc_library.bzl", "cc_library")

cc_library(
    name = "liba_win",
    srcs = ["win.cc"],
    hdrs = ["a.h"],
    includes = ["."],
    visibility = ["//visibility:public"],
)

cc_library(
    name = "liba_linux",
    srcs = ["linux.cc"],
    hdrs = ["a.h"],
    includes = ["."],
    visibility = ["//visibility:public"],
)

cc_library(
    name = "liba_mac",
    srcs = ["mac.cc"],
    hdrs = ["a.h"],
    includes = ["."],
    visibility = ["//visibility:public"],
)

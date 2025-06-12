load("@rules_cc//cc:cc_binary.bzl", "cc_binary")
load("@rules_cc//cc:cc_library.bzl", "cc_library")

cc_library(
    name = "core_jni_lib",
    srcs = ["core_jni.cc"],
    deps = [
        "//core/core_cxx",
        "//tools/jni:copy_jni_hdr_lib",
    ],
    alwayslink = True,
)

cc_binary(
    name = "libcore_jni.so",
    linkshared = True,
    visibility = ["//visibility:public"],
    deps = [
        ":core_jni_lib",
        "//core/core_cxx",
    ],
)

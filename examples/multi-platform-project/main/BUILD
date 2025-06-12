load("@rules_cc//cc:cc_binary.bzl", "cc_binary")

cc_binary(
    name = "main",
    srcs = ["main.cc"],
    deps = select({
        "@platforms//os:windows": ["//liba:liba_win"],
        "@platforms//os:macos": ["//liba:liba_mac"],
        "@platforms//os:linux": ["//liba:liba_linux"],
    }),
)

load("@rules_cc//cc:cc_binary.bzl", "cc_binary")

alias(
    name = "alias_liba",
    actual = "//liba:liba",
)

alias(
    name = "alias_libb",
    actual = "//libb:libb",
)

cc_binary(
    name = "main",
    srcs = [
        "main.cpp",
    ],
    deps = [
        ":alias_liba",
        ":alias_libb",
    ],
)

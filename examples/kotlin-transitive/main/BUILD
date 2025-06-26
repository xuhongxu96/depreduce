load("@rules_java//java:defs.bzl", "java_binary")
load("@rules_kotlin//kotlin:jvm.bzl", "kt_jvm_library")

kt_jvm_library(
    name = "main_lib",
    srcs = ["Main.kt"],
    deps = [
        "//liba",
        "//libd",
    ],
)

java_binary(
    name = "main",
    main_class = "main.Main",
    runtime_deps = [
        ":main_lib",
    ],
)

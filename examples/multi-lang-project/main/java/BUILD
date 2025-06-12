load("@rules_java//java:defs.bzl", "java_binary", "java_library")

java_library(
    name = "core_java_lib",
    srcs = ["src/com/example/core/Core.java"],
    data = ["//core/core_jni:libcore_jni.so"],
    visibility = ["//visibility:public"],
)

java_binary(
    name = "core_java",
    srcs = ["src/com/example/core/Main.java"],
    jvm_flags = ["-Djava.library.path=core/core_jni"],
    main_class = "com.example.core.Main",
    visibility = ["//visibility:public"],
    deps = [":core_java_lib"],
)

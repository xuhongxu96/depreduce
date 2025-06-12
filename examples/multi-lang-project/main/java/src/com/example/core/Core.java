package com.example.core;

public class Core {
    static {
        System.loadLibrary("core_jni");
    }

    public native int coreAdd(int a, int b);
}

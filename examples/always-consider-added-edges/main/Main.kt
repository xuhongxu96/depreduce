package main

import libb.ClassB

class Main {
    companion object {
        @JvmStatic
        fun main(args: Array<String>) {
            println(ClassB.multiply(4, 6))
        }
    }
}
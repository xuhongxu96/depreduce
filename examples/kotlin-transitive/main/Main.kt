package main

import liba.ClassA
import libc.ClassC

class Main {
    companion object {
        @JvmStatic
        fun main(args: Array<String>) {
            println(ClassA.add(2, 3))
            println(ClassC.divide(9, 3))
        }
    }
}
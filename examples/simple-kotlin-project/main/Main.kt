package main

import liba.ClassA
import libb.ClassB

class Main {
    companion object {
        @JvmStatic
        fun main(args: Array<String>) {
            println(ClassA.add(2, 3))
        }
    }
}
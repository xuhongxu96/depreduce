package main

import liba.ClassA

class Main {
    companion object {
        @JvmStatic
        fun main(args: Array<String>) {
            println(ClassA.add(2, 3))
        }
    }
}
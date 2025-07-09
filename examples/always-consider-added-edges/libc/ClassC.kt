package libc

import liba.ClassA

class ClassC {
    companion object {
        fun minus(a: Int, b: Int): Int {
            return ClassA.add(a, -b)
        }
    }
}
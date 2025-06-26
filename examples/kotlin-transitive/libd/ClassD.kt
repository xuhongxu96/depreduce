package libd

import liba.ClassA
import libb.ClassB
import libc.ClassC

class ClassC {
    companion object {
        fun add_and_multiply_and_divide(a: Int, b: Int, c: Int, d: Int): Int {
            return ClassC.divide(ClassA.add(a, b) * ClassB.multiply(a, c), d)
        }
    }
}
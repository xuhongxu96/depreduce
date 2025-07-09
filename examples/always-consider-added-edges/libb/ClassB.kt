package libb

import liba.ClassA

class ClassB {
    companion object {
        fun multiply(a: Int, b: Int): Int {
            var res = 0
            for (i in 1..b) {
                res = ClassA.add(res, a)
            }
            return res
        }
    }
}
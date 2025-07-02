#include <iostream>

#include "a.h"
#include "b.h"
#include "c.h"

int main() {
  std::cout << add(3, 4) << std::endl;
  std::cout << multiply(3, 4) << std::endl;
  std::cout << minus(3, 4) << std::endl;
  return 0;
}
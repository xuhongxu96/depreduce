#include <pybind11/pybind11.h>

#include "core.h"

PYBIND11_MODULE(core_py, module) {
  module.doc() = "Example Core Library";
  module.def("core_add", &core_add, "Core Add Function");
}
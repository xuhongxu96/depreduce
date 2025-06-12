#include "core.h"
#include "jni.h"

extern "C" {

JNIEXPORT jint JNICALL Java_com_example_core_Core_coreAdd(JNIEnv *env,
                                                          jobject obj, jint a,
                                                          jint b) {
  return core_add(a, b);
}
}
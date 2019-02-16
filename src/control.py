from cffi import FFI
from ctypes import c_bool
import sys
ffi = FFI()

ffi.cdef("""
    typedef void* BleepsBoxes;

    BleepsBoxes init();

    uint32_t newbox(BleepsBoxes, uint32_t);

    void setc(BleepsBoxes, uint32_t, uint32_t, uint32_t, const char*);
    void printc(BleepsBoxes, uint32_t, uint32_t, uint32_t);

""")

lib = ffi.dlopen("/home/pent/AsciiBox/target/debug/libasciibox.so")




screen = lib.init()
box = lib.newbox(screen, 0)
print(box)
box = lib.newbox(screen, 0)
print(box)
lib.setc(screen, 1, 3, 3, b'q')
lib.printc(screen, 1, 3, 3)


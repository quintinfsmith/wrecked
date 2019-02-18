from cffi import FFI
from ctypes import c_bool
import sys
ffi = FFI()

ffi.cdef("""
    typedef void* BleepsBoxes;

    BleepsBoxes init();

    uint32_t newbox(BleepsBoxes, uint32_t);

    void flag_recache(BleepsBoxes, uint32_t);
    void draw(BleepsBoxes);
    void setc(BleepsBoxes, uint32_t, uint32_t, uint32_t, const char*);
    void printc(BleepsBoxes, uint32_t, uint32_t, uint32_t);

""")

lib = ffi.dlopen("/home/pent/Projects/AsciiBox/target/debug/libasciibox.so")




screen = lib.init()
box = lib.newbox(screen, 0)
box = lib.newbox(screen, box)
lib.setc(screen, box, 3, 3, b'q')
#lib.printc(screen, 1, 3, 3)

lib.draw(screen)




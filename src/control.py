from cffi import FFI
from ctypes import c_bool
import sys

ffi = FFI()
ffi.cdef("""
    void testfunc();
    void init();
""")

lib = ffi.dlopen("/home/pent/Code/AsciiBox/target/debug/libasciibox.so")
lib.init()
lib.testfunc()


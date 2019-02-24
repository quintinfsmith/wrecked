from cffi import FFI
from ctypes import c_bool
import sys
import tty, termios
import os

def get_terminal_size():
    height, width = os.popen('stty size', 'r').read().split()
    return (int(width), int(height))

class BleepsScreen(object):
    SO_PATH = "/home/pent/Projects/AsciiBox/target/debug/libasciibox.so"
    def __init__(self):
        ffi = FFI()
        ffi.cdef("""
            typedef void* BleepsBoxes;

            BleepsBoxes init(uint32_t, uint32_t);

            uint32_t newbox(BleepsBoxes, uint32_t, uint32_t, uint32_t);

            void movebox(BleepsBoxes, uint32_t, uint32_t, uint32_t);
            void flag_recache(BleepsBoxes, uint32_t);
            void set_bg_color(BleepsBoxes, uint32_t, uint8_t);
            void set_fg_color(BleepsBoxes, uint32_t, uint8_t);
            void setc(BleepsBoxes, uint32_t, uint32_t, uint32_t, const char*);
            void draw(BleepsBoxes);
            void kill(BleepsBoxes);
        """)

        self.lib = ffi.dlopen(self.SO_PATH)
        w, h = get_terminal_size()
        self.box_vector = self.lib.init(w, h)

    def box_setc(self, box_id, x, y, character):
        fmt_character = bytes(character, 'utf-8')
        self.lib.setc(self.box_vector, box_id, x, y, fmt_character)
    def box_move(self, box_id, x, y):
        self.lib.movebox(self.box_vector, box_id, x, y)

    def _new_box(self, width, height, parent=0):

        new_box_id = self.lib.newbox(self.box_vector, parent, width, height)
        return BleepsBox(new_box_id, width, height, self)

    def new_box(self, width, height):
        return self._new_box(width, height)


    def draw(self):
        self.lib.draw(self.box_vector)

    def kill(self):
        self.lib.kill(self.box_vector)


class BleepsBox(object):
    def __init__(self, n, width, height, screen):
        self._screen  = screen
        self.bleeps_id = n
        self.boxes = {}
        self.width = width
        self.height = height

    def setc(self, x, y, character):
        self._screen.box_setc(self.bleeps_id, x, y, character)

    def move(self, new_x, new_y):
        self._screen.box_move(self.bleeps_id, new_x, new_y)

    def set_fg_color(self, new_col):
        self._screen.box_set_fg_color(self.bleeps_id, new_col)

    def set_bg_color(self, new_col):
        self._screen.box_set_bg_color(self.bleeps_id, new_col)

    def new_box(self, width, height):
        box = self._screen._new_box(width, height, self.bleeps_id)
        self.boxes[box.id] = box

        return box


screen = BleepsScreen()
box = screen.new_box(10, 10)
for y in range(box.height):
    box.setc(0, y, '|')
    box.setc(box.width - 1, y, '|')

for x in range(box.width):
    box.setc(x, 0, '=')
    box.setc(x, box.height - 1, '=')

import time
screen.draw()

time.sleep(2)
screen.kill()

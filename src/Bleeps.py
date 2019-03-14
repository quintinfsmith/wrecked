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
            typedef void* BleepsBoxHandler;

            BleepsBoxHandler init(uint32_t, uint32_t);

            uint32_t newbox(BleepsBoxHandler, uint32_t, uint32_t, uint32_t);

            void movebox(BleepsBoxHandler, uint32_t, int32_t, int32_t);
            void flag_recache(BleepsBoxHandler, uint32_t);

            void set_bg_color(BleepsBoxHandler, uint32_t, uint8_t);
            void set_fg_color(BleepsBoxHandler, uint32_t, uint8_t);
            void unset_color(BleepsBoxHandler, uint32_t);
            void unset_bg_color(BleepsBoxHandler, uint32_t);
            void unset_fg_color(BleepsBoxHandler, uint32_t);

            void disable_box(BleepsBoxHandler, uint32_t);
            void enable_box(BleepsBoxHandler, uint32_t);

            void setc(BleepsBoxHandler, uint32_t, uint32_t, uint32_t, const char*);
            void unsetc(BleepsBoxHandler, uint32_t, uint32_t, uint32_t);

            void attach_box(BleepsBoxHandler, uint32_t, uint32_t);
            void detach_box(BleepsBoxHandler, uint32_t);

            void draw(BleepsBoxHandler);
            void kill(BleepsBoxHandler);
        """)

        self.lib = ffi.dlopen(self.SO_PATH)
        self.width, self.height = get_terminal_size()
        self.boxhandler = self.lib.init(self.width, self.height)

    def box_attach(self, box_id, parent_id, position=(0,0)):
        self.lib.attach_box(self.boxhandler, box_id, parent_id)
        if (position != (0,0)):
            self.box_move(box_id, *position)

    def box_detach(self, box_id):
        self.lib.detach_box(self.boxhandler, box_id)

    def box_disable(self, box_id):
        self.lib.disable_box(self.boxhandler, box_id)

    def box_enable(self, box_id):
        self.lib.enable_box(self.boxhandler, box_id)

    def box_setc(self, box_id, x, y, character):
        fmt_character = bytes(character, 'utf-8')
        self.lib.setc(self.boxhandler, box_id, x, y, fmt_character)

    def box_unsetc(self, box_id, x, y):
        self.lib.unsetc(self.boxhandler, box_id, x, y)

    def box_unset_bg_color(self, box_id):
        self.lib.unset_bg_color(self.boxhandler, box_id)

    def box_unset_fg_color(self, box_id):
        self.lib.unset_fg_color(self.boxhandler, box_id)

    def box_unset_color(self, box_id):
        self.lib.unset_color(self.boxhandler, box_id)

    def box_set_bg_color(self, box_id, color):
        self.lib.set_bg_color(self.boxhandler, box_id, color)

    def box_set_fg_color(self, box_id, color):
        self.lib.set_fg_color(self.boxhandler, box_id, color)

    def box_move(self, box_id, x, y):
        self.lib.movebox(self.boxhandler, box_id, x, y)

    def _new_box(self, width, height, parent=0):

        new_box_id = self.lib.newbox(self.boxhandler, parent, width, height)
        return BleepsBox(new_box_id, width, height, self)

    def new_box(self, width, height):
        return self._new_box(width, height)

    def draw(self):
        self.lib.draw(self.boxhandler)

    def kill(self):
        self.lib.kill(self.boxhandler)


class BleepsBox(object):
    def __init__(self, n, width, height, screen):
        self._screen  = screen
        self.bleeps_id = n
        self.boxes = {}
        self.width = width
        self.height = height
        self.enabled = True

    def attach(self, childbox):
        self._screen.box_attach(childbox.bleeps_id, self.bleeps_id)

    def detach(self):
        self._screen.box_detach(self.bleeps_id)

    def enable(self):
        self.enabled = True
        self._screen.box_enable(self.bleeps_id);

    def disable(self):
        self.enabled = False
        self._screen.box_disable(self.bleeps_id);

    def refresh(self):
        self._screen.draw()

    def setc(self, x, y, character):
        self._screen.box_setc(self.bleeps_id, x, y, character)

    def unsetc(self, x, y):
        self._screen.box_unsetc(self.bleeps_id, x, y)

    def move(self, new_x, new_y):
        self._screen.box_move(self.bleeps_id, new_x, new_y)

    def set_fg_color(self, new_col):
        self._screen.box_set_fg_color(self.bleeps_id, new_col)

    def set_bg_color(self, new_col):
        self._screen.box_set_bg_color(self.bleeps_id, new_col)

    def unset_fg_color(self):
        self._screen.box_unset_fg_color(self.bleeps_id)

    def unset_bg_color(self):
        self._screen.box_unset_bg_color(self.bleeps_id)

    def unset_color(self):
        self._screen.box_unset_bg_color(self.bleeps_id)

    def new_box(self, width, height):
        box = self._screen._new_box(width, height, self.bleeps_id)
        self.boxes[box.bleeps_id] = box

        return box


if __name__ == "__main__":
    screen = BleepsScreen()
    box = screen.new_box(10, 10)
    for y in range(box.height):
        box.setc(0, y, '|')
        box.setc(box.width - 1, y, '|')

    for x in range(box.width):
        box.setc(x, 0, '=')
        box.setc(x, box.height - 1, '=')

    box.set_bg_color(4)

    import time
    box.refresh()

    time.sleep(2)
    screen.kill()

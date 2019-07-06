from cffi import FFI
from ctypes import c_bool
import sys
import tty, termios
import os
from localfuncs import get_terminal_size

class BleepsScreen:
    SO_PATH = os.path.dirname(os.path.realpath(__file__)) + "/libasciibox.so"

    def __init__(self):
        super().__init__()
        ffi = FFI()
        ffi.cdef("""
            typedef void* BleepsBoxHandler;

            BleepsBoxHandler init(uint32_t, uint32_t);

            uint32_t newbox(BleepsBoxHandler, uint32_t, uint32_t, uint32_t);

            void movebox(BleepsBoxHandler, uint32_t, int32_t, int32_t);
            void resize(BleepsBoxHandler, uint32_t, uint32_t, uint32_t);
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
            void fillc(BleepsBoxHandler, uint32_t, const char*);

            void attachbox(BleepsBoxHandler, uint32_t, uint32_t);
            void detachbox(BleepsBoxHandler, uint32_t);

            void draw(BleepsBoxHandler, uint32_t);
            void kill(BleepsBoxHandler);

            void removebox(BleepsBoxHandler, uint32_t);
        """)

        self.lib = ffi.dlopen(self.SO_PATH)
        self.width, self.height = get_terminal_size()
        self.boxhandler = self.lib.init(self.width, self.height)

        self.locked = False
        self._serving = 0
        self._queue_number = 0

    def lock(self):
        if self.locked:
            my_number = self._queue_number
            self._queue_number += 1
            while self._serving != my_number and self.locked:
                time.sleep(.01)
        else:
            self._queue_number = 0
            self._serving = 0

        self.locked = True

    def unlock(self):
        self.locked = False
        self._serving += 1

    def box_flag_cache(self, box_id):
        self.lock()

        self.lib.flag_recache(self.boxhandler, box_id)

        self.unlock()

    def box_attach(self, box_id, parent_id, position=(0,0)):
        self.lock()

        self.lib.attachbox(self.boxhandler, box_id, parent_id)
        if (position != (0,0)):
            self.box_move(box_id, *position)

        self.unlock()

    def box_detach(self, box_id):
        self.lock()

        self.lib.detachbox(self.boxhandler, box_id)

        self.unlock()

    def box_disable(self, box_id):
        self.lock()

        self.lib.disable_box(self.boxhandler, box_id)

        self.unlock()

    def box_enable(self, box_id):
        self.lock()

        self.lib.enable_box(self.boxhandler, box_id)

        self.unlock()

    def box_setc(self, box_id, x, y, character):
        self.lock()

        fmt_character = bytes(character, 'utf-8')
        self.lib.setc(self.boxhandler, box_id, x, y, fmt_character)

        self.unlock()

    def box_fillc(self, box_id, character):
        self.lock()

        fmt_character = bytes(character, 'utf-8')
        self.lib.fillc(self.boxhandler, box_id, fmt_character)

        self.unlock()

    def box_unsetc(self, box_id, x, y):
        self.lock()

        self.lib.unsetc(self.boxhandler, box_id, x, y)

        self.unlock()

    def box_unset_bg_color(self, box_id):
        self.lock()

        self.lib.unset_bg_color(self.boxhandler, box_id)

        self.unlock()

    def box_unset_fg_color(self, box_id):
        self.lock()

        self.lib.unset_fg_color(self.boxhandler, box_id)

        self.unlock()

    def box_unset_color(self, box_id):
        self.lock()

        self.lib.unset_color(self.boxhandler, box_id)

    def box_set_bg_color(self, box_id, color):
        self.lock()

        self.lib.set_bg_color(self.boxhandler, box_id, color)

        self.unlock()

    def box_set_fg_color(self, box_id, color):
        self.lock()

        self.lib.set_fg_color(self.boxhandler, box_id, color)

        self.unlock()

    def box_move(self, box_id, x, y):
        self.lock()

        self.lib.movebox(self.boxhandler, box_id, x, y)

        self.unlock()

    def box_resize(self, box_id, width, height):
        self.lock()

        self.lib.resize(self.boxhandler, box_id, width, height)

        self.unlock()

    def new_box(self, **kwargs):
        self.lock()

        width = 1
        if 'width' in kwargs.keys():
            width = kwargs['width']
        height = 1
        if 'height' in kwargs.keys():
            height = kwargs['height']
        parent = 0
        if 'parent' in kwargs.keys():
            parent = kwargs['parent']

        new_box_id = self.lib.newbox(self.boxhandler, parent, width, height)

        self.unlock()
        return BleepsBox(new_box_id, self, width=width, height=height)

    def box_draw(self, box_id):
        self.lock()

        self.lib.draw(self.boxhandler, box_id)

        self.unlock()

    def box_remove(self, box_id):
        self.lock()

        self.lib.removebox(self.boxhandler, box_id)

        self.unlock()

    def draw(self):
        self.lock()

        self.lib.draw(self.boxhandler, 0)

        self.unlock()

    def kill(self):
        self.lib.kill(self.boxhandler)


class BleepsBox(object):
    BLACK = 0
    RED = 1
    GREEN = 2
    YELLOW = 3
    BLUE = 4
    MAGENTA = 5
    CYAN = 6
    WHITE = 7
    BRIGHT = 0x08
    BRIGHTBLACK = BLACK | BRIGHT
    BRIGHTRED = RED |  BRIGHT
    BRIGHTGREEN = GREEN | BRIGHT
    BRIGHTYELLOW = YELLOW | BRIGHT
    BRIGHTBLUE = BLUE | BRIGHT
    BRIGHTMAGENTA = MAGENTA | BRIGHT
    BRIGHTCYAN = CYAN | BRIGHT
    BRIGHTWHITE = WHITE | BRIGHT

    def __init__(self, n, screen, **kwargs):
        self._screen  = screen
        self.bleeps_id = n
        self.boxes = {}
        self.parent = None
        self.enabled = True

        self.width = 1
        if 'width' in kwargs.keys():
            self.width = kwargs['width']
        self.height = 1
        if 'height' in kwargs.keys():
            self.height = kwargs['height']

    def flag_cache(self):
        self._screen.box_flag_cache(self.bleeps_id)

    def attach(self, childbox):
        self.boxes[childbox.bleeps_id] = childbox
        self._screen.box_attach(childbox.bleeps_id, self.bleeps_id)

    def resize(self, width, height):
        self.width = width
        self.height = height
        self._screen.box_resize(self.bleeps_id, width, height)

    def detach(self):
        try:
            del self.parent.boxes[self.bleeps_id]
        except:
            pass

        self._screen.box_detach(self.bleeps_id)

    def fill(self, character):
        self._screen.box_fillc(self.bleeps_id, character)

    def enable(self):
        self.enabled = True
        self._screen.box_enable(self.bleeps_id);

    def disable(self):
        self.enabled = False
        self._screen.box_disable(self.bleeps_id);

    def draw(self):
        self._screen.box_draw(self.bleeps_id)

    def refresh(self):
        self._screen.draw()

    def remove(self):
        self._screen.box_remove(self.bleeps_id)

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

    def new_box(self, **kwargs):
        kwargs['parent'] = self.bleeps_id
        box = self._screen.new_box(**kwargs)
        self.boxes[box.bleeps_id] = box
        box.parent = self

        return box


if __name__ == "__main__":
    screen = BleepsScreen()
    box = screen.new_box(width=10, height=10)
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


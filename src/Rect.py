from cffi import FFI
from ctypes import c_bool
import sys
import tty, termios
import os
from localfuncs import get_terminal_size

def logg(error_code, args, msg):
    strargs = '('
    for i, a in enumerate(args):
        strargs += str(a)
        if i < len(args) - 1:
            strargs += ', '
    strargs += ")"
    newline = "%d - %s: %s\n" % (error_code, strargs, msg)

    with open("logg", "a") as fp:
        fp.write(newline)

class RectManager:
    #SO_PATH = "/home/pent/Projects/100/target/debug/libasciibox.so"
    SO_PATH = "/home/pent/Projects/100/target/release/libasciibox.so"


    def __init__(self):
        ffi = FFI()
        ffi.cdef("""
            typedef void* RectManager;

            RectManager init(uint32_t, uint32_t);
            void kill(RectManager);


            uint32_t new_rect(RectManager, uint32_t, uint32_t, uint32_t);
            uint32_t delete_rect(RectManager, uint32_t);


            uint32_t set_position(RectManager, uint32_t, int32_t, int32_t);
            uint32_t resize(RectManager, uint32_t, uint32_t, uint32_t);
            uint32_t attach(RectManager, uint32_t, uint32_t);
            uint32_t detach(RectManager, uint32_t);


            uint32_t unset_color(RectManager, uint32_t);
            uint32_t set_bg_color(RectManager, uint32_t, uint8_t);
            uint32_t set_fg_color(RectManager, uint32_t, uint8_t);
            uint32_t unset_bg_color(RectManager, uint32_t);
            uint32_t unset_fg_color(RectManager, uint32_t);

            uint32_t disable_rect(RectManager, uint32_t);
            uint32_t enable_rect(RectManager, uint32_t);

            uint32_t set_character(RectManager, uint32_t, uint32_t, uint32_t, const char*);
            uint32_t unset_character(RectManager, uint32_t, uint32_t, uint32_t);

            uint32_t draw(RectManager, uint32_t);
        """)

        self.lib = ffi.dlopen(self.SO_PATH)
        self.width, self.height = get_terminal_size()
        self.rectmanager = self.lib.init(self.width, self.height)

        self._serving = 0
        self._queue_number = 0


    def rect_attach(self, rect_id, parent_id, position=(0,0)):
        self.lib.attach(self.rectmanager, rect_id, parent_id)
        if (position != (0,0)):
            err = self.rect_move(rect_id, *position)

            logg(err, [rect_id], 'attach')

    def rect_detach(self, rect_id):
        err = self.lib.detach(self.rectmanager, rect_id)
        logg(err, [rect_id], 'dettach')


    def rect_disable(self, rect_id):
        err = self.lib.disable(self.rectmanager, rect_id)
        logg(err, [rect_id], 'disable')


    def rect_enable(self, rect_id):
        err = self.lib.enable(self.rectmanager, rect_id)
        logg(err, [rect_id], 'enable')


    def rect_set_character(self, rect_id, x, y, character):
        fmt_character = bytes(character, 'utf-8')
        err = self.lib.set_character(self.rectmanager, rect_id, x, y, fmt_character)

        logg(err, [rect_id, x, y, character], 'set_character')

    def rect_unset_character(self, rect_id, x, y):
        err = self.lib.unset_character(self.rectmanager, rect_id, x, y)
        logg(err, [rect_id, x, y], 'unset_character')

    def rect_unset_bg_color(self, rect_id):
        err = self.lib.unset_bg_color(self.rectmanager, rect_id)

    def rect_unset_fg_color(self, rect_id):
        err = self.lib.unset_fg_color(self.rectmanager, rect_id)


    def rect_unset_color(self, rect_id):
        err = self.lib.unset_color(self.rectmanager, rect_id)

    def rect_set_bg_color(self, rect_id, color):
        err = self.lib.set_bg_color(self.rectmanager, rect_id, color)

    def rect_set_fg_color(self, rect_id, color):
        err = self.lib.set_fg_color(self.rectmanager, rect_id, color)

    def rect_move(self, rect_id, x, y):
        err = self.lib.set_position(self.rectmanager, rect_id, x, y)
        logg(err, [rect_id, x, y], 'move')

    def rect_resize(self, rect_id, width, height):
        err = self.lib.resize(self.rectmanager, rect_id, width, height)
        logg(err, [rect_id, width, height], 'resize')

    def new_rect(self, **kwargs):
        width = 1
        if 'width' in kwargs.keys():
            width = kwargs['width']

        height = 1
        if 'height' in kwargs.keys():
            height = kwargs['height']

        parent = 0
        if 'parent' in kwargs.keys():
            parent = kwargs['parent']

        new_rect_id = self.lib.new_rect(self.rectmanager, parent, width, height)

        return Rect(new_rect_id, self, width=width, height=height)


    def rect_draw(self, rect_id):
        err = self.lib.draw(self.rectmanager, rect_id)
        logg(err, [rect_id], 'draw')

    def rect_remove(self, rect_id):
        err = self.lib.delete_rect(self.rectmanager, rect_id)
        logg(err, [rect_id], 'remove')


    def kill(self):
        self.lib.kill(self.rectmanager)

    def draw(self):
        self.rect_draw(0)

class Rect(object):
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
        self.rect_id = n
        self.rects = {}
        self.parent = None
        self.enabled = True
        self.x = 0
        self.y = 0

        self.width = 1
        if 'width' in kwargs.keys():
            self.width = kwargs['width']
        self.height = 1
        if 'height' in kwargs.keys():
            self.height = kwargs['height']


    def attach(self, child_rect):
        self.rects[child_rect.rect_id] = child_rect
        if (child_rect.x or child_rect.y):
            position = (child_rect.x, child_rect.y)
        else:
            position = (0, 0)
        self._screen.rect_attach(child_rect.rect_id, self.rect_id, position)

    def resize(self, width, height):
        self.width = width
        self.height = height
        self._screen.rect_resize(self.rect_id, width, height)

    def detach(self):
        try:
            del self.parent.rects[self.rect_id]
        except:
            pass

        self._screen.rect_detach(self.rect_id)

    #def fill(self, character):
    #    self._screen.rect_fill(self.rect_id, character)

    def enable(self):
        self.enabled = True
        self._screen.rect_enable(self.rect_id);

    def disable(self):
        self.enabled = False
        self._screen.rect_disable(self.rect_id);

    def draw(self):
        self._screen.rect_draw(self.rect_id)

    def refresh(self):
        self._screen.draw()

    def remove(self):
        self._screen.rect_remove(self.rect_id)

    def set_character(self, x, y, character):
        self._screen.rect_set_character(self.rect_id, x, y, character)

    def unset_character(self, x, y):
        self._screen.rect_unset_character(self.rect_id, x, y)

    def move(self, new_x, new_y):
        self.x = new_x
        self.y = new_y
        self._screen.rect_move(self.rect_id, new_x, new_y)

    def set_fg_color(self, new_col):
        self._screen.rect_set_fg_color(self.rect_id, new_col)

    def set_bg_color(self, new_col):
        self._screen.rect_set_bg_color(self.rect_id, new_col)

    def unset_fg_color(self):
        self._screen.rect_unset_fg_color(self.rect_id)

    def unset_bg_color(self):
        self._screen.rect_unset_bg_color(self.rect_id)

    def unset_color(self):
        self._screen.rect_unset_bg_color(self.rect_id)

    def new_rect(self, **kwargs):
        kwargs['parent'] = self.rect_id
        rect = self._screen.new_rect(**kwargs)
        self.rects[rect.rect_id] = rect
        rect.parent = self

        return rect

if __name__ == "__main__":
    import time, math
    screen = RectManager()

    rect = screen.new_rect(width=screen.width , height=screen.height // 2)
    screen.rect_set_character(0, 4, 0, "Y")
    rect.set_character(4, 2, "Y")
    rect.move(1,1)
   # rect.set_bg_color(4)


#    for i in range(10):
#        rect = rect.new_rect(width=screen.width, height=screen.height)
#    new_rect = rect.new_rect(width=5, height=5)
#    new_rect.move(4, 4)
#
#    new_rect.set_character(4, 0, "Z")
#    new_rect.set_bg_color(1)
#    new_rect.set_fg_color(3)
#    new_rect.draw()

    screen.draw()

    input()

    screen.kill()



from cffi import FFI
from ctypes import c_bool
import sys
import tty, termios
import os
from localfuncs import get_terminal_size

class RectManager:
    #SO_PATH = os.path.dirname(os.path.realpath(__file__)) + "/libasciibox.so"
    #SO_PATH = "/mnt/media/projects/100/target/debug/libasciibox.so"
    SO_PATH = "/home/pent/Projects/100/target/debug/libasciibox.so"

    def __init__(self):
        ffi = FFI()
        ffi.cdef("""
            typedef void* RectManager;

            RectManager init(uint32_t, uint32_t);
            void kill(RectManager);


            uint32_t new_rect(RectManager, uint32_t, uint32_t, uint32_t);
            void delete_rect(RectManager, uint32_t);


            void set_position(RectManager, uint32_t, int32_t, int32_t);
            void resize(RectManager, uint32_t, uint32_t, uint32_t);
            void attach(RectManager, uint32_t, uint32_t);
            void detach(RectManager, uint32_t);


            void unset_color(RectManager, uint32_t);
            void set_bg_color(RectManager, uint32_t, uint8_t);
            void set_fg_color(RectManager, uint32_t, uint8_t);
            void unset_bg_color(RectManager, uint32_t);
            void unset_fg_color(RectManager, uint32_t);

            void disable_rect(RectManager, uint32_t);
            void enable_rect(RectManager, uint32_t);

            void set_character(RectManager, uint32_t, uint32_t, uint32_t, const char*);
            void unset_character(RectManager, uint32_t, uint32_t, uint32_t);

            void draw(RectManager, uint32_t);
        """)

        self.lib = ffi.dlopen(self.SO_PATH)
        self.width, self.height = get_terminal_size()
        self.rectmanager = self.lib.init(self.width, self.height)

        self._serving = 0
        self._queue_number = 0


    def rect_attach(self, rect_id, parent_id, position=(0,0)):
        self.lib.attach(self.rectmanager, rect_id, parent_id)
        if (position != (0,0)):
            self.rect_move(rect_id, *position)

    def rect_detach(self, rect_id):
        self.lib.detach(self.rectmanager, rect_id)


    def rect_disable(self, rect_id):
        self.lib.disable(self.rectmanager, rect_id)


    def rect_enable(self, rect_id):
        self.lib.enable(self.rectmanager, rect_id)


    def rect_set_character(self, rect_id, x, y, character):
        fmt_character = bytes(character, 'utf-8')
        self.lib.set_character(self.rectmanager, rect_id, x, y, fmt_character)

    def rect_unset_character(self, rect_id, x, y):
        self.lib.unset_character(self.rectmanager, rect_id, x, y)

    def rect_unset_bg_color(self, rect_id):
        self.lib.unset_bg_color(self.rectmanager, rect_id)

    def rect_unset_fg_color(self, rect_id):
        self.lib.unset_fg_color(self.rectmanager, rect_id)


    def rect_unset_color(self, rect_id):
        self.lib.unset_color(self.rectmanager, rect_id)

    def rect_set_bg_color(self, rect_id, color):
        self.lib.set_bg_color(self.rectmanager, rect_id, color)

    def rect_set_fg_color(self, rect_id, color):
        self.lib.set_fg_color(self.rectmanager, rect_id, color)

    def rect_move(self, rect_id, x, y):
        self.lib.set_position(self.rectmanager, rect_id, x, y)

    def rect_resize(self, rect_id, width, height):
        self.lib.resize(self.rectmanager, rect_id, width, height)

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
        self.lib.draw(self.rectmanager, rect_id)

    def rect_remove(self, rect_id):
        self.lib.remove(self.rectmanager, rect_id)

    def draw(self):
        self.lib.draw(self.rectmanager, 0)

    def kill(self):
        self.lib.kill(self.rectmanager)

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
        self._screen.rect_attach(child_rect.rect_id, self.rect_id)

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

    def fill(self, character):
        self._screen.rect_fillc(self.rect_id, character)

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
    import time
    screen = RectManager()
    screen.rect_set_fg_color(0, 3)

    rect = screen.new_rect(width=30, height=20)
    new_rect = rect.new_rect(width=5, height=5)
    new_rect.move(4, 4)

    new_rect.set_bg_color(1)
    new_rect.set_fg_color(3)

    for y in range(new_rect.height):
        new_rect.set_character(0, y, '|')
        new_rect.set_character(new_rect.width - 1, y, '|')

    for x in range(new_rect.width):
        new_rect.set_character(x, 0, '=')
        new_rect.set_character(x, new_rect.height - 1, '=')

    for i in range(5):
        new_rect.move(i, i)

        screen.draw()
        #new_rect.draw()
        time.sleep(1)

    screen.kill()



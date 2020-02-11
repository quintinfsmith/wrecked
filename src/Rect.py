from cffi import FFI
from ctypes import c_bool
import sys
import tty, termios
import os, time
from localfuncs import get_terminal_size
import json

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

class RectError(Exception):
    def __init__(self, **kwargs):
        self.msg = json.dumps(kwargs)
        super().__init__(self.msg)

class NotFound(RectError):
    pass
class ParentNotFound(RectError):
    pass
class OutOfBounds(RectError):
    pass
class NoParent(RectError):
    pass
class ChildNotFound(RectError):
    pass

EXCEPTIONS = {
    0: None,
    1: OutOfBounds,
    2: NotFound,
    3: ParentNotFound,
    4: NoParent,
    8: ChildNotFound
}

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

    def replace_with(self, rect):
        self._screen.rect_replace_with(self.rect_id, rect.rect_id)

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

    def queue_draw(self):
        self._screen.rect_queue_draw(self.rect_id)

    def draw_queued(self):
        self._screen.draw_queued()

    def refresh(self):
        self._screen.draw()

    def remove(self):
        self._screen.rect_remove(self.rect_id)

    def set_character(self, x, y, character):
        self._screen.rect_set_character(self.rect_id, x, y, character)

    def set_string(self, x, y, string):
        self._screen.rect_set_string(self.rect_id, x, y, string)

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

    def empty(self):
        self._screen.rect_empty(self.rect_id)

    def clear(self):
        self._screen.rect_clear(self.rect_id)

    def new_rect(self, **kwargs):
        kwargs['parent'] = self.rect_id
        rect = self._screen.create_rect(**kwargs)
        self.rects[rect.rect_id] = rect
        rect.parent = self

        return rect

class RectManager:
    #SO_PATH = "/home/pent/Projects/100/target/debug/libasciibox.so"
    SO_PATH = "/home/pent/Projects/100/target/release/libasciibox.so"
    RECT_CONSTRUCTOR = Rect



    def __init__(self):
        ffi = FFI()
        ffi.cdef("""
            typedef void* RectManager;

            RectManager init(uint32_t, uint32_t);
            void kill(RectManager);


            uint32_t new_rect(RectManager, uint32_t, uint32_t, uint32_t);
            uint32_t delete_rect(RectManager, uint32_t);
            uint32_t draw_queued(RectManager);


            uint32_t set_position(RectManager, uint32_t, int32_t, int32_t);
            uint32_t resize(RectManager, uint32_t, uint32_t, uint32_t);
            uint32_t attach(RectManager, uint32_t, uint32_t);
            uint32_t detach(RectManager, uint32_t);

            uint32_t empty(RectManager, uint32_t);
            uint32_t clear(RectManager, uint32_t);


            uint32_t unset_color(RectManager, uint32_t);
            uint32_t set_bg_color(RectManager, uint32_t, uint8_t);
            uint32_t set_fg_color(RectManager, uint32_t, uint8_t);
            uint32_t unset_bg_color(RectManager, uint32_t);
            uint32_t unset_fg_color(RectManager, uint32_t);

            uint32_t disable_rect(RectManager, uint32_t);
            uint32_t enable_rect(RectManager, uint32_t);

            uint32_t set_character(RectManager, uint32_t, uint32_t, uint32_t, const char*);
            uint32_t set_string(RectManager, uint32_t, uint32_t, uint32_t, const char*);
            uint32_t unset_character(RectManager, uint32_t, uint32_t, uint32_t);

            uint32_t draw(RectManager, uint32_t);
            uint32_t queue_draw(RectManager, uint32_t);

            uint32_t replace_with(RectManager, uint32_t, uint32_t);

        """)

        self.lib = ffi.dlopen(self.SO_PATH)
        self.width, self.height = get_terminal_size()
        self.rectmanager = self.lib.init(self.width, self.height)

        self.root = self.RECT_CONSTRUCTOR(0, self, width=self.width, height=self.height)


    def draw_queued(self):
        err = self.lib.draw_queued(self.rectmanager)

        if err:
            raise EXCEPTIONS[err]()


    def rect_queue_draw(self, rect_id):
        err = self.lib.queue_draw(self.rectmanager, rect_id)


        if err:
            raise EXCEPTIONS[err](rect_id=rect_id)


    def rect_attach(self, rect_id, parent_id, position=(0,0)):
        self.lib.attach(self.rectmanager, rect_id, parent_id)
        err = 0
        if (position != (0,0)):
            err = self.rect_move(rect_id, *position)


        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id,
                parent_id=parent_id,
                position=position
            )

    def rect_detach(self, rect_id):
        err = self.lib.detach(self.rectmanager, rect_id)

        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_replace_with(self, old_id, new_id):
        err = self.lib.replace_with(self.rectmanager, old_id, new_id)

        if err:
            raise EXCEPTIONS[err](
                old_rect_id=old_id,
                new_rect_id=new_id
            )

    def rect_empty(self, rect_id):
        err = self.lib.empty(self.rectmanager, rect_id)

        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_clear(self, rect_id):
        err = self.lib.clear(self.rectmanager, rect_id)

        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_disable(self, rect_id):
        err = self.lib.disable(self.rectmanager, rect_id)

        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )


    def rect_enable(self, rect_id):
        err = self.lib.enable(self.rectmanager, rect_id)

        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )


    def rect_set_character(self, rect_id, x, y, character):
        fmt_character = bytes(character, 'utf-8')
        err = self.lib.set_character(self.rectmanager, rect_id, x, y, fmt_character)


        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id,
                position=(x, y),
                character=character
            )

    def rect_set_string(self, rect_id, x, y, string):
        fmt_string = bytes(string, 'utf-8')
        err = self.lib.set_string(self.rectmanager, rect_id, x, y, fmt_string)


        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id,
                position=(x, y),
                string=string
            )


    def rect_unset_character(self, rect_id, x, y):
        err = self.lib.unset_character(self.rectmanager, rect_id, x, y)

        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id,
                position=(x, y),
                character=character
            )

    def rect_unset_bg_color(self, rect_id):
        err = self.lib.unset_bg_color(self.rectmanager, rect_id)


        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_unset_fg_color(self, rect_id):
        err = self.lib.unset_fg_color(self.rectmanager, rect_id)


        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )


    def rect_unset_color(self, rect_id):
        err = self.lib.unset_color(self.rectmanager, rect_id)


        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_set_bg_color(self, rect_id, color):
        err = self.lib.set_bg_color(self.rectmanager, rect_id, color)

        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id,
                color=color
            )

    def rect_set_fg_color(self, rect_id, color):
        err = self.lib.set_fg_color(self.rectmanager, rect_id, color)

        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id,
                color=color
            )

    def rect_move(self, rect_id, x, y):
        err = self.lib.set_position(self.rectmanager, rect_id, x, y)

        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id,
                position=(x, y)
            )

    def rect_resize(self, rect_id, width, height):
        err = self.lib.resize(self.rectmanager, rect_id, width, height)

        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id,
                dimensions=(width, height)
            )


    # TODO: Handle Errors here
    def create_rect(self, **kwargs):
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

        return self.RECT_CONSTRUCTOR(new_rect_id, self, width=width, height=height)


    def rect_draw(self, rect_id):
        err = self.lib.draw(self.rectmanager, rect_id)


        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id,
                dimensions=(width, height)
            )

    def rect_remove(self, rect_id):
        err = self.lib.delete_rect(self.rectmanager, rect_id)

        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id
            )


    def kill(self):
        rects = []
        for rect_id, rect in self.root.rects.items():
            rects.append(rect)

        while rects:
            rects.pop().detach()

        self.lib.kill(self.rectmanager)

    def draw(self):
        self.rect_draw(0)


if __name__ == "__main__":
    import time, math
    screen = RectManager()

    rect = screen.root.new_rect(
        width=5,
        height=20
    )
    rect.set_bg_color(Rect.BLUE)

    screen.root.set_string(0, 0, "WHOOT" )
    #screen.rect_set_character(0, 4, 0, "Y")
    #screen.rect_set_bg_color(0, Rect.BRIGHTMAGENTA)

    rect.set_string(0, 0, "Yolo")
    rect.move(3,3)

    #rect.set_bg_color(Rect.BLUE)
    #new_rect = rect.new_rect(width=5, height=5)
    #new_rect.move(4, 4)

    #new_rect.set_character(4, 0, "Z")
    #new_rect.set_bg_color(Rect.RED)
    #new_rect.set_fg_color(3)

    ##screen.root.queue_draw()
    ##new_rect.queue_draw()
    ##rect.queue_draw()

    #screen.root.draw()

    screen.draw()
    screen.root.draw()

    input()

    screen.kill()



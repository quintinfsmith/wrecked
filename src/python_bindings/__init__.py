from cffi import FFI
from ctypes import c_bool
import sys, site
import tty, termios
import os, time
import json
import threading
import logging
import platform

def get_terminal_size():
    '''return dimensions of current terminal session'''
    height, width = os.popen("stty size", "r").read().split()
    return (int(width), int(height))

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

class WreckedError(Exception):
    def __init__(self, **kwargs):
        self.msg = json.dumps(kwargs)
        super().__init__(self.msg)

class NotFound(WreckedError):
    pass
class ParentNotFound(WreckedError):
    pass
class OutOfBounds(WreckedError):
    pass
class NoParent(WreckedError):
    pass
class ChildNotFound(WreckedError):
    pass
class BadColor(WreckedError):
    pass
class InvalidUtf8(WreckedError):
    pass
class StringOverflow(WreckedError):
    pass
class UnknownError(WreckedError):
    pass

EXCEPTIONS = {
    0: None,
    1: BadColor,
    2: InvalidUtf8,
    3: StringOverflow,
    4: NotFound,
    5: NoParent,
    6: ParentNotFound,
    7: ChildNotFound,
    8: OutOfBounds,
    255: UnknownError
}

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

class Rect(object):
    def __init__(self, n, rectmanager, **kwargs):
        self.rectmanager  = rectmanager
        self.rect_id = n
        self.rects = {}
        self.parent = None
        self.enabled = True
        self.x = 0
        self.y = 0
        self.transparent = False

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
        self.rectmanager.rect_attach(child_rect.rect_id, self.rect_id, position)

    def resize(self, width, height):
        self.width = width
        self.height = height
        self.rectmanager.rect_resize(self.rect_id, width, height)

    def detach(self):
        try:
            del self.parent.rects[self.rect_id]
        except:
            pass

        self.rectmanager.rect_detach(self.rect_id)

    def replace_with(self, rect):
        self.rectmanager.rect_replace_with(self.rect_id, rect.rect_id)

    def enable(self):
        self.enabled = True
        self.rectmanager.rect_enable(self.rect_id)

    def disable(self):
        self.enabled = False
        self.rectmanager.rect_disable(self.rect_id)

    def draw(self):
        self.rectmanager.rect_draw(self.rect_id)

    def refresh(self):
        self.rectmanager.draw()

    def remove(self):
        self.rectmanager.rect_remove(self.rect_id)

    def set_character(self, x, y, character):
        self.rectmanager.rect_set_character(self.rect_id, x, y, character)

    def set_string(self, x, y, string):
        self.rectmanager.rect_set_string(self.rect_id, x, y, string)

    def unset_character(self, x, y):
        self.rectmanager.rect_unset_character(self.rect_id, x, y)

    def move(self, new_x, new_y):
        self.x = new_x
        self.y = new_y
        self.rectmanager.rect_move(self.rect_id, new_x, new_y)

    def invert(self):
        self.rectmanager.rect_invert(self.rect_id)

    def underline(self):
        self.rectmanager.rect_underline(self.rect_id)

    def bold(self):
        self.rectmanager.rect_bold(self.rect_id)

    def unset_invert(self):
        self.rectmanager.rect_unset_invert(self.rect_id)

    def unset_underline(self):
        self.rectmanager.rect_unset_underline(self.rect_id)

    def unset_bold(self):
        self.rectmanager.rect_unset_bold(self.rect_id)

    def set_fg_color(self, new_col):
        self.rectmanager.rect_set_fg_color(self.rect_id, new_col)

    def set_bg_color(self, new_col):
        self.rectmanager.rect_set_bg_color(self.rect_id, new_col)

    def unset_fg_color(self):
        self.rectmanager.rect_unset_fg_color(self.rect_id)

    def unset_bg_color(self):
        self.rectmanager.rect_unset_bg_color(self.rect_id)

    def unset_color(self):
        self.rectmanager.rect_unset_color(self.rect_id)

    def clear_children(self):
        self.rectmanager.rect_clear_children(self.rect_id)

    def clear_characters(self):
        self.rectmanager.rect_clear_characters(self.rect_id)

    def new_rect(self, **kwargs):
        kwargs['parent'] = self.rect_id
        rect = self.rectmanager.create_rect(**kwargs)
        self.rects[rect.rect_id] = rect
        rect.parent = self

        return rect

    def shift_contents(self, x, y):
        self.rectmanager.rect_shift_contents(self.rect_id, x, y)

    def set_transparency(self, transparency):
        self.rectmanager.rect_set_transparency(self.rect_id, transparency)
        self.transparent = transparency


class RectManager:
    def __init__(self):
        ffi = FFI()
        ffi.cdef("""
            typedef void* RectManager;

            RectManager init();
            bool fit_to_terminal(RectManager);
            void kill(RectManager);


            uint64_t new_rect(RectManager, uint64_t, uint64_t, uint64_t);
            uint64_t new_orphan(RectManager, uint64_t, uint64_t);
            uint32_t delete_rect(RectManager, uint64_t);


            uint32_t set_position(RectManager, uint64_t, int64_t, int64_t);
            uint32_t resize(RectManager, uint64_t, uint64_t, uint64_t);
            uint32_t attach(RectManager, uint64_t, uint64_t);
            uint32_t detach(RectManager, uint64_t);

            uint32_t clear_children(RectManager, uint64_t);
            uint32_t clear_characters(RectManager, uint64_t);


            uint32_t unset_color(RectManager, uint64_t);
            uint32_t set_bg_color(RectManager, uint64_t, uint8_t);
            uint32_t set_fg_color(RectManager, uint64_t, uint8_t);
            uint32_t unset_bg_color(RectManager, uint64_t);
            uint32_t unset_fg_color(RectManager, uint64_t);

            void set_bold_flag(RectManager, uint64_t);
            void unset_bold_flag(RectManager, uint64_t);
            void set_underline_flag(RectManager, uint64_t);
            void unset_underline_flag(RectManager, uint64_t);
            void set_invert_flag(RectManager, uint64_t);
            void unset_invert_flag(RectManager, uint64_t);

            uint32_t disable_rect(RectManager, uint64_t);
            uint32_t enable_rect(RectManager, uint64_t);

            uint32_t set_character(RectManager, uint64_t, int64_t, int64_t, const char*);
            uint32_t set_string(RectManager, uint64_t, int64_t, int64_t, const char*);
            uint32_t unset_character(RectManager, uint64_t, int64_t, int64_t);

            uint32_t render(RectManager, uint64_t);

            uint32_t replace_with(RectManager, uint64_t, uint64_t);

            uint32_t shift_contents(RectManager, uint64_t, int64_t, int64_t);

            uint64_t get_height(RectManager, uint64_t);
            uint64_t get_width(RectManager, uint64_t);

            uint32_t set_transparency(RectManager, uint64_t, bool);

        """)

        lib_path = __file__[0:__file__.rfind("/") + 1] + "libwrecked_manylinux2014_" + platform.machine() + ".so"
        self.lib = ffi.dlopen(lib_path)

        self.rectmanager = self.lib.init()
        self.width = self.lib.get_width(self.rectmanager, 0)
        self.height = self.lib.get_height(self.rectmanager, 0)

        self.root = Rect(0, self, width=self.width, height=self.height)
        self.root.draw()

    def resize(self, new_width, new_height):
        self.width = new_width
        self.height = new_height
        self.root.resize(new_width, new_height)

    def rect_bold(self, rect_id):
        err = self.lib.set_bold_flag(self.rectmanager, rect_id)
        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_invert(self, rect_id):
        err = self.lib.set_invert_flag(self.rectmanager, rect_id)
        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_unset_invert(self, rect_id):
        err = self.lib.unset_invert_flag(self.rectmanager, rect_id)
        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_underline(self, rect_id):
        err = self.lib.set_underline_flag(self.rectmanager, rect_id)
        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_unset_underline(self, rect_id):
        err = self.lib.unset_underline_flag(self.rectmanager, rect_id)
        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_unset_bold(self, rect_id):
        err = self.lib.unset_bold_flag(self.rectmanager, rect_id)
        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_attach(self, rect_id, parent_id, position=(0,0)):
        self.lib.attach(self.rectmanager, rect_id, parent_id)
        err = 0

        # No need to move the rect to 0,0. That is the default position.
        if position != (0,0):
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

    def rect_clear_children(self, rect_id):
        err = self.lib.clear_children(self.rectmanager, rect_id)

        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_clear_characters(self, rect_id):
        err = self.lib.clear_characters(self.rectmanager, rect_id)

        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )

    def rect_disable(self, rect_id):
        err = self.lib.disable_rect(self.rectmanager, rect_id)

        if err:
            raise EXCEPTIONS[err]( rect_id=rect_id )


    def rect_enable(self, rect_id):
        err = self.lib.enable_rect(self.rectmanager, rect_id)

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
        if 'width' not in kwargs.keys():
            kwargs['width'] = 1

        if 'height' not in kwargs.keys():
            kwargs['height'] = 1

        if 'parent' not in kwargs.keys():
            kwargs['parent'] = 0

        constructor = Rect
        if 'constructor' in kwargs.keys():
            constructor = kwargs['constructor']

        new_rect_id = self.lib.new_rect(self.rectmanager, kwargs['parent'], kwargs['width'], kwargs['height'])

        return constructor(new_rect_id, self, **kwargs)


    def rect_draw(self, rect_id):
        err = self.lib.render(self.rectmanager, rect_id)


        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id,
            )

    def rect_remove(self, rect_id):
        err = self.lib.delete_rect(self.rectmanager, rect_id)

        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id
            )

    def rect_set_transparency(self, rect_id, transparency):
        err = self.lib.set_transparency(self.rectmanager, rect_id, transparency)

        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id
            )

    def rect_shift_contents(self, rect_id, x, y):
        err = self.lib.shift_contents(self.rectmanager, rect_id, x, y)

        if err:
            raise EXCEPTIONS[err](
                rect_id=rect_id,
                x=x,
                y=y
            )

    def kill(self):
        rects = []
        for rect_id, rect in self.root.rects.items():
            rects.append(rect)

        while rects:
            rects.pop().detach()

        self.lib.kill(self.rectmanager)

    def render(self):
        self.rect_draw(0)

    def fit_to_terminal(self):
        return self.lib.fit_to_terminal(self.rectmanager)


__RECTMANAGER = None
def init():
    global __RECTMANAGER
    if not __RECTMANAGER:
        __RECTMANAGER = RectManager()
    return __RECTMANAGER.root

def fit_to_terminal():
    global __RECTMANAGER
    output = False
    if __RECTMANAGER:
        output = __RECTMANAGER.fit_to_terminal()
    return output

def kill():
    global __RECTMANAGER
    if __RECTMANAGER:
        __RECTMANAGER.kill()


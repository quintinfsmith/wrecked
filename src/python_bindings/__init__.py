from cffi import FFI
from ctypes import c_bool
import sys
import tty, termios
import os, time
import json
import threading
import logging

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

class RectLogger:
    def __init__(self, logger, log_level=logging.INFO):
        self.logger = logger
        self.log_level = log_level

    def write(self, buf):
        for line in buf.rstrip().splitlines():
            self.logger.log(self.log_level, line.rstrip())

    def flush(self):
        # TODO: Figure out what is needed here
        pass


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

    def __init__(self, n, rectmanager, **kwargs):
        self.rectmanager  = rectmanager
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

    #def fill(self, character):
    #    self.rectmanager.rect_fill(self.rect_id, character)

    def enable(self):
        self.enabled = True
        self.rectmanager.rect_enable(self.rect_id);

    def disable(self):
        self.enabled = False
        self.rectmanager.rect_disable(self.rect_id);

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

    def empty(self):
        self.rectmanager.rect_empty(self.rect_id)

    def clear(self):
        self.rectmanager.rect_clear(self.rect_id)

    def new_rect(self, **kwargs):
        kwargs['parent'] = self.rect_id
        rect = self.rectmanager.create_rect(**kwargs)
        self.rects[rect.rect_id] = rect
        rect.parent = self

        return rect

    def shift_contents(self, x, y):
        self.rectmanager.rect_shift_contents(self.rect_id, x, y)


class RectManager:
    SO_PATH = "/usr/lib/libwrecked_bindings.so"

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

            uint32_t empty(RectManager, uint32_t);
            uint32_t clear(RectManager, uint32_t);


            uint32_t unset_color(RectManager, uint32_t);
            uint32_t set_bg_color(RectManager, uint32_t, uint8_t);
            uint32_t set_fg_color(RectManager, uint32_t, uint8_t);
            uint32_t unset_bg_color(RectManager, uint32_t);
            uint32_t unset_fg_color(RectManager, uint32_t);

            void set_bold_flag(RectManager, uint32_t);
            void unset_bold_flag(RectManager, uint32_t);
            void set_underline_flag(RectManager, uint32_t);
            void unset_underline_flag(RectManager, uint32_t);
            void set_invert_flag(RectManager, uint32_t);
            void unset_invert_flag(RectManager, uint32_t);

            uint32_t disable_rect(RectManager, uint32_t);
            uint32_t enable_rect(RectManager, uint32_t);

            uint32_t set_character(RectManager, uint32_t, uint32_t, uint32_t, const char*);
            uint32_t set_string(RectManager, uint32_t, uint32_t, uint32_t, const char*);
            uint32_t unset_character(RectManager, uint32_t, uint32_t, uint32_t);

            uint32_t draw(RectManager, uint32_t);

            uint32_t replace_with(RectManager, uint32_t, uint32_t);

            uint32_t shift_contents(RectManager, uint32_t, int32_t, int32_t);

        """)

        sl = RectLogger(logging.getLogger('STDOUT'), logging.INFO)
        sys.stdout = sl

        sl = RectLogger(logging.getLogger('STDERR'), logging.ERROR)
        sys.stderr = sl
        self.log_path = '.wreckederr.log'
        if os.path.isfile(self.log_path):
            with open(self.log_path, 'w') as fp:
                fp.write("")

        logging.basicConfig(
           level=logging.DEBUG,
           format='%(message)s',
           filename=self.log_path,
           filemode='a'
        )

        self.lib = ffi.dlopen(self.SO_PATH)

        # TODO: get rectmanager's root node size from  cffi layer
        self.width, self.height = get_terminal_size()
        self.rectmanager = self.lib.init(self.width, self.height)

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
        err = self.lib.draw(self.rectmanager, rect_id)


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

    def draw(self):
        self.rect_draw(0)


class RectStage(RectManager):
    FPS = 60
    DELAY = 1 / FPS
    def __init__(self):
        super().__init__()
        self.scenes = {}
        self.active_scene = None
        self.playing = False

    def set_fps(self, new_fps):
        self.FPS = new_fps
        self.DELAY = 1 / self.FPS

    def play(self):
        self.play_thread = threading.Thread(
            target=self._play
        )
        self.play_thread.start()

    def _resize_checker(self):
        w, h = get_terminal_size()

        if self.width != w or self.height != h:
            self.resize(w, h)
            try:
                scene = self.scenes[self.active_scene]
            except KeyError:
                scene = None

            if scene:
                scene.resize(w, h)

    def _play(self):
        self.playing = True
        while self.playing:
            self._resize_checker()

            try:
                scene = self.scenes[self.active_scene]
            except KeyError:
                scene = None
            if scene:
                try:
                    scene.tick()
                except Exception as e:
                    self.kill()
                    raise e
            time.sleep(self.DELAY)

    # TODO: Handle Errors here
    def create_scene(self, key, constructor, **kwargs):
        kwargs['width'] = self.width
        kwargs['height'] = self.height
        kwargs['constructor'] = constructor

        output = self.create_rect(**kwargs)
        self.scenes[key] = output

        return output

    def start_scene(self, new_scene_key):
        if self.active_scene:
            self.scenes[self.active_scene].disable()
        self.active_scene = new_scene_key
        self.scenes[self.active_scene].enable()
        self.draw()

    def kill(self):
        self.playing = False
        super().kill()


class RectScene(Rect):
    def __init__(self, n, rectmanager, **kwargs):
        super().__init__(n, rectmanager, **kwargs)

    def tick(self):
        pass


if __name__ == "__main__":
    import time, math, threading
    stage = RectStage()
    class TestScene(RectScene):
        def __init__(self, n, rectmanager, **kwargs):
            super().__init__(n, rectmanager, **kwargs)
            self.limit = 60
            self.p = 0
            self.done = False

        def tick(self):
            if not self.done:
                self.p += 1

                if self.p == self.limit // 2:
                    raise KeyError()
                elif self.p == self.limit:
                    self.done = True

                self.set_bg_color(int(self.p * 8 / self.limit))
            self.draw()

    stage.play()
    scene = stage.create_scene(0, TestScene)
    scene.set_string(0, 0, 'Some Test Text')
    stage.start_scene(0)
    while not scene.done and stage.playing:
        time.sleep(.1)
    stage.kill()


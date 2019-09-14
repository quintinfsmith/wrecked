import sys
from localfuncs import get_terminal_size
from poc import Rect

class BleepsScreen:
    def __init__(self):
        self.rect = Rect()

        self.box_cache = {0: self}

        self.width, self.height = get_terminal_size()
        self.rect.resize(self.width, self.height)
        print(self.width, self.height)

    def box_flag_cache(self, box_id):
        self.rect.flag_full_refresh = True


    def box_attach(self, box_id, parent_id, position=(0,0)):
        self.box_cache[parent_id].rect.add_child(self.box_cache[box_id])

        if position != (0,0):
            self.box_move(box_id, *position)


    def box_detach(self, box_id):
        self.box_cache[box_id].rect.detach()

    def box_disable(self, box_id):
        pass

    def box_enable(self, box_id):
        pass

    def box_setc(self, box_id, x, y, character):
        self.box_cache[box_id].rect.set_character(x, y, character)

    def box_fillc(self, box_id, character):
        self.box_cache[box_id].rect.default_character = character

    def box_unsetc(self, box_id, x, y):
        self.box_cache[box_id].rect.unset_character(x, y)

    def box_unset_bg_color(self, box_id):
        pass

    def box_unset_fg_color(self, box_id):
        pass

    def box_unset_color(self, box_id):
        pass

    def box_set_bg_color(self, box_id, color):
        pass

    def box_set_fg_color(self, box_id, color):
        pass

    def box_move(self, box_id, x, y):
        self.box_cache[box_id].rect.move(x, y)

    def box_resize(self, box_id, width, height):
        self.box_cache[box_id].rect.resize(width, height)

    def new_box(self, **kwargs):
        width = 1
        if 'width' in kwargs.keys():
            width = kwargs['width']

        height = 1
        if 'height' in kwargs.keys():
            height = kwargs['height']

        new_rect = Rect()
        new_bleepsbox = BleepsBox(new_rect.rect_id, self, **kwargs)
        new_bleepsbox.rect = new_rect

        self.box_cache[new_rect.rect_id] = new_bleepsbox
        new_rect.resize(width, height)

        if 'parent' in kwargs.keys():
            parent = kwargs['parent']
            self.box_cache[parent].rect.add_child(new_rect)

        return new_bleepsbox


    def box_draw(self, box_id):
        rect = self.box_cache[box_id].rect
        offset = rect.get_offset()
        for (x, y), character in rect.get_display().items():
            sys.stdout.write("\033[%d;%dH%s\n" % (y + 1, x + 1, character))


    #def box_draw_area(self, box_id, x, y, width, height):
    #    rect = self.box_cache[box_id].rect
    #    offset = rect.get_offset()
    #    for (xb, yb), character in rect.get_display(boundries=(x, y, x + width, y + height)).items():
    #        sys.stdout.write("\033[%d;%dH%s" % (yb + 1, xb + 1, character))


    def box_remove(self, box_id):
        self.box_detach(box_id)

    def draw(self):
        self.box_draw(self.rect.rect_id)

    def kill(self):
        pass



class BleepsBox:
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
        self.rect = None
        self.bleeps_id = n
        self.boxes = {}
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
        self._screen.box_detach(self.bleeps_id)

    def fill(self, character):
        self._screen.box_fillc(self.bleeps_id, character)

    def enable(self):
        self.enabled = True
        self._screen.box_enable(self.bleeps_id);

    def disable(self):
        self.enabled = False
        self._screen.box_disable(self.bleeps_id);

    def draw_area(self, box_id, x, y, width, height):
        self._screen.box_draw_area(self.bleeps_id, x, y, width, height)

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
        self.x = new_x
        self.y = new_y
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
    #box.draw()
    # TODO: Why doesn't this work?
    screen.draw()

    time.sleep(2)
    screen.kill()


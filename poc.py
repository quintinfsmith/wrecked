import sys
# NOTES
# * DISALLOW positional overflow, ie no children < 0 or > width/height
# * No invisible Rects with visible children


class Rect(object):
    rect_id = 0
    width = 0
    height = 0

    default_character = ' '

    parent = None # Rect

    children = {}
    child_space = {} # { (x, y): [child id stack] }
    _inverse_child_space = {} # { child id: [(x,y)..] }
    child_positions = {} # { child id: topleft corner }

    character_space = {} # { (x, y): character }

    flag_full_refresh = False
    flags_pos_refresh = set() # [(x, y) ... ]

    # cache
    _cached_display = {}
    def __init__(self):
        self.rect_id = Rect.rect_id
        Rect.rect_id += 1

        self.parent = None

        self.width = 0
        self.height = 0

        self.children = {}
        self.child_space = {}
        self.child_positions = {}
        self._inverse_child_space = {}
        self.color = 0

        self.character_space = {}

        self.flag_full_refresh = True
        self.flags_pos_refresh = set()

        self._cached_display = {}

    def unset_bg_color(self):
        original_color = self.color
        self.color &= 0b1111111111100000

        if self.color != original_color:
            self.flag_full_refresh = True

    def unset_bg_color(self):
        original_color = self.color
        self.color &= 0b1111110000011111

        if self.color != original_color:
            self.flag_full_refresh = True

    def unset_color(self):
        original_color = self.color
        self.color &= 0

        if self.color != original_color:
            self.flag_full_refresh = True


    def set_bg_color(self, new_color):
        if new_color > 15: # Not a usable color
            return

        original_color = self.color
        #Reduce new color to 5 bits
        new_color &= 0b01111
        new_color |= 0b10000

        # clear original color
        self.color &= 0b1111111111100000

        # apply new color
        self.color |= new_color

        if self.color != original_color:
            self.flag_full_refresh = True

    def set_fg_color(self, new_color):
        if new_color > 15: # Not a usable color
            return

        original_color = self.color
        #Reduce new color to 5 bits
        new_color &= 0b01111
        new_color |= 0b10000

        # clear original color
        self.color &= 0b1111110000011111

        # apply new color
        self.color |= (new_color << 5)

        if self.color != original_color:
            self.flag_full_refresh = True


    def add_child(self, child):
        self.children[child.rect_id] = child
        self._inverse_child_space[child.rect_id] = []
        self.set_child_position(child.rect_id, 0, 0)

        child.parent = self

    def detach(self):
        self.parent.detach_child(self.rect_id)

    def detach_child(self, child_id):
        child = self.children[child_id]
        self.clear_child_space(child_id)
        del self.child_positions[child_id]
        del self.children[child_id]
        return child


    def resize(self, width, height):
        self.width = width
        self.height = height
        if self.parent:
            x, y = self.parent.child_positions[self.rect_id]
            self.parent.update_child_space(self.rect_id, (x, y, x + width, y + height))


    def move(self, x, y):
        self.parent.set_child_position(self.rect_id, x, y)


    def set_child_position(self, child_id, x, y):
        child = self.children[child_id]
        self.child_positions[child_id] = (x, y)
        self.update_child_space(child_id, (x, y, x + child.width, y + child.height))


    # SPOT REFRESH
    def clear_child_space(self, child_id):
        for position in self._inverse_child_space[child_id]:
            self.child_space[position].remove(child_id)
            self.flags_pos_refresh.add(position)

        self._inverse_child_space[child_id] = set()


    # SPOT REFRESH
    def update_child_space(self, child_id, corners):
        self.clear_child_space(child_id)

        for y in range(corners[1], corners[3]):
            for x in range(corners[0], corners[2]):
                if (x, y) not in self.child_space.keys():
                    self.child_space[(x, y)] = []

                self.child_space[(x, y)].append(child_id)
                self._inverse_child_space[child_id].add((x, y))

                self.flags_pos_refresh.add((x, y))


    # SPOT REFRESH
    def set_character(self, x, y, character):
        self.character_space[(x, y)] = character
        self.flags_pos_refresh.add((x, y))

    def unset_character(self, x, y):
        del self.character_space[(x, y)]
        self.flags_pos_refresh.add((x, y))


    #def get_character(self, x, y):
    #    return self.character_space[(x, y)]


    def _update_cached_display(self, **kwargs):
        '''
            Will Never update outside of 0 - width or 0 - height
        '''

        # If full refresh is requested, fill flags_pos_refresh with all potential coords
        if self.flag_full_refresh:
            self.flag_full_refresh = False
            self.flags_pos_refresh = set()
            for y in range(self.height):
                for x in range(self.width):
                    self.flags_pos_refresh.add((x, y))


        # Iterate through flags_pos_refresh and update any children that cover the requested positions
        # Otherwise set _cached_display
        child_recache = {}
        for (x, y) in self.flags_pos_refresh:
            if (x, y) not in self.child_space.keys() or not self.child_space[(x, y)]:
                if (x, y) not in self.character_space.keys():
                    self.character_space[(x, y)] = self.default_character

                self._cached_display[(x, y)] = (self.character_space[(x, y)], self.color)
            else:
                child_id = self.child_space[(x, y)][-1]
                if child_id not in child_recache.keys():
                    child_recache[child_id] = []
                child_recache[child_id].append((x, y))


        for child_id, coords in child_recache.items():
            childx, childy = self.child_positions[child_id]
            child = self.children[child_id]
            child._update_cached_display()

            for (x, y) in coords:
                self._cached_display[(x, y)] = child._cached_display[(x - childx, y - childy)]

        self.flags_pos_refresh = set()



    def get_display(self, **kwargs):
        boundries = (0, 0, self.width, self.height)
        if "boundries" in kwargs.keys():
            boundries = kwargs['boundries']

        offset = (0, 0)
        if "offset" in kwargs.keys():
            offset = kwargs['offset']

        original_cache = self._cached_display.copy()
        self._update_cached_display()

        output = {}
        for (x, y), new_c in self._cached_display.items():
            if not ((x >= boundries[0] and x < boundries[2]) and (y >= boundries[1] and y < boundries[3])):
                continue

            try:
                if original_cache[(x, y)] != new_c:
                    output[(x, y)] = new_c
            except KeyError:
                output[(x,y)] = new_c

        return output


    def get_offset(self):
        offset = (0, 0)
        if self.parent:
            parent_offset = self.parent.get_offset()
            offset = (offset[0] + parent_offset[0], offset[1] + parent_offset[0])
        return offset

    def draw(self):
        offset = self.get_offset()
        for (x, y), (character, color) in self.get_display().items():
            if (color):
                # ForeGround
                if (color >> 5) & 16 == 16:
                    if (color >> 5) & 8 == 8:
                        sys.stdout.write("\033[9%dm\n" % ((color >> 5) & 7))
                    else:
                        sys.stdout.write("\033[3%dm\n" % ((color >> 5) & 7))
                else:
                    sys.stdout.write("\033[39m\n")

                # BackGround
                if (color & 16 == 16):
                    if color & 8 == 8:
                        sys.stdout.write("\033[10%dm\n" % (color & 7))
                    else:
                        sys.stdout.write("\033[4%dm\n" % (color & 7))
                else:
                    sys.stdout.write("\033[49m\n")


            sys.stdout.write("\033[%d;%dH%s\n" % (y + 1, x + 1, character))


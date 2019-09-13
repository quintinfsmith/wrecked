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

    character_space = { } # { (x, y): character }

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

        self.character_space = {}

        self.flag_full_refresh = True
        self.flags_pos_refresh = set()

        self._cached_display = {}

    def add_child(self, child):
        self.children[child.rect_id] = child
        self._inverse_child_space[child.rect_id] = []
        self.set_child_position(child.rect_id, 0, 0)

        child.parent = self

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
                if (x,y) not in self.child_space.keys():
                    self.child_space[(x, y)] = []

                self.child_space[(x, y)].append(child_id)
                self._inverse_child_space[child_id].add((x, y))

                self.flags_pos_refresh.add((x, y))


    # SPOT REFRESH
    def set_character(self, x, y, character):
        self.character_space[(x, y)] = character
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

                self._cached_display[(x, y)] = self.character_space[(x, y)]
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

        self._update_cached_display()

        output = {}
        for y in range(boundries[0], boundries[2]):
            for x in range(boundries[1], boundries[3]):
                output[(x, y)] = self._cached_display[(x, y)]

        return output

if __name__ == "__main__":
    import sys
    mainbox = Rect()
    subbox = Rect()
    subsubbox = Rect()
    mainbox.add_child(subbox)
    subbox.add_child(subsubbox)

    mainbox.resize(20, 20)
    subbox.resize(10, 10)
    for y in range(subbox.height):
        for x in range(subbox.width):
            subbox.set_character(x, y, 'X')

    subsubbox.resize(10, 10)
    subsubbox.move(5, 5)
    for y in range(subsubbox.height):
        for x in range(subsubbox.width):
            subsubbox.set_character(x, y, '.')

    for (x, y), c in mainbox.get_display().items():
        sys.stdout.write("\033[%d;%dH%s" % (x + 1, y + 1, c))





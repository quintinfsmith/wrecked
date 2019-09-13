# NOTES
# * DISALLOW positional overflow, ie no children < 0 or > width/height
# * No invisible Rects with visible children


class Rect(object):
    rect_id = 0
    width = 0
    height = 0

    parent = None # Rect

    children = {}
    child_space = {} # { (x, y): [child id stack] }
    child_positions = {} # { child id: topleft corner }

    character_space = { } # { (x, y): character }

    flag_full_refresh = False
    flags_pos_refresh = set() # [(x, y) ... ]

    # cache
    _cached_display = {}

    def add_children(self, child): pass
    def resize(self): pass

    def move(self, x, y):
        parent.set_child_position(self.rect_id, x, y)

    def set_child_position(self, child_id, x, y): pass

    def set(self, x, y, character): pass
    def get(self, x, y): pass

    def update_cached_display(self, **kwargs):
        '''
            Will Never update outside of 0 - width or 0 - height
        '''
        child_recache = {}

        # If full refresh is requested, fill flags_pos_refresh with all potential coords
        if self.flag_full_refresh:
            self.flags_pos_refresh = set()
            for y in range(self.height):
                for x in range(self.width):
                    self.flags_pos_refresh((x, y))

        # Iterate through flags_pos_refresh and update any children that cover the requested positions
        # Otherwise set _cached_display
        for (x, y) in self.flags_pos_refresh:
            if (x, y) not in self.child_space.keys():
                self._cached_display[(x, y)] = self.character_space[(x, y)]
            else:
                child_id = self.child_space[(x, y)]
                if child_id not in child_recache.keys():
                    child_recache[child_id] = []
                child_recache[child_id].append((x, y))


        for child_id, coords in child_recache.items():
            childx, childy = self.child_positions[child_id]
            child = self.children[child_id]
            child.update_cached_display()

            for (x, y) in coords:
                self._cached_display[(x, y)] = child.get(childx - x, childy - y)

        self.flags_pos_refresh = set()


    def get_display(self, **kwargs):
        boundries = (0, 0, self.width, self.height)
        if "boundries" in kwargs.keys():
            boundries = kwargs['boundries']

        offset = (0, 0)
        if "offset" in kwargs.keys():
            offset = kwargs['offset']

        # TODO: Update cached_display

        output = {}
        for (x, y) in self._cached_display:
            pass



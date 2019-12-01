import sys
# NOTES
# * DISALLOW rendering positional overflow, ie no children < 0 or > width/height
# * No invisible Rects with visible children

class RectManager:

    def __init__(self):
        self.rects = {}
        self._id_gen = 0

    def get_new_id(self):
        new_id = self._id_gen
        self._id_gen += 1
        return new_id


class Rect(object):
    width = 0
    height = 0

    default_character = ' '
    _rect_manager = RectManager() # singleton
    parent = None # Rect

    children = {}
    child_space = {} # { (x, y): [child id stack] }
    _inverse_child_space = {} # { child id: [(x,y)..] }
    child_positions = {} # { child id: topleft corner }

    character_space = {} # { (x, y): character }

    flag_full_refresh = False
    flags_pos_refresh = set() # [(x, y) ... ]

    enabled = True

    has_been_drawn = False

    # cache
    _cached_display = {}
    def __init__(self):
        self.rect_id = self._rect_manager.get_new_id()
        self._rect_manager.rects[self.rect_id] = self

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

        self.child_ghosts = {}


    def enable(self):
        was_enabled = self.enabled
        self.enabled = True
        if self.parent and not was_enabled:
            x, y = self.get_offset()
            self.parent.set_child_space(self.rect_id, x, y, x + self.width, y + self.height)


    def disable(self):
        was_enabled = self.enabled

        self.enabled = False
        if self.parent and was_enabled:
            self.parent.clear_child_space(self.rect_id)


    def unset_bg_color(self):
        original_color = self.color
        self.color &= 0b1111111111100000

        if self.color != original_color:
            self.flag_full_refresh = True


    def unset_fg_color(self):
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
            self.parent.set_child_position(self.rect_id, x, y)

    def move(self, x, y):
        # TODO: Throw Error if no parent
        self.parent.set_child_position(self.rect_id, x, y)


    def set_child_position(self, child_id, x, y):
        child = self.children[child_id]
        self.child_positions[child_id] = (x, y)
        self.update_child_space(child_id, (x, y, x + child.width, y + child.height))


    # SPOT REFRESH
    def clear_child_space(self, child_id):
        for position in self._inverse_child_space[child_id]:
            self.child_space[position].remove(child_id)
            self.set_precise_refresh_flag(*position)

            if child_id not in self.child_ghosts.keys():
                self.child_ghosts[child_id] = set()
            self.child_ghosts[child_id].add(position)

        self._inverse_child_space[child_id] = set()


    # SPOT REFRESH
    def update_child_space(self, child_id, corners):
        self.clear_child_space(child_id)

        for y in range(corners[1], corners[3]):
            for x in range(corners[0], corners[2]):
                if x >= 0 and x < self.width and y >= 0 and y < self.height:
                    if (x, y) not in self.child_space.keys():
                        self.child_space[(x, y)] = []

                    self.child_space[(x, y)].append(child_id)
                    self._inverse_child_space[child_id].add((x, y))

                    try:
                        self.child_ghosts[child_id].remove((x, y))
                    except KeyError:
                        # Not a ghost.
                        pass

                    self.set_precise_refresh_flag(x, y)


    # SPOT REFRESH
    def set_character(self, x, y, character):
        self.character_space[(x, y)] = character
        self.set_precise_refresh_flag(x, y)

    def unset_character(self, x, y):
        self.character_space[(x, y)] = self.default_character
        self.set_precise_refresh_flag(x, y)


    def _update_cached_by_positions(self, positions, boundries):
        child_recache = {}
        i = 0
        positions = list(positions)

        while i < len(positions):
            (x, y) = positions[i]
            if not (x >= boundries[0] and x < boundries[2] and y >= boundries[1] and y < boundries[3]):
                i += 1
                continue
            else:
                positions.pop(i)


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

            new_boundries = [
                boundries[0] - childx,
                boundries[1] - childy,
                boundries[2] - childx,
                boundries[3] - childy
            ]

            child._update_cached_display(new_boundries)

            for (x, y) in coords:
                if childx > x and childy > y and x <= child.width and y <= child.height:
                    continue

                if x >= 0 and x < self.width and y >= 0 and y < self.height:
                    self._cached_display[(x, y)] = child._cached_display[(x - childx, y - childy)]

        self.flags_pos_refresh = set(positions)


    def _update_cached_display(self, boundries):
        '''
            Will Never update outside of 0 - width or 0 - height
        '''

        # Since Children indicate to parents that a refresh is requested,
        # if no flag is set, there is no need to delve down
        if not (self.flags_pos_refresh or self.flag_full_refresh):
            return

        self.has_been_drawn = True


        # If full refresh is requested, fill flags_pos_refresh with all potential coords
        if self.flag_full_refresh:
            self.flag_full_refresh = False
            self.flags_pos_refresh = set()
            for y in range(self.height):
                for x in range(self.width):
                    self.flags_pos_refresh.add((x, y))

        # Iterate through flags_pos_refresh and update any children that cover the requested positions
        # Otherwise set _cached_display
        positions_to_refresh = self.flags_pos_refresh.copy()
        self._update_cached_by_positions(positions_to_refresh, boundries)



    def flag_refresh(self):
        self.flag_full_refresh = True

    def set_precise_refresh_flag(self, x, y):
        self.flags_pos_refresh.add((x, y))
        if self.parent:
            offset = self.parent.child_positions[self.rect_id]
            self.parent.set_precise_refresh_flag(offset[0] + x, offset[1] + y)

    def get_display(self, **kwargs):
        boundries = (0, 0, self.width, self.height)
        if "boundries" in kwargs.keys():
            boundries = kwargs['boundries']

        offset = (0, 0)
        if "offset" in kwargs.keys():
            offset = kwargs['offset']


        self._update_cached_display(boundries)

        output = {}
        for (x, y), new_c in self._cached_display.items():
            if not (x >= boundries[0] and x < boundries[2] and y >= boundries[1] and y < boundries[3]):
                continue
            output[(x,y)] = new_c

        # Ghosts
        if self.parent:
            self.parent.handle_ghosts(self.rect_id)
        return output

    def handle_ghosts(rect_id):
        ghosts = self.child_ghosts[rect_id]

        offx, offy = (0, 0)
        first_offx, first_offy = self.child_positions[rect_id]

        top = self
        while top.parent:
            x, y = top.parent.child_positions[top.rect_id]
            offx += x
            offy += y
            top = top.parent

        new_ghosts = set()
        for (x, y) in ghosts:
            new_ghosts.add(
                (
                    x + offx,
                    y + offy
                )
            )

        top._update_cached_by_positions(new_ghosts, [0, 0, top.width, top.height])

        for (x, y) in new_ghosts:
            ghostpos = (x - first_offx, y - first_offy)
            if ghostpos[0] >= 0 and ghostpos[1] >= 0 and ghostpos[0] < top.width and ghostpos[1] < top.height:
                output[ghostpos] = top._cached_display[(x, y)]

        self.child_ghosts[rect_id] = set()



    def get_offset(self):
        '''
            Recursively find the absolute offset of this Rect
        '''

        if self.parent:
            offset = self.parent.child_positions[self.rect_id]
            parent_offset = self.parent.get_offset()
            offset = (
                offset[0] + parent_offset[0],
                offset[1] + parent_offset[1]
            )
        else:
            offset = (0, 0)

        return offset

    def get_top(self):
        top = self
        while top.parent:
            top = top.parent

        return top

    def draw(self, **kwargs):

        offset = self.get_offset()

        top = self
        while top.parent:
            top = top.parent


        display_data = list(self.get_display(boundries=[0, 0, top.width, top.height]).items())
        display_data.sort()

        output = ""
        current_row = -1
        current_col = -1
        for (x, y), (character, color) in display_data:
            if not (y + offset[1] >= 0 and x + offset[0] >= 0 and y + offset[1] < top.height and x + offset[0] < top.width):
                continue

            if x != current_row or y != current_col:
                output += "\033[%d;%dH" % (y + offset[1] + 1, x + offset[0] + 1)
            current_row = x
            current_col = y

            if color:
                # ForeGround
                if (color >> 5) & 16 == 16:
                    if (color >> 5) & 8 == 8:
                        output += "\033[9%dm" % ((color >> 5) & 7)
                    else:
                        output += "\033[3%dm" % ((color >> 5) & 7)
                else:
                    output += "\033[39m"

                # BackGround
                if (color & 16 == 16):
                    if color & 8 == 8:
                        output += "\033[10%dm" % (color & 7)
                    else:
                        output += "\033[4%dm" % (color & 7)
                else:
                    output += "\033[49m"
            else:
                output += "\033[0m"


            output += "%s" % character
            current_row += 1

        if output:
            sys.stdout.write(output + "\033[0;0H\n")


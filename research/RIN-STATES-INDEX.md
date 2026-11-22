# RinRin native state tables

32 slots, each with branch/update/end callbacks. Tables at analysis addresses 0x81174800, 0x81174980, 0x81174b00; records are CodeWarrior member pointers `(thisDelta=0, virtualIndex=-1, function)`. Native onMove calls the first two tables at 0x810cad90/0x810cadb0. changeState calls the end table at 0x810cb20c. Native source offsets/relocations are in enemy-link.json.

- 0: `wait`
- 1: `run`
- 2: `atk_uraken`
- 3: `atk_tetuzankou`
- 4: `scaffold_jump`
- 5: `right_jump`
- 6: `left_jump`
- 7: `front_jump`
- 8: `atk_kamikaze`
- 9: `right_step`
- 10: `left_step`
- 11: `back_step`
- 12: `front_step`
- 13: `atk_bomb`
- 14: `atk_throw`
- 15: `bad_smell`
- 16: `dm_explosion`
- 17: `keen_compe`
- 18: `dm_scaffold_bomb`
- 19: `dm_giantswing`
- 20: `dm_electricshock`
- 21: `dm_right`
- 22: `dm_left`
- 23: `dm_body`
- 24: `provocation`
- 25: `scaffold_wait`
- 26: `finish`
- 27: `finish_wait`
- 28: `atk_giantslalom`
- 29: `guard`
- 30: `dm_down_damage`
- 31: `pl_dead`
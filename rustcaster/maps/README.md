# Maps

Every `.txt` file of this folder shows up in the level list ("Play"), sorted by name.
To add your own map, drop a `.txt` file here (e.g. `my_map.txt`) and reopen the level list.
`example.txt` is a small map using every tile: copy it and edit it to get started.

## Format

One character per tile:

| Char  | Tile                                   |
|-------|----------------------------------------|
| `P`   | player start (exactly one)             |
| `#`   | wall                                   |
| `X`   | see-through grid (blocks the player)   |
| `0`   | coin                                   |
| `E`   | exit (at least one)                    |
| space | floor                                  |

Rules:
- the map must be a rectangle: every line has the same length (pad with spaces);
- tiles outside the map behave like walls, so the border does not need to be closed.

Example (`example.txt`):

```
###########
#P #      #
#0 ##X #  #
# 0    #  #
#X#0 ######
# # 0000X #
# #   #0XE#
#######E###
```

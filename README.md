# Rust-Raycaster
Second try to do a raycaster in Rust this time

Using sdl2 for rust : https://docs.rs/sdl2/latest/sdl2/

Maps are in txt format

Player position     : 'P'
Walls               : '#'
Grids               : 'X'
Coins               : '0'
Monsters            : 'M'
Ends                : 'E'

There is a map example :
```
###########
#P #      #
#0 ##X #  #
# 0    # M#
#X#0 ######
# # 0000X #
# #   #0XE#
#######E###
```
The map must be a rectangle

for example lines jumps are represented as '$'
```
 Correct:         Incorrect: 

 ####  $           ####$
 #   P $           #    P$
 #     $           #$
 ##   #$           ##    #$
 #E   #$           #E    #$
 ######$           #######$
```
# Rust-Raycaster

This project use Rust and sdl2 for rust : https://docs.rs/sdl2/latest/sdl2/

## Build & run (Windows, only Docker needed)

The game is cross-compiled to a Windows `.exe` inside Docker, then launched natively:
```
.\run.ps1                  # builds then runs maps\data.txt
.\run.ps1 maps\map1.txt    # another map (path relative to rustcaster/)
.\run.ps1 -Build           # build only
```

## Maps

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

 ####   $           ####$
 #    P $           #    P$
 #      $           #$
 ##    #$           ##    #$
 #E    #$           #E    #$
 #######$           #######$
```

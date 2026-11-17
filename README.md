# Rust-Raycaster

This project use Rust and sdl2 for rust : https://docs.rs/sdl2/latest/sdl2/

## Build & run (Windows, only Docker needed)

The game is cross-compiled to a Windows `.exe` inside Docker, then launched natively:
```
.\run.ps1                  # builds then opens the menu (pick a map / preset)
.\run.ps1 maps\map1.txt    # plays a map directly (path relative to rustcaster/)
.\run.ps1 -Build           # build only
```

## Controls

W / S (or Up / Down) move, A / D (or Left / Right) turn, Escape pauses
(FOV, sensitivity, preset and minimap are set from the pause menu).

Code layout: see [ARCHITECTURE.md](ARCHITECTURE.md).

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

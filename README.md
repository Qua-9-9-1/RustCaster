# Rust-Raycaster

This project use Rust and sdl2 for rust : https://docs.rs/sdl2/latest/sdl2/

## Build & run (Windows, only Docker needed)

The game is cross-compiled to a Windows `.exe` inside Docker, then launched natively:
```
.\run.ps1                  # builds then opens the menu (pick a map / preset)
.\run.ps1 maps\map1.txt    # plays a map directly (path relative to rustcaster/)
.\run.ps1 -Build           # build only
.\run.ps1 -Package         # release build zipped into rustcaster\dist\rustcaster-windows-x64.zip
```

## Releases

Pushing a tag `v*` (e.g. `git tag v0.1.0; git push origin v0.1.0`) runs the GitHub Actions
workflow `.github/workflows/release.yml`, which publishes a release with:
- `rustcaster-windows-x64.zip`: the game, `SDL2.dll`, assets and maps, ready to play.
- `rustcaster-linux-x64.tar.gz`: needs SDL2 and GTK 3 installed (`sudo apt install libsdl2-2.0-0 libgtk-3-0`).

Running the workflow manually ("Run workflow") only builds and keeps the archives as workflow artifacts.

## Controls

In game: W / S (or Up / Down) move, A / D (or Left / Right) turn, Escape (or the
top-right pause button) opens the pause menu. The map button next to it toggles the minimap.

Menus work with the mouse, or with the arrows + Enter, Escape going back:
- Title screen: Play, Create preset, Settings, Quit.
- Level select: pick a map in the list (wheel / Up / Down) and a style with `<` `>`
  (Left / Right), then Start (Enter).
- Pause: Resume, Settings, style (applied immediately), Restart map, Main menu, Quit.
- Settings (title screen or pause): field of view, turn speed, minimap.
- Reaching an exit shows the score, coins and time with Next map / Replay / Menu.

Code layout: see [ARCHITECTURE.md](ARCHITECTURE.md).

## Custom presets

From the menu, open the preset editor: give your preset a name, pick a base preset ("Start from"),
then import your own images (PNG, JPG, BMP, GIF, TGA) for the wall, floor, ceiling, grid, coin,
end flag and sky. Images are scaled down to 256 px; grid, coin and end keep their transparency.
Click a card to tweak its flat colour, toggle Outside / Fog, set the fog strength (`-` / `+`),
turn the sun and clouds on or off ("Sky extras" card), check the live preview and Save.

User presets are saved in `rustcaster/presets/<name>/` (PNG files + `preset.json`) and appear in the
preset list. Built-in presets (`rustcaster/assets/presets/`) are never overwritten; your own presets can
be edited again or deleted from the editor.

## Maps

Maps are in txt format: every `.txt` file of `rustcaster/maps/` (the `maps/` folder next to the
executable in the release archives) appears in the level list. See `maps/README.md` and
`maps/example.txt`.

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

# pxls-placemaper
Placemap for pxls.space.

Features:
```
1. User Stats:
Pixels
Survivors
Undo
Pixel (th)

2. Placemap:
Actual Pixel
Undo Pixel
Survived Pixel
```
Coming Soon:
```
1. Self replaced pixel count
2. Times of color used + Top
```
Futures (Maybe):
```
1. Animation for each pixels you place + delay per frame

Separate
Global Stats + Some no meaning info
1. Most Active Pixel
2. ???
```
Instruction

1. Open settings.ron
2. Change following value

```
Settings(
    user_key: "",      <--- https://pxls.space/profile?action=data
    canvas_code: 71,   <--- The canvas
    name: "Name"       <--- Any name
    pix_th: [100]      <--- Any number
)
```

File name in "input" folder
```
Ex: Canvas Code = 71
LOG: pixels_c71.sanit.log.tar.xz
IMAGE: Canvas_71_Initial.png
PALETTE: palette_c71.txt | Got From Clueless => /palette => Paint.Net
```
After finished process, 4 items added in "output" folder, Placemap (survivor & pixels placed & undo) and Stats.

Time To Finish ~40s (Outdated)

Simple as this, for now... maybe little too much boiler code.

No optimize done.

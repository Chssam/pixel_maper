# pxls-placemaper
Placemap for pxls.space.

Warn: Survivor pixels is not accurate, It will not count as survived even thought is undo from other placer.

It will be less than actual survived pixels.

Simple as this, for now... maybe little too much boiler code.

No optimize done.

Features:
```
1. User Stats:
Pixels
Survivors
Undo
Pixel (th)

2. Placemap Without Undo
```
Futures (Maybe):
```
1. Animation for each pixels you place + delay per frame
2. Placemap With Only Undo
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
After finished process, 3 items added in "output" folder, Placemap (survivor & pixels placed) and Stats.

Time To Finish ~40s

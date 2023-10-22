# pxls-placemaper
Placemap for pxls.space.

Next week:
+ Reproduce json palette look up (Currently using Clueless's palette)

Instruction
```
Settings(
    user_key: "",      <--- https://pxls.space/profile?action=data
    canvas_code: 71,   <--- The canvas
    name: "Name"       <--- Any name
    pix_th: [100]      <--- Any number
)
```

Input file items can be found on:

+ [Canvas](https://wiki.pxls.space/index.php?title=Category:Canvases)
+ [Logs](https://pxls.space/extra/logs/)
+ Clueless (The Discord Bot) with commands "/palette"

The main folder should look like this

<https://github.com/Chssam/pixel_maper/blob/main/sources/pxls-placemaper%20outlook.png>

Cover
- V0.1.1 - Use txt as palette
- V0.2.0 - Use json as palette (Which include name)
- V0.2.0 slower 20s than V0.1.1

Maybe too much boiler code.

No optimize done.

Futures (Maybe):
```
1. Animation for each pixels you place + delay per frame

Separate
Global Stats + Some no meaning info
1. Most Active Pixel
2. ???
```

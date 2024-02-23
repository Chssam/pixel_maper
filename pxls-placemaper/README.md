# pxls-placemaper
Placemap for [pxls.space](https://pxls.space/).

If you haven't already try Rust. Install Rust and Cargo.

1. Find a place for your code in folder
2. Type 'cargo init'
3. Copy V0.4 Code or File
4. Create 'input' and 'output' folder

The main folder should look like

![this](https://github.com/Chssam/pixel_maper/blob/main/sources/pxls-placemaper%20outlook.png)

5. 'input' items can be found on [Pxls Items](https://pxls.space/extra)
6. Change any value in **settings.ron** file
7. Type 'cargo run --release', or 'cargo run' (Which is slow, very very slow, debug mode)
8. Items will be generated in 'output' folder
9. Your STATS!

Warn:
- Not accurate, maybe
- There's tiny difference between Clueless's placemap vs this one

Cover
- V0.1.1 - Use txt as palette
- V0.2.0 - Use json as palette (Which include name)
- V0.2.0 - slower 20s than V0.1.1
- V0.3.0 - All items from [Pxls Items](https://pxls.space/extra)
- V0.3.0 - Same process time as V0.1.1
- V0.Latest - Image Name Lookup Changes
- V0.Latest - Abandoned the name, follow up with the latest number instead
- V0.4 - With GIF Placemap!

Suggest to use the latest, got more features and ~~bug~~ fixes.

Additional:
- Only tested after C71
- Somehow can't read C74 logs 
- V0.1, 2, 3 tested in debug mode, while V0.4 tested in release mode, basically V0.4 slower because more feature.


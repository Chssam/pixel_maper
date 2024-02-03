use image::{
    codecs::gif::{GifEncoder, Repeat},
    *,
};
use serde::Deserialize;
use sha256::digest;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::prelude::*,
    time::{Duration, SystemTime},
};
use xz2::read::XzDecoder;

const IN: &str = "./input/";
const OUT: &str = "./output/";
const VERSION: &str = "0.4.0";

#[derive(Debug, Deserialize)]
struct Settings {
    user_key: String,
    name: String,
    canvas_code: u32,
    palette_code: u32,
    pix_th: Vec<u32>,
    pix_frame: u32,
}

struct PaletteInfo {
    rgba: Rgba<u8>,
    name: String,
}

fn count_pixel(imaged: ImageBuffer<Rgba<u8>, Vec<u8>>) -> usize {
    imaged.pixels().filter(|x| x.0[3] == 255).count()
}

fn main() -> anyhow::Result<()> {
    let begin_time = SystemTime::now();
    let Settings {
        user_key,
        name,
        canvas_code,
        palette_code: palette,
        pix_th,
        pix_frame,
    } = ron::de::from_reader(File::open("settings.ron").expect("Settings.ron not exist"))
        .expect("Invalid .ron format");

    let pal_vec = {
        let mut collect_pal = Vec::with_capacity(40);
        let palette_ctx = fs::read_to_string(format!("{IN}palette_{palette}_paintnet.txt"))
            .expect("Palette not exist");
        for value in palette_ctx.lines() {
            let splited: Vec<&str> = value.trim().split(";").collect();
            let [hexy, color_name] = splited[..] else {
                continue;
            };
            if hexy.is_empty() || color_name.is_empty() {
                continue;
            }
            let hexed = hex::decode(hexy.trim().trim_start_matches("#")).expect("Invalid Hex code");
            let [a, r, g, b] = hexed[..] else {
                panic!("Unvalid RGB");
            };
            collect_pal.push(PaletteInfo {
                rgba: Rgba([r, g, b, a]),
                name: color_name.trim().to_owned(),
            });
        }
        collect_pal.shrink_to_fit();
        collect_pal
    };

    let (mut img_placed, mut img_undo, mut img_survivor, mut img_gif) = {
        let (width, height) =
            image::open(format!("{IN}canvas-{canvas_code}-initial.png"))?.dimensions();
        let copy_intial = RgbaImage::new(width, height);
        (
            copy_intial.clone(),
            copy_intial.clone(),
            copy_intial.clone(),
            vec![Frame::from_parts(
                copy_intial,
                0,
                0,
                Delay::from_saturating_duration(Duration::from_millis(500)),
            )],
        )
    };
    // let mut img_frames: Vec<RgbaImage> = Vec::new();

    let mut logs = String::new();
    XzDecoder::new(
        File::open(format!("{IN}pixels_c{canvas_code}.sanit.log.tar.xz"))
            .expect("Log file not exist"),
    )
    .read_to_string(&mut logs)
    .expect("Log file problems");

    let mut logs_queue: Vec<&str> = logs.trim().split("\n").collect();
    let mut old_pix = Rgba([0; 4]);
    let mut active_pix = 0;
    let mut previous_pix_survivor_color = Rgba([0; 4]);

    let [mut pixels, mut undo, mut replaced] = [0; 3];
    let mut color_use: HashMap<i8, i32> = HashMap::new();
    let mut pix_place: Vec<String> = Vec::new();
    let mut old_survivor_pix: HashMap<(u32, u32), Rgba<u8>> = HashMap::new();

    for lines in logs_queue.drain(..) {
        let splited: Vec<&str> = lines.split("\t").collect();
        let [date, rand_hash, x, y, color_index, action] = splited[..] else {
            continue;
        };

        let digest_format = [date, x, y, color_index, &user_key].join(",");
        let digested = digest(digest_format.clone());

        let (x, y) = (x.parse()?, y.parse()?);

        if digested.encode_utf16().ne(rand_hash.encode_utf16()) {
            if action.contains("undo") {
                let Some(old_survivor) = old_survivor_pix.remove(&(x, y)) else {
                    continue;
                };
                img_survivor.put_pixel(x, y, old_survivor);
                continue;
            }
            let old_survivor = img_survivor.get_pixel(x, y);
            old_survivor_pix.insert((x, y), *old_survivor);
            img_survivor.put_pixel(x, y, Rgba([0; 4]));
            continue;
        }

        let indexed: usize = color_index.parse()?;
        let rgba = pal_vec[indexed].rgba;

        if action.contains("undo") {
            if pix_th.contains(&pixels) {
                pix_place.pop();
            }
            if pixels % pix_frame == 0 {
                img_gif.pop();
            }
            if old_pix.0[3] == 255 {
                replaced -= 1;
            }
            if let Some(color) = color_use.get_mut(&active_pix) {
                *color -= 1;
            } else {
                panic!("No Way")
            };
            pixels -= 1;
            undo += 1;

            img_placed.put_pixel(x, y, old_pix);
            img_survivor.put_pixel(x, y, previous_pix_survivor_color);
            img_undo.put_pixel(x, y, pal_vec[active_pix as usize].rgba);
            continue;
        }

        pixels += 1;
        active_pix = indexed as i8;
        old_pix = *img_survivor.get_pixel(x, y);
        if old_pix.0[3] == 255 {
            replaced += 1;
        }

        color_use
            .entry(active_pix)
            .and_modify(|v| *v += 1)
            .or_insert(1);

        img_placed.put_pixel(x, y, rgba);

        previous_pix_survivor_color = *img_survivor.get_pixel(x, y);
        img_survivor.put_pixel(x, y, rgba);
        if pixels % pix_frame == 0 {
            img_gif.push(Frame::from_parts(
                img_placed.clone(),
                0,
                0,
                Delay::from_saturating_duration(Duration::from_millis(100)),
            ));
        }
        if pix_th.contains(&pixels) {
            pix_place.push(format!("{pixels}\t{x}\t{y}\t{}", pal_vec[indexed].name));
        }
    }

    img_gif.push(Frame::from_parts(
        img_placed.clone(),
        0,
        0,
        Delay::from_saturating_duration(Duration::from_millis(1000)),
    ));

    let format_name = |naming: &str| -> String { format!("{OUT}C{canvas_code} {name} {naming}") };

    img_placed.save(format_name("Placemap.png"))?;
    img_undo.save(format_name("Placemap Undo.png"))?;
    img_survivor.save(format_name("Placemap Survivor.png"))?;
    let gif_file = File::create(format_name("Placemap Gif.gif"))?;
    let mut encode_gif = GifEncoder::new(gif_file);
    encode_gif.set_repeat(Repeat::Infinite)?;
    encode_gif.encode_frames(img_gif.into_iter())?;

    let survived = count_pixel(img_survivor);
    let diff_pos_place = count_pixel(img_placed);
    let diff_pos_undo = count_pixel(img_undo);

    let mut sort_color: Vec<(i8, i32)> = color_use.into_iter().map(|(a, b)| (a, b)).collect();
    sort_color.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let sort_string: String = sort_color
        .iter()
        .enumerate()
        .map(|(rank, (a, b))| {
            format!(
                "{}\t{}\t{:.4}\t{}\n",
                rank + 1,
                b,
                *b as f32 / pixels as f32 * 100.0,
                pal_vec[*a as usize].name
            )
        })
        .collect();
    let make_string = format!(
        "Version: {}\nCanvas: {}\nUsers: {}\nPixels: {}\nSurvivor: {}\nUndo: {}\nReplace: {}\n\nDifferent Position\nPlace: {}\nUndo: {}\n\nTop Color:\nPlace\tUsed\tPercent\tColor\n{}\n\nPlace\tX\tY\tColor\n{}",
        VERSION, canvas_code, name, pixels, survived, undo, replaced, diff_pos_place, diff_pos_undo, sort_string, pix_place.join("\n")
    );
    fs::write(format!("{OUT}C{canvas_code} Stats {name}.txt"), make_string)?;

    let time_taken = begin_time.elapsed()?;
    println!("Time Taken: {:?}", time_taken);

    Ok(())
}

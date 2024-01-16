use image::{ImageBuffer, Rgba};
use serde::Deserialize;
use sha256::digest;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::prelude::*,
};
use xz2::read::XzDecoder;

const IN: &str = "./input/";
const OUT: &str = "./output/";

#[derive(Debug, Deserialize)]
struct Settings {
    user_key: String,
    name: String,
    canvas_code: u32,
    palette: u32,
    pix_th: Vec<u32>,
}

fn count_pixel(imaged: ImageBuffer<Rgba<u8>, Vec<u8>>) -> usize {
    imaged.pixels().filter(|x| x.0[3] == 255).count()
}

fn main() -> anyhow::Result<()> {
    let Settings {
        user_key,
        name,
        canvas_code,
        palette,
        pix_th,
    } = ron::de::from_reader(File::open("settings.ron")?)?;

    let palette_ctx = fs::read_to_string(format!("{IN}palette_{palette}_paintnet.txt"))?;
    let mut original_img =
        image::open(format!("{IN}canvas-{canvas_code}-initial.png"))?.into_rgba8();
    original_img.fill(0);
    let (mut palette_indexed, mut stored_color_name) = (Vec::new(), Vec::new());
    let mut logs = String::new();
    XzDecoder::new(File::open(format!(
        "{IN}pixels_c{canvas_code}.sanit.log.tar.xz"
    ))?)
    .read_to_string(&mut logs)?;

    for value in palette_ctx.lines().skip(1) {
        let splited: Vec<&str> = value.trim().split(";").collect();
        let [hexy, color_name] = splited[..] else {
            continue;
        };
        let hexed = hex::decode(hexy.trim().trim_start_matches("#"))?;
        let [a, r, g, b] = hexed[..] else {
            panic!("Unvalid RGB");
        };
        palette_indexed.push(Rgba([r, g, b, a]));
        stored_color_name.push(color_name.trim());
    }

    let (mut img_placed, mut img_undo, mut img_survivor) =
        (original_img.clone(), original_img.clone(), original_img);
    let logs_queue: Vec<&str> = logs.trim().split("\n").collect();
    let mut old_pix = Rgba([0; 4]);
    let mut active_pix = 0;
    let mut previous_pix_survivor_color = Rgba([0; 4]);

    let [mut pixels, mut undo, mut replaced] = [0; 3];
    let mut color_use: HashMap<i8, i32> = HashMap::new();
    let mut pix_place: Vec<String> = Vec::new();
    let mut old_survivor_pix: HashMap<(u32, u32), Rgba<u8>> = HashMap::new();

    for lines in logs_queue {
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
        let rgba = palette_indexed[indexed];

        if action.contains("undo") {
            if pix_th.iter().any(|x| x == &pixels) {
                pix_place.pop();
            }
            pixels -= 1;
            undo += 1;
            if old_pix.0[3] == 255 {
                replaced -= 1;
            }
            if let Some(color) = color_use.get_mut(&active_pix) {
                *color -= 1;
            } else {
                panic!("No Way")
            };
            img_placed.put_pixel(x, y, old_pix);
            img_survivor.put_pixel(x, y, previous_pix_survivor_color);
            img_undo.put_pixel(x, y, palette_indexed[active_pix as usize]);
            continue;
        }

        pixels += 1;
        active_pix = indexed as i8;
        old_pix = *img_survivor.get_pixel(x, y);
        if old_pix.0[3] == 255 {
            replaced += 1;
        }
        if let Some(color) = color_use.get_mut(&active_pix) {
            *color += 1;
        } else {
            color_use.insert(active_pix, 1);
        };
        img_placed.put_pixel(x, y, rgba);
        previous_pix_survivor_color = *img_survivor.get_pixel(x, y);
        img_survivor.put_pixel(x, y, rgba);
        if pix_th.iter().any(|x| x == &pixels) {
            pix_place.push(format!(
                "{pixels}\t\t{x}\t{y}\t{}",
                stored_color_name[indexed]
            ));
        }
    }

    let format_name = |naming: &str| -> String {
        format!("{OUT}C{canvas_code} {naming} {name}.png")
    };

    img_placed.save(format_name("Placemap"))?;
    img_undo.save(format_name("Placemap Undo"))?;
    img_survivor.save(format_name("Placemap Survivor"))?;

    let survived = count_pixel(img_survivor);
    let diff_pos_place = count_pixel(img_placed);
    let diff_pos_undo = count_pixel(img_undo);

    let mut sort_color: Vec<(i8, i32)> = color_use.into_iter().map(|(a, b)| (a, b)).collect();
    sort_color.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    let sort_string: String = sort_color
        .iter()
        .enumerate()
        .map(|(rank, (a, b))| format!("{}\t{}\t{}\n", rank + 1, b, stored_color_name[*a as usize]))
        .collect();
    let make_string = format!(
        "Users: {}\nPixels: {}\nSurvivor: {}\nUndo: {}\nReplace: {}\n\nDifferent Position\nPlace: {}\nUndo: {}\n\nTop Color:\nPlace\tUsed\tColor\n{}\n\nPlace\tX\tY\tIndex\n{}",
        name, pixels, survived, undo, replaced, diff_pos_place, diff_pos_undo, sort_string, pix_place.join("\n")
    );
    fs::write(format!("{OUT}C{canvas_code} Stats {name}.txt"), make_string)?;

    Ok(())
}

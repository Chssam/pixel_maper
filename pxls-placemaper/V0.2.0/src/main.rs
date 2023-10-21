use image::{Rgba, RgbaImage, ImageBuffer};
use serde::{Deserialize, Serialize};
use sha256::digest;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::prelude::*,
};
use xz2::read::XzDecoder;

const IN: &str = "./input/";
const OUT: &str = "./output/";

#[derive(Serialize, Deserialize, Debug)]
struct Palette {
    #[serde(flatten)]
    color: HashMap<String, String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Settings {
    user_key: String,
    canvas_code: u32,
    name: String,
    pix_th: Vec<u32>,
}

fn filt_str(value: &str) -> String {
    value.to_string().chars().filter(|a| a.is_ascii_alphanumeric()).collect()
}

fn count_pixel(imaged: ImageBuffer<Rgba<u8>, Vec<u8>>) -> usize {
    imaged.pixels().filter(|x| x.0[3] == 255).count()
}

fn main() {
    // Read "settings.ron"
    let open_settings = File::open("settings.ron").unwrap();
    let Settings {
        user_key,
        canvas_code,
        name,
        pix_th,
    } = ron::de::from_reader(open_settings).unwrap();

    let xz_file = format!("{IN}pixels_c{canvas_code}.sanit.log.tar.xz");
    let palette_file = format!("{IN}palette_c{canvas_code}.json");
    let pix_img_placed = format!("{OUT}C{canvas_code}_Placemap_{name}.png");
    let pix_img_undo = format!("{OUT}C{canvas_code}_Placemap_Undo_{name}.png");
    let pix_img_survivor = format!("{OUT}C{canvas_code}_Placemap_Survivor_{name}.png");
    let user_stats = format!("{OUT}C{canvas_code}_Stats_{name}.txt");

    let image_file = image::open(format!("{IN}Canvas_{canvas_code}_Initial.png")).unwrap();
    let (img_x, img_y) = (image_file.width(), image_file.height());

    // Palette Index
    let mut palette_indexed = Vec::new();
    let mut stored_color_name = Vec::new();
    let mut palette_ctx = String::new();
    File::open(palette_file)
        .unwrap()
        .read_to_string(&mut palette_ctx)
        .unwrap();

    // Decode ".xz" File
    let xz_file = File::open(xz_file).unwrap();
    let mut decode_xz = XzDecoder::new(xz_file);
    let mut ctx = String::new();
    decode_xz.read_to_string(&mut ctx).unwrap();

    // let mut lines: Vec<&str> = palette_ctx.trim().split("\n").collect();
    // lines.remove(0);
    // for single_line in lines {
    //     let c = single_line.split(" ").next().unwrap();
    //     let hexed = hex::decode(c).unwrap();
    //     let [a, r, g, b] = hexed[..] else {
    //         panic!("Unvalid ARGB (RGBA)");
    //     };
    //     palette_indexed.push(Rgba([r, g, b, a]));
    // }

    for value in palette_ctx.lines() {
        let splited: Vec<&str> = value.trim().split(":").collect();
        let [color_name, hexnt] = splited[..] else {
            continue;
        };
        let real_color_name = filt_str(color_name);
        let hexy = filt_str(hexnt);
        let hexed = hex::decode(hexy).unwrap();
        let [r, g, b] = hexed[..] else {
            panic!("Unvalid RGB");
        };
        palette_indexed.push(Rgba([r, g, b, 255]));
        stored_color_name.push(real_color_name);

    }

    // Filter User | Build Image
    let mut img_placed = RgbaImage::new(img_x, img_y);
    let mut img_undo = RgbaImage::new(img_x, img_y);
    let mut img_survivor = RgbaImage::new(img_x, img_y);
    let vec_queue: Vec<&str> = ctx.trim().split("\n").collect();
    let mut old_pix = Rgba([0; 4]);
    let mut active_pix = 0;
    let mut previous_pix_survivor_color = Rgba([0; 4]);

    let mut pixels = 0;
    let mut undo = 0;
    let mut replaced = 0;
    let mut color_use: HashMap<i8, i32> = HashMap::new();
    let mut pix_place = String::new();
    let mut old_survivor_pix: HashMap<(u32, u32), Rgba<u8>> = HashMap::new();

    for value in vec_queue {
        let splited: Vec<&str> = value.split("\t").collect();
        let [date, rand_hash, x, y, color_index, action] = splited[..] else {
            continue;
        };

        let digest_format = [date, x, y, color_index, &user_key].join(",");
        let digested = digest(digest_format.clone());

        let (x, y) = (x.parse().unwrap(), y.parse().unwrap());
        if digested.encode_utf16().ne(rand_hash.encode_utf16()) {
            if action == "undo" {
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

        let indexed: usize = color_index.parse().unwrap();
        let rgba = palette_indexed[indexed];

        if action.contains("undo") {
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
        old_pix = *img_placed.get_pixel(x, y);
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
            pix_place.push_str(&format!("{pixels}\t\t{x}\t{y}\t{}\n", stored_color_name[indexed]));
        }
    }

    img_placed.save(pix_img_placed).unwrap();
    img_undo.save(pix_img_undo).unwrap();
    img_survivor.save(pix_img_survivor).unwrap();

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
        "Users: {}\nPixels: {}\nSurvivor: {}\nUndo: {}\nReplace: {}\n\nDifferent Position\nPlace: {}\nUndo: {}\n\nTop Color:\nPlace\tUsed\tColor\n{}\n\nPix place\tX\tY\tIndex\n{}",
        name, pixels, survived, undo, replaced, diff_pos_place, diff_pos_undo, sort_string, pix_place
    );
    fs::write(user_stats, make_string).unwrap();
}

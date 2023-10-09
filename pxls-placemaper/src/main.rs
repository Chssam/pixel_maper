use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha256::digest;
use std::fs::{self, File};
use std::io::prelude::*;
use xz2::read::XzDecoder;

/*
File name in Input
Ex: Canvas Code = 71
LOG: pixels_c71.sanit.log.tar.xz
IMAGE: Canvas_71_Initial.png
PALETTE: palette_c71.txt | Got From Clueless => /palette => Paint.Net
*/

const INPUT: &str = "./input/";
const OUTPUT: &str = "./output/";

#[derive(Debug, Deserialize, Serialize)]
struct Settings {
    user_key: String,
    canvas_code: u32,
    name: String,
    pix_th: Vec<u32>,
}

#[derive(Default)]
struct Stats {
    pixels: u32,
    undo: u32,
    survivor: u32,
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

    let xz_file = format!("{INPUT}pixels_c{canvas_code}.sanit.log.tar.xz");
    let palette_file = format!("{INPUT}palette_c{canvas_code}.txt");
    let name_img = format!("{OUTPUT}C{canvas_code}_Placemap_{name}.png");
    let name_img_survivor = format!("{OUTPUT}C{canvas_code}_Placemap_Survivor_{name}.png");
    let output_stats = format!("{OUTPUT}C{canvas_code}_Stats_{name}.txt");

    let image_file = image::open(format!("{INPUT}Canvas_{canvas_code}_Initial.png")).unwrap();
    let (img_x, img_y) = (image_file.width(), image_file.height());

    // Decode ".xz" File
    let xz_file = File::open(xz_file).unwrap();
    let mut decode_xz = XzDecoder::new(xz_file);
    let mut ctx = String::new();
    decode_xz.read_to_string(&mut ctx).unwrap();

    // Palette Index
    let mut palette_indexed = Vec::new();
    let mut palette_ctx = String::new();
    File::open(palette_file)
        .unwrap()
        .read_to_string(&mut palette_ctx)
        .unwrap();
    let mut lines: Vec<&str> = palette_ctx.trim().split("\n").collect();
    lines.remove(0);
    for single_line in lines {
        let c = single_line.split(" ").next().unwrap();
        let hexed = hex::decode(c).unwrap();
        palette_indexed.push(hexed);
    }

    // Filter User | Build Image
    let mut imged_placed_pixels = RgbaImage::new(img_x, img_y);
    let mut imged_survivor_pixels = RgbaImage::new(img_x, img_y);
    let vec_queue: Vec<&str> = ctx.trim().split("\n").collect();
    let mut previous_pix_color = Rgba([0; 4]);
    let Stats {
        mut pixels,
        mut undo,
        mut survivor,
    } = Stats::default();
    let mut pix_place: Vec<String> = Vec::new();
    let mut full_string = String::new();

    for value in vec_queue {
        let splited: Vec<&str> = value.split("\t").collect();
        let [date, rand_hash, x, y, color_index, action] = splited[..] else {
            continue;
        };
        let digest_format = [date, x, y, color_index, &user_key].join(",");
        let digested = digest(digest_format.clone());

        let (x, y) = (x.parse().unwrap(), y.parse().unwrap());
        if digested.encode_utf16().ne(rand_hash.encode_utf16()) {
            imged_survivor_pixels.put_pixel(x, y, Rgba([0; 4]));
            continue;
        }
        let indexed: usize = color_index.parse().unwrap();
        let in_color = &palette_indexed[indexed];
        let rgba = Rgba([in_color[1], in_color[2], in_color[3], 255]);

        if !action.contains("undo") {
            imged_survivor_pixels.put_pixel(x, y, rgba);
        }

        if action.contains("undo") {
            pix_place.pop();
            pixels -= 1;
            undo += 1;
            if pix_th[..].contains(&pixels) {
                pix_place.pop();
            }
            imged_placed_pixels.put_pixel(x, y, previous_pix_color);
            continue;
        }

        pixels += 1;
        previous_pix_color = *imged_placed_pixels.get_pixel(x, y);
        if !action.contains("undo") {
            imged_placed_pixels.put_pixel(x, y, rgba);
        }
        if pix_th.iter().any(|x| x == &pixels) {
            pix_place.push(format!("{pixels}: {x}\t{y}\t{color_index}\n"));
        }
    }
    survivor = imged_survivor_pixels
        .pixels()
        .filter(|x| x.0[3] > 0)
        .count() as u32;
    for s in pix_place {
        full_string.push_str(&s);
    }
    imged_placed_pixels.save(name_img).unwrap();
    imged_survivor_pixels.save(name_img_survivor).unwrap();
    let make_string = format!(
        "Users: {}\nPixels: {}\nSurvivor: {}\nUndo: {}\n{}",
        name, pixels, survivor, undo, full_string
    );
    fs::write(output_stats, make_string).unwrap();
}

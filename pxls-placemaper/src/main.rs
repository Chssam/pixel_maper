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

#[derive(Debug, Deserialize, Serialize)]
struct Settings {
    user_key: String,
    canvas_code: u32,
    name: String,
}

#[derive(Default)]
struct Stats {
    pixels: u32,
    undo: u32,
}

fn main() {
    // Read "settings.ron"
    let open_settings = File::open("settings.ron").unwrap();
    let settings: Settings = ron::de::from_reader(open_settings).unwrap();
    let canvas_code = settings.canvas_code;

    let xz_file = format!("./input/pixels_c{}.sanit.log.tar.xz", canvas_code);
    let palette_file = format!("./input/palette_c{}.txt", canvas_code);
    let name_img = format!(
        "./output/C{}_Placemap_{}.png",
        canvas_code, settings.name
    );
    let output_stats = format!(
        "./output/C{}_Stats_{}.txt",
        canvas_code, settings.name
    );

    let image_file = image::open(format!("./input/Canvas_{}_Initial.png", canvas_code)).unwrap();
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
    let mut imged = RgbaImage::new(img_x, img_y);
    let vec_queue: Vec<&str> = ctx.trim().split("\n").collect();
    let mut previous_pix_color = Rgba([0; 4]);
    let mut stats = Stats::default();

    for value in vec_queue {
        let splited: Vec<&str> = value.split("\t").collect();
        let [date, rand_hash, x, y, color_index, action] = splited[..] else {
            continue;
        };
        let digest_format = [date, x, y, color_index, &settings.user_key].join(",");
        let digested = digest(digest_format.clone());
        if digested.encode_utf16().ne(rand_hash.encode_utf16()) {
            continue;
        }
        let indexed: usize = color_index.parse().unwrap();
        let in_color = &palette_indexed[indexed];
        let (x, y) = (x.parse().unwrap(), y.parse().unwrap());
        let rgba = Rgba([in_color[1], in_color[2], in_color[3], 255]);

        if action.contains("undo") {
            stats.pixels -= 1;
            stats.undo += 1;
            imged.put_pixel(x, y, previous_pix_color);
            continue;
        }
        stats.pixels += 1;
        previous_pix_color = *imged.get_pixel(x, y);
        imged.put_pixel(x, y, rgba);
    }
    imged.save(name_img).unwrap();
    let make_string = format!("Users: {}\nPixels: {}\nUndo: {}", settings.name, stats.pixels, stats.undo);
    fs::write(output_stats, make_string).unwrap();
}

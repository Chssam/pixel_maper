use std::io::prelude::*;
use std::fs::File;
use image::{RgbaImage, Rgba};
use xz2::read::XzDecoder;
use sha256::digest;

const KEY: &str = "YOUR KEY";

const CANVAS_CODE: u32 = 71;
// Ex: pixels_c71.sanit.log.tar.xz
// Ex: palette_c71.txt | Got From Clueless => Paint.Net

const OUTPUT_NAME: &str = "Mapped";

fn main() {
    let xz_name = format!("pixels_c{}.sanit.log.tar.xz", CANVAS_CODE);
    let palette_name = format!("palette_c{}.txt", CANVAS_CODE);
    let output_name = format!("C{}_Placemap_{}.png", CANVAS_CODE, OUTPUT_NAME);

    // Decode .xz File
    let xz_file = File::open(xz_name).unwrap();
    let mut decode_xz = XzDecoder::new(xz_file);
    let mut ctx = String::new();
    decode_xz.read_to_string(&mut ctx).unwrap();

    // Palette Index
    let mut palette_indexed = Vec::new();
    let mut palette_ctx = String::new();
    File::open(palette_name).unwrap().read_to_string(&mut palette_ctx).unwrap();
    let mut lines: Vec<&str> = palette_ctx.trim().split("\n").collect();
    lines.remove(0);
    for single_line in lines {
        let c = single_line.split(" ").next().unwrap();
        let hexed = hex::decode(c).unwrap();
        palette_indexed.push(hexed);
    }

    // Filter User
    let mut imged = RgbaImage::new(1546, 1546);
    let vec_queue: Vec<&str> = ctx.trim().split("\n").collect();
    let mut previous_pix_color = Rgba([0; 4]);

    for value in vec_queue {
        let splited: Vec<&str> = value.split("\t").collect();
        let [date, rand_hash, x, y, color_index, action] = splited[..] else {
            continue;
        };
        let digest_format = [date, x, y, color_index, KEY].join(",");
        let digested = digest(digest_format.clone());
        if digested.encode_utf16().ne(rand_hash.encode_utf16()) {
            continue;
        }
        let indexed: usize = color_index.parse().unwrap();
        let in_color = &palette_indexed[indexed];
        let (x, y) = (x.parse().unwrap(), y.parse().unwrap());
        let rgba = Rgba([in_color[1], in_color[2], in_color[3], 255]);

        if action.contains("undo") {
            imged.put_pixel(x, y, previous_pix_color);
            continue;
        }
        previous_pix_color = *imged.get_pixel(x, y);
        imged.put_pixel(x, y, rgba);
    }
    imged.save(output_name).unwrap();
}

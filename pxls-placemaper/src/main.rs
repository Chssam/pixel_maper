use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha256::digest;
use std::{
    fs::{self, File},
    io::prelude::*, collections::HashMap,
};
use xz2::read::XzDecoder;

/*
File name in Input
Ex: Canvas Code = 71
LOG: pixels_c71.sanit.log.tar.xz
IMAGE: Canvas_71_Initial.png
PALETTE: palette_c71.txt | Got From Clueless => /palette => Paint.Net
*/

const IN: &str = "./input/";
const OUT: &str = "./output/";

#[derive(Debug, Deserialize, Serialize)]
struct Settings {
    user_key: String,
    canvas_code: u32,
    name: String,
    pix_th: Vec<u32>,
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
    let palette_file = format!("{IN}palette_c{canvas_code}.txt");
    let pix_img_placed = format!("{OUT}C{canvas_code}_Placemap_{name}.png");
    let pix_img_undo = format!("{OUT}C{canvas_code}_Placemap_Undo_{name}.png");
    let pix_img_survivor = format!("{OUT}C{canvas_code}_Placemap_Survivor_{name}.png");
    let user_stats = format!("{OUT}C{canvas_code}_Stats_{name}.txt");

    let image_file = image::open(format!("{IN}Canvas_{canvas_code}_Initial.png")).unwrap();
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
    let mut img_placed = RgbaImage::new(img_x, img_y);
    let mut img_undo = RgbaImage::new(img_x, img_y);
    let mut img_survivor = RgbaImage::new(img_x, img_y);
    let vec_queue: Vec<&str> = ctx.trim().split("\n").collect();
    let mut old_pix = Rgba([0; 4]);
    let mut hold_pix = Rgba([0; 4]);
    let mut previous_pix_survivor_color = Rgba([0; 4]);

    let mut pixels = 0;
    let mut undo = 0;
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

        let indexed: usize = color_index.parse().unwrap();
        let in_color = &palette_indexed[indexed];
        let rgba = Rgba([in_color[1], in_color[2], in_color[3], 255]);

        if action.contains("undo") {
            pixels -= 1;
            undo += 1;
            img_placed.put_pixel(x, y, old_pix);
            img_survivor.put_pixel(x, y, previous_pix_survivor_color);
            img_undo.put_pixel(x, y, hold_pix);
            continue;
        }

        pixels += 1;
        hold_pix = rgba;
        old_pix = *img_placed.get_pixel(x, y);
        img_placed.put_pixel(x, y, rgba);
        previous_pix_survivor_color = *img_survivor.get_pixel(x, y);
        img_survivor.put_pixel(x, y, rgba);
        if pix_th.iter().any(|x| x == &pixels) {
            pix_place.push_str(&format!("{pixels}\t\t{x}\t{y}\t{color_index}\n"));
        }
    }
    let survived = img_survivor.pixels().filter(|x| x.0[3] == 255).count();
    let diff_pos_place = img_placed.pixels().filter(|x| x.0[3] == 255).count();
    let diff_pos_undo = img_undo.pixels().filter(|x| x.0[3] == 255).count();
    img_placed.save(pix_img_placed).unwrap();
    img_undo.save(pix_img_undo).unwrap();
    img_survivor.save(pix_img_survivor).unwrap();
    let make_string = format!(
        "Users: {}\nPixels: {}\nSurvivor: {}\nUndo: {}\n\nDifferent Position\nPlace: {}\nUndo: {}\n\nPix place\tX\tY\tIndex\n{}",
        name, pixels, survived, undo, diff_pos_place, diff_pos_undo, pix_place
    );
    fs::write(user_stats, make_string).unwrap();
}

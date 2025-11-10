use anyhow::anyhow;
use image::*;
use log::{error, info};
use serde::Deserialize;
use std::{borrow::Cow, collections::HashMap, fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct Settings {
	pub http: String,
	pub output_file: String,
	pub log_file_name: String,
	pub pix_th: Vec<u32>,
	pub pix_per_frame: u32,
	pub frame_delay: u64,
	pub img_size: (u32, u32),
}

/// Hex Color | Color Name
pub struct PaletteVec(pub HashMap<Rgba<u8>, Cow<'static, str>>);

impl Default for PaletteVec {
	fn default() -> Self {
		Self(HashMap::with_capacity(40))
	}
}

impl PaletteVec {
	pub fn new(input_dir: &Path, palette_code: &str) -> Result<PaletteVec, anyhow::Error> {
		let mut collect_pal = PaletteVec::default();
		let palette_path = input_dir.join(format!("palette_{palette_code}_paintnet.txt"));
		let palette_ctx = fs::read_to_string(palette_path)?;

		for value in palette_ctx.lines() {
			let splited: Vec<&str> = value.trim().split('\t').collect();
			let [_n, color_name, pre_hexy] = splited[..] else {
				continue;
			};
			if pre_hexy.is_empty() || color_name.is_empty() {
				continue;
			}
			let hexy = format!("{}FF", pre_hexy.trim());
			let hexed = hex::decode(hexy).map_err(|err| anyhow!("Hex code fail: {:?}", err))?;
			let [r, g, b, a] = hexed[..] else {
				error!("Invalid ARGB");
				continue;
			};
			let rgba = Rgba([r, g, b, a]);
			let name = Cow::from(color_name.trim().to_owned());
			collect_pal.0.insert(rgba, name);
		}

		collect_pal.0.shrink_to_fit();
		info!("Complete reading Palette {}.", palette_code);
		Ok(collect_pal)
	}
	pub fn to_color_used(&self) -> ColorUsed {
		let mut color_used = ColorUsed::default();
		for (n, _) in self.0.iter() {
			color_used.0.insert(*n, 0);
		}
		println!("Color Used: {:?}", color_used.0);
		color_used
	}
}

///
pub struct ImageCollection {
	pub place: ImageBuffer<Rgba<u8>, Vec<u8>>,
	pub undo: ImageBuffer<Rgba<u8>, Vec<u8>>,
	pub survivor: ImageBuffer<Rgba<u8>, Vec<u8>>,
	pub gif: Vec<Frame>,
}

#[derive(Default)]
pub struct OutputInfo {
	pub pixels: u32,
	pub undo: u32,
	pub replaced: u32,
	pub survived: usize,
	pub diff_pos_place: usize,
	pub diff_pos_undo: usize,
	pub color_used: ColorUsed,
	pub pix_place: Vec<String>,
}

impl OutputInfo {
	pub fn new(color_used: ColorUsed) -> Self {
		Self {
			color_used,
			..Default::default()
		}
	}
}

#[derive(Default)]
pub struct ColorUsed(pub HashMap<Rgba<u8>, i32>);

impl ColorUsed {
	#[inline]
	pub fn add_used(&mut self, index: &Rgba<u8>) {
		*self.0.get_mut(index).unwrap() += 1;
	}
	#[inline]
	pub fn sub_used(&mut self, index: &Rgba<u8>) {
		*self.0.get_mut(index).unwrap() -= 1;
	}
}

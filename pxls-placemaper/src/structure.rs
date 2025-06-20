use anyhow::anyhow;
use image::*;
use log::{error, info};
use serde::Deserialize;
use std::{borrow::Cow, collections::HashMap, fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct Settings {
	pub user_key: String,
	pub name: String,
	pub canvas_code: String,
	pub palette_code: u8,
	pub pix_th: Vec<u32>,
	pub pix_per_frame: u32,
	pub frame_delay: u64,
}

pub struct PaletteInfo {
	pub rgba: Rgba<u8>,
	pub name: Cow<'static, str>,
}

pub struct PaletteVec(pub Vec<PaletteInfo>);

impl Default for PaletteVec {
	fn default() -> Self {
		Self(Vec::with_capacity(40))
	}
}

impl PaletteVec {
	pub fn new(input_dir: &Path, palette_code: u8) -> Result<PaletteVec, anyhow::Error> {
		let mut collect_pal = PaletteVec::default();
		let palette_path = input_dir.join(format!("palette_{palette_code}_paintnet.txt"));
		let palette_ctx = fs::read_to_string(palette_path)?;

		for value in palette_ctx.lines() {
			let splited: Vec<&str> = value.trim().split(';').collect();
			let [hexy, color_name] = splited[..] else {
				continue;
			};
			if hexy.is_empty() || color_name.is_empty() {
				continue;
			}
			let hexed =
				hex::decode(hexy.trim()).map_err(|err| anyhow!("Hex code fail: {:?}", err))?;
			let [a, r, g, b] = hexed[..] else {
				error!("Invalid ARGB");
				continue;
			};
			let rgba = Rgba([r, g, b, a]);
			let name = Cow::from(color_name.trim().to_owned());
			collect_pal.0.push(PaletteInfo { rgba, name });
		}

		collect_pal.0.shrink_to_fit();
		info!("Complete reading Palette {}.", palette_code);
		Ok(collect_pal)
	}
	pub fn to_color_used(&self) -> ColorUsed {
		let mut color_used = ColorUsed::default();
		for (n, _) in self.0.iter().enumerate() {
			color_used.0.insert(n as i8, 0);
		}
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
pub struct ColorUsed(pub HashMap<i8, i32>);

impl ColorUsed {
	#[inline]
	pub fn add_used(&mut self, index: &i8) {
		*self.0.get_mut(index).unwrap() += 1;
	}
	#[inline]
	pub fn sub_used(&mut self, index: &i8) {
		*self.0.get_mut(index).unwrap() -= 1;
	}
}

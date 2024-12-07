use image::*;
use serde::Deserialize;
use std::collections::HashMap;

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
	pub name: String,
}

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

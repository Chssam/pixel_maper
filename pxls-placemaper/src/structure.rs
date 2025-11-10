use anyhow::{Result, anyhow};
use image::{imageops::crop, *};
use log::{error, info};
use serde::Deserialize;
use std::{
	borrow::Cow,
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
pub struct Settings {
	pub user_key: String,
	pub name: String,
	pub canvas_code: String,
	pub palette_code: u8,
	pub pix_th: Vec<u32>,
	pub pix_per_frame: u32,
	pub frame_delay: u16,
}

pub struct PaletteInfo {
	pub name: Cow<'static, str>,
	pub rgba: Rgba<u8>,
}

pub struct PaletteVec {
	pub info: Vec<PaletteInfo>,
	pub blank_index: u8,
}

impl Default for PaletteVec {
	fn default() -> Self {
		Self {
			info: Vec::with_capacity(40),
			blank_index: 0,
		}
	}
}

impl PaletteVec {
	pub fn new(input_dir: &Path, palette_code: u8) -> Result<PaletteVec> {
		let mut pal_vec = PaletteVec::default();
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
			pal_vec.info.push(PaletteInfo { rgba, name });
		}

		pal_vec.info.shrink_to_fit();
		pal_vec.blank_index = pal_vec.info.len() as u8;
		info!("Complete reading Palette {}.", palette_code);
		Ok(pal_vec)
	}

	pub fn to_color_used(&self) -> ColorUsed {
		let mut color_used = ColorUsed::default();
		for (n, _) in self.info.iter().enumerate() {
			color_used.0.insert(n as u8, 0);
		}
		color_used
	}

	pub fn expand_palette(&self) -> Vec<(u8, u8, u8)> {
		let mut pal = self
			.info
			.iter()
			.map(|pal_info| {
				let [r, g, b, _a] = pal_info.rgba.0;
				(r, g, b)
			})
			.collect::<Vec<_>>();

		pal.push((0, 0, 0));
		pal.shrink_to_fit();
		pal
	}

	pub fn flat_palette(&self) -> Vec<u8> {
		let mut pal = self
			.info
			.iter()
			.map(|pal_info| {
				let [r, g, b, _a] = pal_info.rgba.0;
				[r, g, b]
			})
			.collect::<Vec<_>>();

		pal.push([0, 0, 0]);
		let mut new_pal = pal.into_iter().flatten().collect::<Vec<_>>();
		new_pal.shrink_to_fit();
		new_pal
	}
}

pub struct ImageCollection {
	pub place: GrayImage,
	pub undo: GrayImage,
	pub survivor: GrayImage,
	pub gif: Vec<GrayImageCropped>,
}

pub struct GrayImageCropped {
	pub left: u16,
	pub top: u16,
	pub img: GrayImage,
}

impl GrayImageCropped {
	pub fn new(left: u16, top: u16, img: GrayImage) -> Self {
		Self { left, top, img }
	}

	pub fn new_pure(img: GrayImage) -> Self {
		Self::new(0, 0, img)
	}

	pub fn diff_out_self(&mut self, other: &GrayImage, transparent: u8) {
		let transparent = Luma([transparent]);
		let left = self.left as u32;
		let top = self.top as u32;
		self.img
			.enumerate_pixels_mut()
			.for_each(|(x_pos, y_pos, px_own)| {
				let x = x_pos + left;
				let y = y_pos + top;
				let px_other = other.get_pixel(x, y);
				if px_own == px_other {
					*px_own = transparent;
				}
			});
	}

	pub fn crop_in(&mut self, transparent: u8) {
		let (left_new, top_new) = self.img.crop_in(transparent);
		self.left += left_new;
		self.top += top_new;
	}
}

impl ImageCollection {
	pub fn new_size(width: u32, height: u32, pixel: u8) -> Self {
		let intial = GrayImage::from_pixel(width, height, Luma([pixel]));
		Self::new(intial)
	}

	pub fn new(intial: GrayImage) -> Self {
		Self {
			place: intial.clone(),
			undo: intial.clone(),
			survivor: intial.clone(),
			gif: vec![GrayImageCropped::new_pure(intial)],
		}
	}
}

pub struct PixelInfoAt {
	pub at: u32,
	pub pixel: u8,
	pub x: u32,
	pub y: u32,
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
	pub pix_place: Vec<PixelInfoAt>,
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
pub struct ColorUsed(pub HashMap<u8, i32>);

impl ColorUsed {
	pub fn add_used(&mut self, index: &u8) {
		*self.0.get_mut(index).unwrap() += 1;
	}

	pub fn sub_used(&mut self, index: &u8) {
		*self.0.get_mut(index).unwrap() -= 1;
	}

	pub fn to_vec(self) -> Vec<(u8, i32)> {
		self.0.into_iter().collect::<Vec<_>>()
	}
}

pub trait ExtentGrayImage {
	/// Self, x, y
	fn crop_in(&mut self, transparent: u8) -> (u16, u16);
	fn save_in_color(&self, palette: &[(u8, u8, u8)], path: PathBuf) -> Result<()>;
	// fn diff_out(&mut self, other: &GrayImageCropped, transparent: u8);
}

impl ExtentGrayImage for GrayImage {
	fn crop_in(&mut self, transparent: u8) -> (u16, u16) {
		let (x_dim, y_dim) = self.dimensions();
		let (mut left, mut top) = (0, 0);
		let (mut right, mut bottom) = (x_dim, y_dim);
		let transparent = &Luma([transparent]);
		for x in 0..x_dim {
			let mut not_transparent = false;
			for y in 0..y_dim {
				not_transparent = self.get_pixel(x, y) != transparent;
				left = x;
				if not_transparent {
					break;
				}
			}
			if not_transparent {
				break;
			}
		}

		for y in 0..y_dim {
			let mut not_transparent = false;
			for x in left..x_dim {
				not_transparent = self.get_pixel(x, y) != transparent;
				top = y;
				if not_transparent {
					break;
				}
			}
			if not_transparent {
				break;
			}
		}

		for x in 0..x_dim {
			let mut not_transparent = false;
			let x = x_dim - x - 1;
			for y in top..y_dim {
				not_transparent = self.get_pixel(x, y) != transparent;
				right = x.saturating_add(1);
				if not_transparent {
					break;
				}
			}
			if not_transparent {
				break;
			}
		}

		for y in 0..y_dim {
			let mut not_transparent = false;
			let y = y_dim - y - 1;
			for x in left..right {
				not_transparent = self.get_pixel(x, y) != transparent;
				bottom = y.saturating_add(1);
				if not_transparent {
					break;
				}
			}
			if not_transparent {
				break;
			}
		}

		let width = right - left;
		let height = bottom - top;

		let sub_img = crop(self, left, top, width, height);
		*self = sub_img.to_image();
		(left as u16, top as u16)
	}

	fn save_in_color(&self, palette: &[(u8, u8, u8)], path: PathBuf) -> Result<()> {
		let img_colored = self
			.clone()
			.expand_palette(palette, Some(palette.len() as u8 - 1));
		img_colored.save(path)?;
		Ok(())
	}

	// fn diff_out(&mut self, other: &GrayImageCropped, transparent: u8) {
	// 	let transparent = Luma([transparent]);
	// 	other
	// 		.img
	// 		.enumerate_pixels()
	// 		.for_each(|(x_other, y_other, px_other)| {
	// 			let x = x_other + other.left as u32;
	// 			let y = y_other + other.top as u32;
	// 			let px = self.get_pixel_mut(x, y);
	// 			if px == px_other {
	// 				*px = transparent;
	// 			}
	// 		});
	// }
}

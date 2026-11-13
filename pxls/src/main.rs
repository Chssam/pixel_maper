//! Invalid Digit because -1 for Palette Index

use std::{
	collections::HashMap,
	fmt::Write as _,
	fs::{
		self,
		File,
	},
	io::{
		BufReader,
		prelude::*,
	},
	mem::take,
	path::{
		Path,
		PathBuf,
	},
	time::Instant,
};

use anyhow::Result;
use gif::{
	DisposalMethod,
	Frame,
	Repeat,
};
use glam::{
	UVec2,
	uvec2,
};
use image::{
	GrayImage,
	Luma,
	imageops::overlay,
};
use lzma_rust2::XzReader;
use sha256::digest;
use tracing::{
	error,
	info,
};

mod structure;
use structure::*;

use crate::helper::write_to_file;
mod helper;

const USER_UNDO: &str = "user undo";

// Formatting
const PLACE_LEN: usize = 10;
const USED_LEN: usize = 10;
const PERCENT_LEN: usize = 12;
const COLOR_LEN: usize = 20;
const MAX_POS_LEN: usize = 10;

fn main() {
	let begin_time = Instant::now();

	tracing_subscriber::fmt::init();

	if let Err(err) = stable_check_run() {
		error!("Unable to process: {:?}", err);
	};

	let time_taken = begin_time.elapsed();
	info!("Completed Placemap\nTime Taken: {:?}\nPress 'Enter' to terminate", time_taken);

	let mut buf = String::new();
	let _ = std::io::stdin().read_line(&mut buf);
}

fn stable_check_run() -> Result<()> {
	let settings = read_setting()?;

	let pal_vec = PaletteVec::new(settings.palette_code)?;

	let mut image_collection =
		ImageCollection::intial_img(&settings.canvas_code, pal_vec.blank_index)?;

	let mut output_info = OutputInfo::new(pal_vec.to_color_used());

	process_place_map(&pal_vec, &settings, &mut image_collection, &mut output_info)?;

	save_img_collection(image_collection, &settings, &pal_vec)?;

	create_user_stats(output_info, settings, pal_vec)?;

	Ok(())
}

fn create_user_stats(
	output_info: OutputInfo,
	full_set_setting: Settings,
	pal_vec: PaletteVec,
) -> Result<()> {
	let Settings {
		name,
		canvas_code,
		..
	} = full_set_setting;
	let OutputInfo {
		pixels,
		undo,
		replaced,
		survived,
		diff_pos_place,
		diff_pos_undo,
		color_used,
		mut pix_place,
	} = output_info;

	info!("Creating user stats...");

	let mut sort_color = color_used.to_vec();
	sort_color.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

	let total_sort = sort_color.len() * 54;
	let sort_string: String = sort_color.iter().enumerate().fold(
		String::with_capacity(total_sort),
		|mut v, (rank, (color_index, used))| {
			let pal_name = &pal_vec.info[*color_index as usize].name;
			let percent_pix = *used as f64 / pixels as f64 * 100.;

			write!(&mut v, "{: <PLACE_LEN$}", rank + 1).unwrap();
			write!(&mut v, "{: <USED_LEN$}", used).unwrap();
			write!(&mut v, "{: <PERCENT_LEN$.4}", percent_pix).unwrap();
			writeln!(&mut v, "{: <COLOR_LEN$}", pal_name).unwrap();

			v
		},
	);

	let total_place = pix_place.len() * 52;
	let to_pix_place = pix_place.drain(..).fold(
		String::with_capacity(total_place),
		|mut pix_str,
		 PixelInfoAt {
		     at,
		     color_index,
		     pos: UVec2 {
		         x,
		         y,
		     },
		 }| {
			let pal_name = &pal_vec.info[color_index as usize].name;

			write!(&mut pix_str, "{: <PLACE_LEN$}", at).unwrap();
			write!(&mut pix_str, "{: <MAX_POS_LEN$}", x).unwrap();
			write!(&mut pix_str, "{: <MAX_POS_LEN$}", y).unwrap();
			writeln!(&mut pix_str, "{: <COLOR_LEN$}", pal_name).unwrap();

			pix_str
		},
	);

	let mut output = String::new();

	// Perform Sub on u32, if panic overflow Sub, then it's bug
	let overwrite_by_other = pixels - survived - replaced;

	writeln!(&mut output, "Canvas: {}", canvas_code)?;
	writeln!(&mut output, "Users: {}", name)?;
	writeln!(&mut output, "Pixels: {}", pixels)?;
	writeln!(&mut output, "Undo: {}", undo)?;
	writeln!(&mut output, "Survivor: {}", survived)?;
	writeln!(&mut output, "Overplace by Other: {}", overwrite_by_other)?;
	writeln!(&mut output, "Self Replace: {}", replaced)?;

	writeln!(&mut output, "\nDifferent Position")?;
	writeln!(&mut output, "Place: {}", diff_pos_place)?;
	writeln!(&mut output, "Undo: {}", diff_pos_undo)?;

	writeln!(&mut output, "\n\nTop Color")?;
	write!(&mut output, "{: <PLACE_LEN$}", "Place")?;
	write!(&mut output, "{: <USED_LEN$}", "Used")?;
	write!(&mut output, "{: <PERCENT_LEN$}", "Percent")?;
	writeln!(&mut output, "{: <COLOR_LEN$}", "Color")?;
	writeln!(&mut output, "{}", sort_string)?;

	writeln!(&mut output, "\n\nExtra filter pixel")?;
	write!(&mut output, "{: <PLACE_LEN$}", "Place")?;
	write!(&mut output, "{: <MAX_POS_LEN$}", "X")?;
	write!(&mut output, "{: <MAX_POS_LEN$}", "Y")?;
	writeln!(&mut output, "{: <COLOR_LEN$}", "Color")?;
	writeln!(&mut output, "{}", to_pix_place)?;

	let stats_file_name = Path::new(OUTPUT_PATH).join(format!("C{canvas_code} Stats {name}.txt"));
	write_to_file(&stats_file_name, output.as_bytes());

	info!("Saved user stats.");
	Ok(())
}

fn extract_log(canvas_code: &str) -> Result<String> {
	let mut logs = String::new();
	info!("Reading Canvas {} logs.", canvas_code);

	let xz_logged_path =
		Path::new(INPUT_PATH).join(format!("pixels_c{canvas_code}.sanit.log.tar.xz"));
	let open_file = File::open(xz_logged_path)?;
	let buf_reader = BufReader::new(open_file);
	XzReader::new(buf_reader, true).read_to_string(&mut logs)?;

	info!("Complete Canvas {} logs ", canvas_code);
	Ok(logs)
}

fn save_img_collection(
	mut image_collection: ImageCollection,
	Settings {
		name,
		canvas_code,
		frame_delay,
		..
	}: &Settings,
	pal_vec: &PaletteVec,
) -> Result<()> {
	let blank = pal_vec.blank_index;
	let format_name = |naming: &str| -> PathBuf {
		Path::new(OUTPUT_PATH).join(format!("C{canvas_code} {name} {naming}"))
	};

	{
		let palette = pal_vec.expand_palette();

		info!("Saving placemap...");

		take(&mut image_collection.place).save_in_color(&palette, format_name("Placemap.png"))?;
		take(&mut image_collection.undo)
			.save_in_color(&palette, format_name("Placemap Undo.png"))?;
		take(&mut image_collection.survivor)
			.save_in_color(&palette, format_name("Placemap Survivor.png"))?;

		info!("Saved placemap.");
	}

	info!("Encoding animated placemap.");

	let flat_palette = pal_vec.flat_palette();
	let total_frame = image_collection.gif.len() + 2;
	let mut gif_ready: Vec<Frame> = Vec::with_capacity(total_frame);

	image_collection.gif.drain(..).for_each(
		|GrayImageCropped {
		     left,
		     top,
		     img,
		 }| {
			let width = img.width() as u16;
			let height = img.height() as u16;
			let buff = img.to_vec();

			let frame = Frame {
				delay: *frame_delay,
				dispose: DisposalMethod::Any,
				transparent: Some(blank),
				left,
				top,
				width,
				height,
				palette: Some(flat_palette.clone()),
				buffer: buff.into(),
				..Default::default()
			};

			gif_ready.push(frame);
		},
	);

	let global_palette = &pal_vec.flat_palette();
	let width = image_collection.place.width() as u16;
	let height = image_collection.place.height() as u16;

	let mut gif_file = File::create(format_name("Placemap Gif.gif"))?;
	let mut gif_encoder = gif::Encoder::new(&mut gif_file, width, height, global_palette)?;
	gif_encoder.set_repeat(Repeat::Finite(1))?;

	for frame in gif_ready.into_iter() {
		gif_encoder.write_frame(&frame)?;
	}

	info!("Encoded animated placemap.");

	Ok(())
}

fn process_place_map(
	pal_vec: &PaletteVec,
	Settings {
		user_key,
		pix_th,
		pix_per_frame,
		canvas_code,
		..
	}: &Settings,
	ImageCollection {
		place: img_placed,
		undo: img_undo,
		survivor: img_survivor,
		gif: img_gif,
	}: &mut ImageCollection,
	OutputInfo {
		pixels,
		undo,
		replaced,
		survived,
		diff_pos_place,
		diff_pos_undo,
		color_used,
		pix_place,
	}: &mut OutputInfo,
) -> Result<()> {
	let logs = extract_log(canvas_code)?;
	let logs_queue = logs.trim().split('\n');
	let blank = pal_vec.blank_index;

	let mut active_pix = blank;
	let mut old_pix = Luma([0]);
	let mut prev_lived_color = Luma([0]);
	let mut prev_process_frame = img_placed.clone();
	let mut process_frame = img_placed.clone();
	let mut last_saved_frame_backup = img_placed.clone();
	let mut last_saved_frame = img_placed.clone();

	// xy : color
	let mut vec_survivor_pix: HashMap<UVec2, Luma<u8>> = HashMap::new();

	info!("Processing logs...");

	for (at, lines) in logs_queue.into_iter().enumerate() {
		let splited: Vec<&str> = lines.split('\t').collect();
		let [date, rand_hash, x, y, color_index, action] = splited[..] else {
			error!("Invalid at line {}: {:?}", at, splited);
			continue;
		};

		let digest_format = [date, x, y, color_index, user_key].join(",");
		let digested = digest(digest_format);

		let (x, y) = (x.parse()?, y.parse()?);

		// Not The Key Owner
		if digested.encode_utf16().ne(rand_hash.encode_utf16()) {
			if action == USER_UNDO {
				let Some(old_survivor) = vec_survivor_pix.remove(&uvec2(x, y)) else {
					continue;
				};
				img_survivor.put_pixel(x, y, old_survivor);
				continue;
			}
			let old_survivor = img_survivor.get_pixel(x, y);
			vec_survivor_pix.insert(uvec2(x, y), *old_survivor);
			img_survivor.put_pixel(x, y, Luma([blank]));
			continue;
		}

		// Ignore -1
		let Some(indexed): Option<u8> = color_index.parse().ok() else {
			continue;
		};
		let luma = Luma([indexed]);

		if action == USER_UNDO {
			pix_th.contains(pixels).then(|| pix_place.pop());
			if *pixels % pix_per_frame == 0 {
				last_saved_frame = last_saved_frame_backup.clone();
				process_frame = prev_process_frame.clone();
				img_gif.pop();
			}
			(old_pix.0[0] != blank).then(|| *replaced -= 1);
			color_used.sub_used(&active_pix);
			*pixels -= 1;
			*undo += 1;

			img_placed.put_pixel(x, y, old_pix);
			process_frame.put_pixel(x, y, old_pix);
			img_survivor.put_pixel(x, y, prev_lived_color);
			img_undo.put_pixel(x, y, Luma([active_pix]));
			continue;
		}

		old_pix = *img_placed.get_pixel(x, y);

		active_pix = indexed as u8;
		color_used.add_used(&active_pix);

		img_placed.put_pixel(x, y, luma);
		process_frame.put_pixel(x, y, luma);

		// Keep previous Cordinate Pixel's [Color] before apply
		prev_lived_color = *img_survivor.get_pixel(x, y);
		(prev_lived_color.0[0] != blank).then(|| *replaced += 1);
		img_survivor.put_pixel(x, y, luma);

		*pixels += 1;
		if *pixels % pix_per_frame == 0 {
			prev_process_frame = process_frame.clone();
			let mut uncrop = process_frame.clone();
			process_frame.fill(blank);
			let (left, top) = uncrop.crop_in(blank);
			let mut cropped = GrayImageCropped::new(left, top, uncrop);
			cropped.diff_out_self(&last_saved_frame, blank);
			cropped.crop_in(blank);
			last_saved_frame_backup = last_saved_frame.clone();
			overlay(&mut last_saved_frame, &cropped.img, cropped.left as i64, cropped.top as i64);
			img_gif.push(cropped);
		}

		if pix_th.contains(pixels) {
			pix_place.push(PixelInfoAt {
				at:          *pixels,
				color_index: indexed,
				pos:         uvec2(x, y),
			});
		}
	}

	if *pixels % pix_per_frame != 0 {
		let mut uncrop = process_frame;
		let (left, top) = uncrop.crop_in(blank);
		let mut cropped = GrayImageCropped::new(left, top, uncrop);
		cropped.diff_out_self(&last_saved_frame, blank);
		cropped.crop_in(blank);
		img_gif.push(cropped);
	}

	info!("Processed logs.");

	let count_visible_pixel =
		|imaged: &GrayImage| -> u32 { imaged.pixels().filter(|x| x.0[0] != blank).count() as u32 };

	*survived = count_visible_pixel(img_survivor);
	*diff_pos_place = count_visible_pixel(img_placed);
	*diff_pos_undo = count_visible_pixel(img_undo);

	Ok(())
}

fn read_setting() -> Result<Settings> {
	let bytes_read = fs::read(SETTING_PATH)?;
	let settings = ron::de::from_bytes(&bytes_read)?;
	info!("Complete reading Setting.");
	Ok(settings)
}

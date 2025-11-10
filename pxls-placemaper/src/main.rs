use anyhow::Result;
use env_logger::Env;
use gif::{DisposalMethod, Frame, Repeat};
use image::{GenericImageView as _, GrayImage, Luma, imageops::overlay};
use log::{error, info};
use sha256::digest;
use std::{
	collections::HashMap,
	fmt::Write as _,
	fs::{self, File},
	io::prelude::*,
	path::{Path, PathBuf},
	time::Instant,
};
use xz2::read::XzDecoder;

mod structure;
use structure::*;

fn main() {
	let begin_time = Instant::now();

	let env = Env::default()
		.filter_or("MY_LOG_LEVEL", "trace")
		.write_style_or("MY_LOG_STYLE", "always");

	env_logger::init_from_env(env);

	if let Err(err) = stable_check_run() {
		error!("Unable to process: {:?}", err);
	};

	let time_taken = begin_time.elapsed();
	info!(
		"Completed Placemap\nTime Taken: {:?}\nPress 'Enter' will terminate",
		time_taken
	);

	let mut buf = String::new();
	let _ = std::io::stdin().read_line(&mut buf);
}

fn stable_check_run() -> Result<()> {
	let input_dir = Path::new("input");
	let output_dir = Path::new("output");

	let settings = read_setting()?;

	let pal_vec = PaletteVec::new(input_dir, settings.palette_code)?;

	let mut image_collection = intial_img(input_dir, &settings.canvas_code, pal_vec.blank_index)?;

	let mut output_info = OutputInfo::new(pal_vec.to_color_used());

	process_place_map(
		input_dir,
		&pal_vec,
		&settings,
		&mut image_collection,
		&mut output_info,
	)?;

	save_img_collection(&mut image_collection, output_dir, &settings, &pal_vec)?;

	create_user_stats(output_info, settings, output_dir, pal_vec)?;

	Ok(())
}

fn create_user_stats(
	output_info: OutputInfo,
	full_set_setting: Settings,
	output_dir: &Path,
	pal_vec: PaletteVec,
) -> Result<()> {
	let Settings {
		name, canvas_code, ..
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

	let sort_string: String =
		sort_color
			.iter()
			.enumerate()
			.fold(String::new(), |mut v, (rank, (a, b))| {
				writeln!(
					&mut v,
					"{}\t{}\t{:.4}\t{}",
					rank + 1,
					b,
					*b as f32 / pixels as f32 * 100.0,
					pal_vec.info[*a as usize].name
				)
				.unwrap();
				v
			});

	let total_place = pix_place.len() * 25;
	let to_pix_place = pix_place.drain(..).fold(
		String::with_capacity(total_place),
		|mut pix_str, PixelInfoAt { at, pixel, x, y }| {
			let name = pal_vec.info[pixel as usize].name.to_string();
			writeln!(&mut pix_str, "{at}\t{x}\t{y}\t{name}").unwrap();
			pix_str
		},
	);

	let make_string = format!(
		"Canvas: {}\nUsers: {}\nPixels: {}\nSurvivor: {}\nUndo: {}\nReplace: {}\n\nDifferent Position\nPlace: {}\nUndo: {}\n\nTop Color:\nPlace\tUsed\tPercent\tColor\n{}\n\nPlace\tX\tY\tColor\n{}",
		canvas_code,
		name,
		pixels,
		survived,
		undo,
		replaced,
		diff_pos_place,
		diff_pos_undo,
		sort_string,
		to_pix_place
	);

	let stats_file_name = output_dir.join(format!("C{canvas_code} Stats {name}.txt"));
	fs::write(stats_file_name, make_string)?;

	info!("Saved user stats.");
	Ok(())
}

fn extract_log(input_dir: &Path, canvas_code: &str) -> Result<String> {
	let mut logs = String::new();
	info!("Reading Canvas {} logs.", canvas_code);

	let xz_logged_path = input_dir.join(format!("pixels_c{canvas_code}.sanit.log.tar.xz"));
	let open_file = File::open(xz_logged_path)?;
	XzDecoder::new(open_file).read_to_string(&mut logs)?;

	info!("Complete Canvas {} logs ", canvas_code);
	Ok(logs)
}

fn save_img_collection(
	image_collection: &mut ImageCollection,
	output_dir: &Path,
	Settings {
		name,
		canvas_code,
		frame_delay,
		..
	}: &Settings,
	pal_vec: &PaletteVec,
) -> Result<()> {
	let blank = pal_vec.blank_index;
	let format_name =
		|naming: &str| -> PathBuf { output_dir.join(format!("C{canvas_code} {name} {naming}")) };

	{
		let palette = pal_vec.expand_palette();

		info!("Saving placemap...");

		image_collection
			.place
			.save_in_color(&palette, format_name("Placemap.png"))?;

		image_collection
			.undo
			.save_in_color(&palette, format_name("Placemap Undo.png"))?;
		image_collection
			.survivor
			.save_in_color(&palette, format_name("Placemap Survivor.png"))?;

		info!("Saved placemap.");
	}

	info!("Encoding animated placemap.");

	let flat_palette = pal_vec.flat_palette();
	let total_frame = image_collection.gif.len() + 2;
	let mut gif_ready: Vec<Frame> = Vec::with_capacity(total_frame);

	image_collection
		.gif
		.drain(..)
		.for_each(|GrayImageCropped { left, top, img }| {
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
		});

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
	input_dir: &Path,
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
	let logs = extract_log(input_dir, canvas_code)?;
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
	let mut vec_survivor_pix: HashMap<(u32, u32), Luma<u8>> = HashMap::new();

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
			if action == "user undo" {
				let Some(old_survivor) = vec_survivor_pix.remove(&(x, y)) else {
					continue;
				};
				img_survivor.put_pixel(x, y, old_survivor);
				continue;
			}
			let old_survivor = img_survivor.get_pixel(x, y);
			vec_survivor_pix.insert((x, y), *old_survivor);
			img_survivor.put_pixel(x, y, Luma([blank]));
			continue;
		}

		let indexed: u8 = color_index.parse()?;
		let luma = Luma([indexed]);

		if action == "user undo" {
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
			overlay(
				&mut last_saved_frame,
				&cropped.img,
				cropped.left as i64,
				cropped.top as i64,
			);
			img_gif.push(cropped);
		}

		if pix_th.contains(pixels) {
			pix_place.push(PixelInfoAt {
				at: *pixels,
				pixel: indexed,
				x,
				y,
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
		|imaged: &GrayImage| -> usize { imaged.pixels().filter(|x| x.0[0] != blank).count() };

	*survived = count_visible_pixel(img_survivor);
	*diff_pos_place = count_visible_pixel(img_placed);
	*diff_pos_undo = count_visible_pixel(img_undo);

	Ok(())
}

fn intial_img(input_dir: &Path, canvas_code: &str, pixel: u8) -> Result<ImageCollection> {
	let img_path = input_dir.join(format!("canvas-{canvas_code}-initial.png"));
	let (width, height) = image::open(img_path)?.dimensions();

	let img_collection = ImageCollection::new_size(width, height, pixel);

	info!("Intial Image ready");

	Ok(img_collection)
}

fn read_setting() -> Result<Settings> {
	let bytes_read = fs::read("settings.ron")?;
	let settings = ron::de::from_bytes(&bytes_read)?;
	info!("Complete reading Setting.");
	Ok(settings)
}

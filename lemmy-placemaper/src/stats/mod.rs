use anyhow::anyhow;
use image::{
	codecs::gif::{GifEncoder, Repeat},
	*,
};
use log::{error, info};
use std::{
	collections::HashMap,
	fmt::Write as _,
	fs::{self, File},
	path::{Path, PathBuf},
	time::Duration,
};

mod structure;

use structure::*;

pub fn canvas_main() -> anyhow::Result<()> {
	let input_dir = Path::new("input");
	let output_dir = Path::new("output");

	let settings = read_setting()?;

	let pal_vec = PaletteVec::new(input_dir, "2025")?;

	let mut image_collection = generate_intial_img(settings.img_size);

	let mut output_info = OutputInfo::new(pal_vec.to_color_used());

	process_place_map(
		input_dir,
		&pal_vec,
		&settings,
		&mut image_collection,
		&mut output_info,
	)?;

	save_img_collection(
		&mut image_collection,
		output_dir,
		&settings.output_file,
		&settings.http,
	)?;

	create_user_stats(output_info, settings, output_dir, &pal_vec)?;

	Ok(())
}

fn create_user_stats(
	output_info: OutputInfo,
	full_set_setting: Settings,
	output_dir: &Path,
	pal_vec: &PaletteVec,
) -> anyhow::Result<()> {
	let Settings {
		http, output_file, ..
	} = full_set_setting;
	let OutputInfo {
		pixels,
		undo,
		replaced,
		survived,
		diff_pos_place,
		diff_pos_undo,
		color_used,
		pix_place,
	} = output_info;

	info!("Creating user stats...");

	let mut sort_color: Vec<(Rgba<u8>, i32)> = color_used.0.into_iter().collect();
	sort_color.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

	let sort_string: String =
		sort_color
			.iter()
			.enumerate()
			.fold(String::new(), |mut v, (rank, (a, b))| {
				let _ = writeln!(
					&mut v,
					"{}\t{}\t{:.4}\t{}",
					rank + 1,
					b,
					*b as f32 / pixels as f32 * 100.0,
					pal_vec.0.get(a).unwrap()
				);
				v
			});

	let make_string = format!(
		"Canvas: {}\nUsers: {}\nPixels: {}\nSurvivor: {}\nUndo: {}\nReplace: {}\n\nDifferent Position\nPlace: {}\nUndo: {}\n\nTop Color:\nPlace\tUsed\tPercent\tColor\n{}\n\nPlace\tX\tY\tColor\n{}",
		output_file,
		http,
		pixels,
		survived,
		undo,
		replaced,
		diff_pos_place,
		diff_pos_undo,
		sort_string,
		pix_place.join("\n")
	);

	let last_name = http.split("/").last().unwrap();
	let stats_file_name = output_dir.join(format!("{output_file} Stats {last_name}.txt"));
	fs::write(stats_file_name, make_string)?;

	info!("Saved user stats.");
	Ok(())
}

fn extract_log(log_path: &Path) -> String {
	fs::read_to_string(log_path).expect("Fail to read log file")
}

fn save_img_collection(
	image_collection: &mut ImageCollection,
	output_dir: &Path,
	output_name: &str,
	http: &str,
) -> anyhow::Result<()> {
	image_collection.gif.push(Frame::from_parts(
		image_collection.place.clone(),
		0,
		0,
		Delay::from_saturating_duration(Duration::from_millis(3000)),
	));

	info!("Saving placemap...");
	let last_name = http.split("/").last().unwrap();
	let format_name = |naming: &str| -> PathBuf {
		output_dir.join(format!("{output_name} {last_name} {naming}"))
	};

	image_collection.place.save(format_name("Placemap.png"))?;
	image_collection
		.undo
		.save(format_name("Placemap Undo.png"))?;
	image_collection
		.survivor
		.save(format_name("Placemap Survivor.png"))?;

	info!("Saved placemap.");

	info!("Encoding animated placemap.");
	let gif_file = File::create(format_name("Placemap Gif.gif"))?;
	let mut encode_gif = GifEncoder::new(gif_file);
	encode_gif.set_repeat(Repeat::Finite(1))?;
	encode_gif.encode_frames(image_collection.gif.clone().into_iter())?;
	info!("Encoded animated placemap.");

	Ok(())
}

fn process_place_map(
	input_dir: &Path,
	pal_vec: &PaletteVec,
	Settings {
		http,
		log_file_name,
		pix_th,
		pix_per_frame,
		frame_delay,
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
) -> anyhow::Result<()> {
	let logs = extract_log(&input_dir.join(&log_file_name));
	let logs_queue = logs.trim().split('\n');

	let mut active_pix = Rgba([255; 4]);
	let mut old_pix = Rgba([0; 4]);
	let mut prev_lived_color = Rgba([0; 4]);

	// xy : color
	let mut vec_survivor_pix: HashMap<(u32, u32), Rgba<u8>> = HashMap::new();

	info!("Processing logs...");

	for (at, lines) in logs_queue.into_iter().enumerate() {
		let splited: Vec<&str> = lines.split('\t').collect();
		let [
			_id,
			log_http,
			x,
			y,
			color_hex,
			_create_at,
			_is_mod_action,
			_is_top,
			delete_at,
		] = splited[..]
		else {
			error!("Invalid at line {}: {:?}", at, splited);
			continue;
		};

		let hexed = format!("{}FF", color_hex.trim());

		let (x, y) = (x.parse()?, y.parse()?);
		let action_undo = delete_at != "\\N";

		// Not The Key Owner
		if log_http != http {
			if action_undo {
				let Some(old_survivor) = vec_survivor_pix.remove(&(x, y)) else {
					continue;
				};
				img_survivor.put_pixel(x, y, old_survivor);
				continue;
			}
			let old_survivor = img_survivor.get_pixel(x, y);
			vec_survivor_pix.insert((x, y), *old_survivor);
			img_survivor.put_pixel(x, y, Rgba([0; 4]));
			continue;
		}

		let rgba = {
			let de_hexed = hex::decode(&hexed).unwrap();
			let [r, g, b, a] = de_hexed[..] else {
				continue;
			};
			Rgba([r, g, b, a])
		};

		if action_undo {
			pix_th.contains(pixels).then(|| pix_place.pop());
			(*pixels % pix_per_frame == 0).then(|| img_gif.pop());
			(old_pix.0[3] == 255).then(|| *replaced -= 1);
			color_used.sub_used(&active_pix);
			*pixels -= 1;
			*undo += 1;

			img_placed.put_pixel(x, y, old_pix);
			img_survivor.put_pixel(x, y, prev_lived_color);
			img_undo.put_pixel(x, y, rgba);
			continue;
		}

		old_pix = *img_placed.get_pixel(x, y);

		active_pix = rgba;
		color_used.add_used(&active_pix);

		img_placed.put_pixel(x, y, rgba);

		// Keep previous Cordinate Pixel's [Color] before apply
		prev_lived_color = *img_survivor.get_pixel(x, y);
		(prev_lived_color.0[3] == 255).then(|| *replaced += 1);
		img_survivor.put_pixel(x, y, rgba);

		*pixels += 1;
		if *pixels % pix_per_frame == 0 {
			img_gif.push(Frame::from_parts(
				img_placed.clone(),
				0,
				0,
				Delay::from_saturating_duration(Duration::from_millis(*frame_delay)),
			));
		}

		if pix_th.contains(pixels) {
			let pal_name = pal_vec.0.get(&rgba).unwrap();
			pix_place.push(format!("{pixels}\t{x}\t{y}\t{}", pal_name));
		}
	}

	info!("Processed logs.");

	let count_visible_pixel = |imaged: &ImageBuffer<Rgba<u8>, Vec<u8>>| -> usize {
		imaged.pixels().filter(|x| x.0[3] == 255).count()
	};

	*survived = count_visible_pixel(img_survivor);
	*diff_pos_place = count_visible_pixel(img_placed);
	*diff_pos_undo = count_visible_pixel(img_undo);

	Ok(())
}

fn generate_intial_img(img_size: (u32, u32)) -> ImageCollection {
	let (width, height) = img_size;
	let copy_intial = RgbaImage::new(width, height);
	ImageCollection {
		place: copy_intial.clone(),
		undo: copy_intial.clone(),
		survivor: copy_intial.clone(),
		gif: vec![Frame::from_parts(
			copy_intial,
			0,
			0,
			Delay::from_saturating_duration(Duration::from_millis(500)),
		)],
	}
}

fn read_setting() -> Result<Settings, anyhow::Error> {
	let open_file = File::open("settings.ron").map_err(|_err| anyhow!("Settings.ron not exist"))?;
	let settings =
		ron::de::from_reader(open_file).map_err(|_err| anyhow!("Invalid .ron format"))?;
	info!("Complete reading Setting.");
	Ok(settings)
}

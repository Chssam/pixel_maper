use image::{
	codecs::gif::{GifEncoder, Repeat},
	*,
};
use sha256::digest;
use std::{
	collections::HashMap,
	fmt::Write as _,
	fs::{self, File},
	io::prelude::*,
	path::{Path, PathBuf},
	thread,
	time::{Duration, Instant},
};
use xz2::read::XzDecoder;

mod structure;
use structure::*;

fn main() -> anyhow::Result<()> {
	let input_dir = Path::new("input");
	let output_dir = Path::new("output");

	let begin_time = Instant::now();
	let settings = read_setting();

	let pal_vec = palette_info(input_dir, settings.palette_code);

	let mut image_collection = generate_intial_img(input_dir, &settings.canvas_code);

	let mut output_info = OutputInfo::default();

	{
		let logs = extract_log(input_dir, &settings.canvas_code);
		process_place_map(
			logs,
			&settings,
			&pal_vec,
			&mut image_collection,
			&mut output_info,
		)?;
	}

	save_img_collection(
		&mut image_collection,
		output_dir,
		&settings.canvas_code,
		&settings.name,
	)?;

	create_user_stats(output_info, settings, output_dir, &pal_vec)?;

	let time_taken = begin_time.elapsed();

	println!(
		"Completed Placemap\nTime Taken: {:?}\nAuto close in 10s",
		time_taken
	);

	thread::sleep(Duration::from_secs(10));
	Ok(())
}

fn create_user_stats(
	output_info: OutputInfo,
	full_set_setting: Settings,
	output_dir: &Path,
	pal_vec: &[PaletteInfo],
) -> anyhow::Result<()> {
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
		pix_place,
	} = output_info;

	println!("Creating user stats...");

	let mut sort_color: Vec<(i8, i32)> = color_used.0.into_iter().collect();
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
					pal_vec[*a as usize].name
				);
				v
			});

	let make_string = format!(
        "Canvas: {}\nUsers: {}\nPixels: {}\nSurvivor: {}\nUndo: {}\nReplace: {}\n\nDifferent Position\nPlace: {}\nUndo: {}\n\nTop Color:\nPlace\tUsed\tPercent\tColor\n{}\n\nPlace\tX\tY\tColor\n{}",
        canvas_code, name, pixels, survived, undo, replaced, diff_pos_place, diff_pos_undo, sort_string, pix_place.join("\n")
    );

	let stats_file_name = output_dir.join(format!("C{canvas_code} Stats {name}.txt"));
	fs::write(stats_file_name, make_string)?;

	println!("Saved user stats.");
	Ok(())
}

fn extract_log(input_dir: &Path, canvas_code: &str) -> String {
	let mut logs = String::new();
	println!("Reading Canvas {} logs.", canvas_code);

	let xz_logged = input_dir.join(format!("pixels_c{canvas_code}.sanit.log.tar.xz"));
	XzDecoder::new(File::open(xz_logged).expect("Log file not exist"))
		.read_to_string(&mut logs)
		.expect("Log file problems");

	println!("Complete Canvas {} logs ", canvas_code);
	logs
}

fn save_img_collection(
	image_collection: &mut ImageCollection,
	output_dir: &Path,
	canvas_code: &str,
	name: &str,
) -> anyhow::Result<()> {
	image_collection.gif.push(Frame::from_parts(
		image_collection.place.clone(),
		0,
		0,
		Delay::from_saturating_duration(Duration::from_millis(3000)),
	));

	println!("Saving placemap...");
	let format_name =
		|naming: &str| -> PathBuf { output_dir.join(format!("C{canvas_code} {name} {naming}")) };

	image_collection.place.save(format_name("Placemap.png"))?;
	image_collection
		.undo
		.save(format_name("Placemap Undo.png"))?;
	image_collection
		.survivor
		.save(format_name("Placemap Survivor.png"))?;
	println!("Saved placemap.");

	println!("Encoding animated placemap.");
	let gif_file = File::create(format_name("Placemap Gif.gif"))?;
	let mut encode_gif = GifEncoder::new(gif_file);
	encode_gif.set_repeat(Repeat::Finite(1))?;
	encode_gif.encode_frames(image_collection.gif.clone().into_iter())?;
	println!("Encoded animated placemap.");

	Ok(())
}

fn process_place_map(
	logs: String,
	settings: &Settings,
	pal_vec: &[PaletteInfo],
	image_collection: &mut ImageCollection,
	output_info: &mut OutputInfo,
) -> anyhow::Result<()> {
	let Settings {
		user_key,
		pix_th,
		pix_per_frame,
		frame_delay,
		..
	} = settings;

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

	let ImageCollection {
		place: img_placed,
		undo: img_undo,
		survivor: img_survivor,
		gif: img_gif,
	} = image_collection;

	let mut logs_queue: Vec<&str> = logs.trim().split('\n').collect();

	let mut active_pix = 0;

	let mut old_pix = Rgba([0; 4]);

	let mut prev_lived_color = Rgba([0; 4]);
	let mut vec_survivor_pix: HashMap<(u32, u32), Rgba<u8>> = HashMap::new();

	for (n, _) in pal_vec.iter().enumerate() {
		color_used.0.insert(n as i8, 0);
	}

	println!("Processing logs...");

	for lines in logs_queue.drain(..) {
		let splited: Vec<&str> = lines.split('\t').collect();
		let [date, rand_hash, x, y, color_index, action] = splited[..] else {
			continue;
		};

		let digest_format = [date, x, y, color_index, user_key].join(",");
		let digested = digest(digest_format.clone());

		let (x, y) = (x.parse()?, y.parse()?);

		// Not The Key Owner
		if digested.encode_utf16().ne(rand_hash.encode_utf16()) {
			if action.contains("undo") {
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

		let indexed: usize = color_index.parse()?;
		let pal_info = &pal_vec[indexed];
		let rgba = pal_info.rgba;

		if action.contains("undo") {
			pix_th.contains(pixels).then(|| pix_place.pop());
			(*pixels % pix_per_frame == 0).then(|| img_gif.pop());
			(old_pix.0[3] == 255).then(|| *replaced -= 1);
			color_used.sub_used(&active_pix);
			*pixels -= 1;
			*undo += 1;

			img_placed.put_pixel(x, y, old_pix);
			img_survivor.put_pixel(x, y, prev_lived_color);
			img_undo.put_pixel(x, y, pal_vec[active_pix as usize].rgba);
			continue;
		}

		old_pix = *img_placed.get_pixel(x, y);

		active_pix = indexed as i8;
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
			pix_place.push(format!("{pixels}\t{x}\t{y}\t{}", pal_info.name));
		}
	}

	println!("Processed logs.");

	let count_pixel = |imaged: &ImageBuffer<Rgba<u8>, Vec<u8>>| -> usize {
		imaged.pixels().filter(|x| x.0[3] == 255).count()
	};

	*survived = count_pixel(img_survivor);
	*diff_pos_place = count_pixel(img_placed);
	*diff_pos_undo = count_pixel(img_undo);

	Ok(())
}

fn generate_intial_img(input_dir: &Path, canvas_code: &str) -> ImageCollection {
	let img_path = input_dir.join(format!("canvas-{canvas_code}-initial.png"));
	let (width, height) = image::open(img_path)
		.expect("Image Path Exist")
		.dimensions();
	let copy_intial = RgbaImage::new(width, height);

	println!("Intial Image ready");

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

fn read_setting() -> Settings {
	let settings =
		ron::de::from_reader(File::open("settings.ron").expect("Settings.ron not exist"))
			.expect("Invalid .ron format");
	println!("Complete reading Setting.");
	settings
}

fn palette_info(input_dir: &Path, palette_code: u8) -> Vec<PaletteInfo> {
	let mut collect_pal = Vec::with_capacity(40);
	let palette_path = input_dir.join(format!("palette_{palette_code}_paintnet.txt"));
	let palette_ctx = fs::read_to_string(palette_path).expect("Palette not exist");

	for value in palette_ctx.lines() {
		let splited: Vec<&str> = value.trim().split(';').collect();
		let [hexy, color_name] = splited[..] else {
			continue;
		};
		if hexy.is_empty() || color_name.is_empty() {
			continue;
		}
		let hexed = hex::decode(hexy.trim()).expect("Invalid Hex code");
		let [a, r, g, b] = hexed[..] else {
			println!("Invalid ARGB");
			continue;
		};
		collect_pal.push(PaletteInfo {
			rgba: Rgba([r, g, b, a]),
			name: color_name.trim().to_owned(),
		});
	}

	collect_pal.shrink_to_fit();
	println!("Complete reading Palette {}.", palette_code);
	collect_pal
}

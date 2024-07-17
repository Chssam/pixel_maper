use image::{
	codecs::gif::{GifEncoder, Repeat},
	*,
};
use serde::Deserialize;
use std::{
	collections::HashMap,
	fs::{self, File},
	path::{Path, PathBuf},
	time::{Duration, SystemTime},
};

#[derive(Debug, Deserialize)]
struct Settings {
	user: String,
	output_file: String,
	log_file_name: String,
	pix_th: Vec<u32>,
	pix_per_frame: u32,
	frame_delay: u64,
	img_size: (u32, u32),
}

struct ImageCollection {
	place: ImageBuffer<Rgba<u8>, Vec<u8>>,
	undo: ImageBuffer<Rgba<u8>, Vec<u8>>,
	survivor: ImageBuffer<Rgba<u8>, Vec<u8>>,
	gif: Vec<Frame>,
}

#[derive(Default)]
struct OutputInfo {
	pixels: u32,
	undo: u32,
	replaced: u32,
	survived: usize,
	diff_pos_place: usize,
	diff_pos_undo: usize,
	color_use: HashMap<Rgba<u8>, i32>,
	pix_place: Vec<String>,
}

fn main() -> anyhow::Result<()> {
	let input_dir = Path::new("input");
	let output_dir = Path::new("output");

	let begin_time = SystemTime::now();
	let settings = read_setting();

	let mut image_collection = generate_intial_img(settings.img_size);

	let mut output_info = OutputInfo::default();
	{
		let logs = extract_log(&input_dir.join(&settings.log_file_name));
		process_place_map(logs, &settings, &mut image_collection, &mut output_info)?;
	}

	save_img_collection(&mut image_collection, output_dir, &settings.output_file)?;

	create_user_stats(output_info, settings, output_dir)?;

	let time_taken = begin_time.elapsed()?;
	println!("Time Taken: {:?}\nCompleted Placemap", time_taken);
	Ok(())
}

fn create_user_stats(
	output_info: OutputInfo,
	full_set_setting: Settings,
	output_dir: &Path,
) -> anyhow::Result<()> {
	let Settings { output_file, .. } = full_set_setting;
	let OutputInfo {
		pixels,
		undo,
		replaced,
		survived,
		diff_pos_place,
		diff_pos_undo,
		color_use,
		pix_place,
	} = output_info;
	println!("Creating user stats...");
	let mut sort_color: Vec<(Rgba<u8>, i32)> = color_use.into_iter().collect();
	sort_color.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
	let sort_string: String = sort_color
		.iter()
		.enumerate()
		.map(|(rank, (a, b))| {
			format!(
				"{}\t{}\t{:.4}\t{}\n",
				rank + 1,
				b,
				*b as f32 / pixels as f32 * 100.0,
				hex::encode(a.0)
			)
		})
		.collect();
	let make_string = format!(
        "Users: {}\nPixels: {}\nSurvivor: {}\nUndo: {}\nReplace: {}\n\nDifferent Position\nPlace: {}\nUndo: {}\n\nTop Color:\nPlace\tUsed\tPercent\tColor\n{}\n\nPlace\tX\tY\tColor\n{}",
        output_file, pixels, survived, undo, replaced, diff_pos_place, diff_pos_undo, sort_string, pix_place.join("\n")
    );
	let stats_file_name = output_dir.join(format!("{output_file} Stats.txt"));
	fs::write(stats_file_name, make_string)?;
	println!("Saved user stats.");
	Ok(())
}

fn extract_log(log_path: &Path) -> String {
	fs::read_to_string(log_path).expect("Fail to read log file")
}

fn save_img_collection(
	image_collection: &mut ImageCollection,
	output_dir: &Path,
	name: &str,
) -> anyhow::Result<()> {
	image_collection.gif.push(Frame::from_parts(
		image_collection.place.clone(),
		0,
		0,
		Delay::from_saturating_duration(Duration::from_millis(3000)),
	));

	println!("Saving placemap...");
	let format_name = |naming: &str| -> PathBuf { output_dir.join(format!("{name} {naming}")) };

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
	encode_gif.set_repeat(Repeat::Infinite)?;
	encode_gif.encode_frames(image_collection.gif.clone().into_iter())?;
	println!("Encoded animated placemap.");
	Ok(())
}

fn process_place_map(
	logs: String,
	settings: &Settings,
	image_collection: &mut ImageCollection,
	output_info: &mut OutputInfo,
) -> anyhow::Result<()> {
	let Settings {
		user,
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
		color_use,
		pix_place,
	} = output_info;
	let ImageCollection {
		place: img_placed,
		undo: img_undo,
		survivor: img_survivor,
		gif: img_gif,
	} = image_collection;
	let mut logs_queue: Vec<&str> = logs.trim().split('\n').collect();
	logs_queue.dedup();
	let mut old_pix = Rgba([0; 4]);
	let mut active_pix = Rgba([0; 4]);
	let mut previous_pix_survivor_color = Rgba([0; 4]);

	let mut vec_survivor_pix: HashMap<(u32, u32), Rgba<u8>> = HashMap::new();

	println!("Processing logs...");
	for lines in logs_queue.drain(..) {
		let splited: Vec<&str> = lines.split('\t').collect();
		let [_date, user_mail, action, x, y, color_hex] = splited[..] else {
			continue;
		};

		let hexed = format!("{}FF", color_hex.trim());
		let (x, y) = (x.parse()?, y.parse()?);

		if !user.contains(&user_mail.to_owned()) {
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

		if action.contains("undo") {
			pix_th.contains(pixels).then(|| pix_place.pop());
			(*pixels % pix_per_frame == 0).then(|| img_gif.pop());
			(old_pix.0[3] == 255).then(|| *replaced -= 1);
			*color_use.get_mut(&active_pix).unwrap() -= 1;
			*pixels -= 1;
			*undo += 1;

			img_placed.put_pixel(x, y, old_pix);
			img_survivor.put_pixel(x, y, previous_pix_survivor_color);
			img_undo.put_pixel(x, y, active_pix);
			continue;
		}

		let rgba = {
			let de_hexed = hex::decode(&hexed).unwrap();
			let [r, g, b, a] = de_hexed[..] else {
				continue;
			};
			Rgba([r, g, b, a])
		};

		*pixels += 1;
		active_pix = rgba;
		old_pix = *img_survivor.get_pixel(x, y);
		(old_pix.0[3] == 255).then(|| *replaced += 1);

		color_use
			.entry(active_pix)
			.and_modify(|v| *v += 1)
			.or_insert(1);

		img_placed.put_pixel(x, y, rgba);

		previous_pix_survivor_color = *img_survivor.get_pixel(x, y);
		img_survivor.put_pixel(x, y, rgba);
		if *pixels % pix_per_frame == 0 {
			img_gif.push(Frame::from_parts(
				img_placed.clone(),
				0,
				0,
				Delay::from_saturating_duration(Duration::from_millis(*frame_delay)),
			));
		}
		if pix_th.contains(pixels) {
			pix_place.push(format!("{pixels}\t{x}\t{y}\t{}", color_hex));
		}
	}
	println!("Processed logs.");
	*survived = count_pixel(img_survivor);
	*diff_pos_place = count_pixel(img_placed);
	*diff_pos_undo = count_pixel(img_undo);
	Ok(())
}

fn count_pixel(imaged: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> usize {
	imaged.pixels().filter(|x| x.0[3] == 255).count()
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

fn read_setting() -> Settings {
	let settings =
		ron::de::from_reader(File::open("settings.ron").expect("Settings.ron not exist"))
			.expect("Invalid .ron format");
	println!("Complete reading Setting.");
	settings
}

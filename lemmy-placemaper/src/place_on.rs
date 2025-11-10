use std::{
	collections::HashMap,
	fs::{self},
	path::Path,
};

use image::{DynamicImage, GenericImageView, Rgba};

#[derive(Default, Debug)]
struct CollectionRank(HashMap<String, i32>);

pub fn place_on_main() -> anyhow::Result<()> {
	let input_dir = Path::new("input");
	let output_dir = Path::new("output");
	let image_ref = image::open(input_dir.join("template.png"))?;

	let mut collection_rank = CollectionRank::default();
	let logs = extract_log(&input_dir.join("pixels-patched.txt"));
	process_place_map(logs, &mut collection_rank, image_ref)?;

	create_user_stats(collection_rank, output_dir)?;
	Ok(())
}

fn create_user_stats(collection_rank: CollectionRank, output_dir: &Path) -> anyhow::Result<()> {
	let mut sort_with_placed: Vec<(String, i32)> = collection_rank.0.into_iter().collect();
	sort_with_placed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
	let ranked: Vec<String> = sort_with_placed
		.into_iter()
		.enumerate()
		.filter_map(|(n, x)| {
			if x.1 <= 0 {
				return None;
			}
			let mut fix_length = x.0.clone();
			fix_length.push_str(&" ".repeat(50 - x.0.len()));
			Some(format!("{}\t{}\t{}", n + 1, fix_length, x.1))
		})
		.collect();
	let make_string = format!("Place\tInstace\t\t\t\t\t\t\tPlaced\n{}", ranked.join("\n"));
	let stats_file_name = output_dir.join("Pixel On Ponies 2024.txt");
	fs::write(stats_file_name, make_string)?;
	println!("Saved user stats.");
	Ok(())
}

fn extract_log(log_path: &Path) -> String {
	fs::read_to_string(log_path).expect("Fail to read log file")
}

fn process_place_map(
	logs: String,
	collection_rank: &mut CollectionRank,
	image_ref: DynamicImage,
) -> anyhow::Result<()> {
	let mut logs_queue: Vec<&str> = logs.trim().split('\n').collect();
	logs_queue.dedup();

	let mut prev_color: HashMap<String, Rgba<u8>> = HashMap::new();

	for lines in logs_queue.drain(..) {
		let splited: Vec<&str> = lines.split('\t').collect();
		let [_date, user_mail, action, x, y, color_hex] = splited[..] else {
			continue;
		};
		let (x, y) = (x.parse()?, y.parse()?);
		let hexed = format!("{}FF", color_hex.trim());

		let rgba = match hex::decode(&hexed) {
			Ok(value) => {
				let [r, g, b, a] = value[..] else {
					continue;
				};
				Rgba([r, g, b, a])
			},
			Err(_err) => {
				// println!("Invalid Hex: {:?} : {}", err, hexed);
				continue;
			},
		};
		let pixels = collection_rank.0.entry(user_mail.to_owned()).or_insert(0);

		let img_rgba = image_ref.get_pixel(x, y);

		if rgba == img_rgba && !action.contains("undo") {
			*pixels += 1;
			prev_color
				.entry(user_mail.to_owned())
				.and_modify(|x| *x = rgba)
				.or_insert(rgba);
		} else {
			let Some(prev_pix) = prev_color.get(user_mail) else {
				continue;
			};
			(prev_pix == &img_rgba).then(|| *pixels -= 1);
		}
	}
	Ok(())
}

use log::{error, info};
use std::{
	fs::{self},
	path::Path,
};

pub fn sort_main() -> anyhow::Result<()> {
	let input_dir = Path::new("input");
	let output_dir = Path::new("output");

	process_data(input_dir, output_dir)?;

	Ok(())
}

// fn process_data(input_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
// 	let logs = extract_log(&input_dir.join("database edit.txt"));
// 	let mut logs_queue = logs.trim().lines().collect::<Vec<_>>();

// 	info!("Processing logs...");

// 	let mut at = 0;
// 	logs_queue = logs_queue
// 		.iter()
// 		.filter_map(|lines| {
// 			let splited: Vec<&str> = lines.split('\t').collect();
// 			let [
// 				id_1,
// 				_log_http,
// 				_x,
// 				_y,
// 				_color_hex,
// 				_create_at,
// 				_is_mod_action,
// 				_is_top,
// 				_delete_at,
// 			] = splited[..]
// 			else {
// 				error!("Invalid at line {}: {:?}", at, splited);
// 				return None;
// 			};
// 			at += 1;
// 			Some(*lines)
// 		})
// 		.collect::<Vec<_>>();

// 	logs_queue.sort_by(|&lines_1, &line_2| {
// 		let splited: Vec<&str> = lines_1.split('\t').collect();
// 		let [
// 			id_1,
// 			_log_http,
// 			_x,
// 			_y,
// 			_color_hex,
// 			_create_at,
// 			_is_mod_action,
// 			_is_top,
// 			_delete_at,
// 		] = splited[..]
// 		else {
// 			unreachable!();
// 		};

// 		let splited: Vec<&str> = line_2.split('\t').collect();
// 		let [
// 			id_2,
// 			_log_http,
// 			_x,
// 			_y,
// 			_color_hex,
// 			_create_at,
// 			_is_mod_action,
// 			_is_top,
// 			_delete_at,
// 		] = splited[..]
// 		else {
// 			unreachable!();
// 		};

// 		let a_1 = id_1.parse::<u64>().unwrap();
// 		let a_2 = id_2.parse::<u64>().unwrap();

// 		a_1.cmp(&a_2)
// 	});

// 	let mut ensure = 1;

// 	for lines in logs_queue.iter() {
// 		let splited: Vec<&str> = lines.split('\t').collect();
// 		let [
// 			id,
// 			_log_http,
// 			_x,
// 			_y,
// 			_color_hex,
// 			_create_at,
// 			_is_mod_action,
// 			_is_top,
// 			_delete_at,
// 		] = splited[..]
// 		else {
// 			unreachable!()
// 		};
// 		let id_parse = id.parse::<u64>().unwrap();
// 		if id_parse != ensure {
// 			println!("Incorrect at {ensure}");
// 			break;
// 		}
// 		ensure += 1;
// 	}

// 	info!("Processed logs.");

// 	let sorted_file = output_dir.join(format!("Sorted Database.txt"));
// 	fs::write(sorted_file, logs_queue.join("\n"))?;

// 	Ok(())
// }

fn process_data(input_dir: &Path, output_dir: &Path) -> anyhow::Result<()> {
	let logs = extract_log(&input_dir.join("Sorted Database.txt"));
	let logs_queue = logs.trim().lines();

	info!("Processing logs...");

	let the_http = "https://toast.ooo/u/starpup";
	let new_log = logs_queue
		.into_iter()
		.filter_map(|lines| {
			let splited: Vec<&str> = lines.split('\t').collect();
			let [
				_id,
				log_http,
				x,
				y,
				_color_hex,
				_create_at,
				_is_mod_action,
				is_top,
				_delete_at,
			] = splited[..]
			else {
				unreachable!()
			};
			// let (x, y) = (x.parse::<u32>().unwrap(), y.parse::<u32>().unwrap());
			// let top_left = x > 467 && y > 180;
			// let bottom_right = x < 489 && y < 212;
			let is_top = is_top == "t";
			let hoof = log_http == the_http && is_top;
			hoof.then(|| lines)
		})
		.collect::<Vec<_>>();

	println!("Total: {}", new_log.len());

	info!("Processed logs.");

	let sorted_file = output_dir.join(format!("Starpup Top.txt"));
	fs::write(sorted_file, new_log.join("\n"))?;

	Ok(())
}

fn extract_log(log_path: &Path) -> String {
	fs::read_to_string(log_path).expect("Fail to read log file")
}

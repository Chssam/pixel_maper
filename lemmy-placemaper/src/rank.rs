use std::{
	collections::HashMap,
	fs::{self},
	path::Path,
};

#[derive(Default, Debug)]
struct CollectionRank(HashMap<String, OutputInfoPerUser>);

#[derive(Default, Debug)]
struct OutputInfoPerUser {
	pixels: i16,
	undo: i16,
}

pub fn rank_main() -> anyhow::Result<()> {
	let input_dir = Path::new("input");
	let output_dir = Path::new("output");

	let mut collection_rank = CollectionRank::default();
	{
		let logs = extract_log(&input_dir.join("Sorted Database.txt"));
		process_place_map(logs, &mut collection_rank)?;
	}

	create_user_stats(collection_rank, output_dir)?;
	Ok(())
}

fn create_user_stats(collection_rank: CollectionRank, output_dir: &Path) -> anyhow::Result<()> {
	let mut sort_with_placed: Vec<(String, OutputInfoPerUser)> =
		collection_rank.0.into_iter().collect();
	sort_with_placed.sort_by(|a, b| b.1.pixels.partial_cmp(&a.1.pixels).unwrap());
	let ranked: Vec<String> = sort_with_placed
		.into_iter()
		.enumerate()
		.map(|(n, (user_mail, output))| {
			let mut fix_length = user_mail.clone();
			fix_length.push_str(&" ".repeat(70 - user_mail.len()));
			format!(
				"{}\t{}\t{}\t{}",
				n + 1,
				fix_length,
				output.pixels,
				output.undo
			)
		})
		.collect();
	let make_string = format!(
		"Place\tInstace\t\t\t\t\t\t\t\t\tPlaced\tUndo\n{}",
		ranked.join("\n")
	);
	let stats_file_name = output_dir.join(format!("Ranking Lemmy 2025.txt"));
	fs::write(stats_file_name, make_string)?;
	println!("Saved user stats.");
	Ok(())
}

fn extract_log(log_path: &Path) -> String {
	fs::read_to_string(log_path).expect("Fail to read log file")
}

fn process_place_map(logs: String, collection_rank: &mut CollectionRank) -> anyhow::Result<()> {
	let logs_queue = logs.trim().split('\n');

	for lines in logs_queue.into_iter() {
		let splited: Vec<&str> = lines.split('\t').collect();
		// id, "userId", x, y, color, "createdAt", "isModAction", "isTop", "deletedAt"
		let [
			_id,
			user_mail,
			_x,
			_y,
			_color_hex,
			_create_at,
			is_mod_action,
			_is_top,
			delete_at,
		] = splited[..]
		else {
			continue;
		};

		let OutputInfoPerUser { pixels, undo } =
			collection_rank.0.entry(user_mail.to_owned()).or_default();

		if is_mod_action == "t" {
			println!("Mod: {is_mod_action}\tUser: {user_mail}");
			continue;
		}
		if delete_at != "\\N" {
			*pixels -= 1;
			*undo += 1;
			continue;
		}
		*pixels += 1;
	}
	println!("Done");
	Ok(())
}

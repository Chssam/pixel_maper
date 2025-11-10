use std::{io::Write as _, time::Instant};

use env_logger::Env;
use log::error;

mod place_on;
mod rank;
mod sort;
mod stats;

use place_on::place_on_main;
use rank::rank_main;
use sort::sort_main;
use stats::canvas_main;

fn main() {
	let env = Env::default()
		.filter_or("MY_LOG_LEVEL", "trace")
		.write_style_or("MY_LOG_STYLE", "always");

	env_logger::init_from_env(env);

	let begin_time = Instant::now();

	let mut lines = std::io::stdin().lines();
	let _ = std::io::stdout().flush();
	let Some(Ok(line)) = lines.next() else {
		return;
	};

	if line.is_empty() {
		return;
	};

	if let Err(err) = stable_check_run(&line) {
		error!("Unable to process: {:?}", err);
	};

	let time_taken = begin_time.elapsed();
	println!(
		"Completed Placemap\nTime Taken: {:?}\nPress 'Enter' will terminate",
		time_taken
	);

	let mut buf = String::new();
	let _ = std::io::stdin().read_line(&mut buf);
}

fn stable_check_run(cmd: &str) -> Result<(), anyhow::Error> {
	match cmd {
		"place" => place_on_main()?,
		"canvas" => canvas_main()?,
		"rank" => rank_main()?,
		"sort" => sort_main()?,
		_ => {},
	}
	Ok(())
}

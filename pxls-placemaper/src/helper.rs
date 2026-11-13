use std::path::PathBuf;

use tracing::{
	error,
	warn,
};

pub fn write_to_file(path: &PathBuf, byted: &[u8]) {
	if let Err(err) = std::fs::write(path, byted) {
		warn!(
			"Fail to save path: {:#?}\nPath: {:?}\nCreating directory.",
			err,
			path.file_name()
		);
		let mut dir_path = path.clone();
		dir_path.pop();
		if let Err(err) = std::fs::create_dir_all(path) {
			error!("Fail to create path: {:?}", err);
		};
	}
}

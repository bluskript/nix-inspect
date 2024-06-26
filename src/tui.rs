use crossterm::{
	terminal::{disable_raw_mode, LeaveAlternateScreen},
	ExecutableCommand,
};
use std::{io::stdout, panic};

pub fn install_panic_hook() {
	let original_hook = panic::take_hook();
	panic::set_hook(Box::new(move |panic_info| {
		stdout().execute(LeaveAlternateScreen).unwrap();
		disable_raw_mode().unwrap();
		original_hook(panic_info);
	}));
}

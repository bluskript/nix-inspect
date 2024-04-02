use std::{env, fs::File, io::Read, path::PathBuf, sync::Arc};

use key_handler::register_key_handler;
use logging::{initialize_logging, project_directory};
use model::{Bookmark, BrowserPath, BrowserStack, BrowserStackItem, Message, Model, RunningState};
use parking_lot::RwLock;
use ratatui::widgets::ListState;
use serde::{Deserialize, Serialize};
use update::UpdateContext;
use view::view;
use workers::WorkerHost;

use crate::view::ViewData;

pub mod key_handler;
pub mod logging;
pub mod model;
pub mod tui;
pub mod update;
pub mod view;
pub mod workers;

#[derive(Serialize, Deserialize, Default)]
pub struct Config {
	bookmarks: Option<Vec<Bookmark>>,
}

pub fn read_config(p: PathBuf) -> anyhow::Result<Config> {
	let config = std::fs::read_to_string(p)?;
	let cfg: Config = serde_json::from_str(&config)?;
	Ok(cfg)
}

fn main() -> color_eyre::Result<()> {
	let mut terminal = tui::init_terminal()?;
	initialize_logging()?;
	tui::install_panic_hook();

	let project_dirs = &project_directory().unwrap();
	let config = read_config(project_dirs.config_local_dir().join("config.json"))
		.unwrap_or(Config::default());

	// Same behavior as nixos-rebuild
	let mut hostname_file = File::open("/proc/sys/kernel/hostname")?;
	let mut hostname = String::new();
	hostname_file.read_to_string(&mut hostname)?;

	let hostname_path = format!("nixosConfigurations.{}", hostname.trim());

	let user = env::var("USER")?;
	let user_path = format!("{hostname_path}.config.home-manager.users.{user}");

	tracing::debug!("{:?} {:?}", hostname_path, user_path);

	let worker_host = WorkerHost::new();
	let model = Arc::new(RwLock::new(Model {
		running_state: RunningState::Running,
		visit_stack: BrowserStack(vec![BrowserStackItem::Root]),
		bookmarks: config.bookmarks.unwrap_or(vec![
			Bookmark {
				display: hostname.clone(),
				path: BrowserPath::from(hostname_path),
			},
			Bookmark {
				display: user.to_string(),
				path: BrowserPath::from(user_path.to_string()),
			},
		]),
		root_view_state: ListState::default().with_selected(Some(0)),
		bookmark_view_state: ListState::default().with_selected(Some(0)),
		..Default::default()
	}));

	let mut update_context = UpdateContext {
		req_tx: worker_host.tx.clone(),
	};

	let (tx, rx) = kanal::unbounded::<Message>();
	register_key_handler(&tx, Arc::clone(&model));

	{
		let worker_rx = worker_host.rx.clone();
		std::thread::spawn(move || loop {
			match worker_rx.recv() {
				Ok((p, v)) => {
					let _ = tx.send(Message::Data(p, v.into()));
				}
				Err(_) => break,
			}
		});
	}

	while model.read().running_state != RunningState::Stopped {
		// Render the current view
		let mut view_data: ViewData = ViewData::default();
		terminal.draw(|f| {
			view_data = view(&model.read(), f);
		})?;

		let mut current_msg = Some(rx.recv()?);

		// Process updates as long as they return a non-None message
		while let Some(msg) = current_msg {
			tracing::info!("{:?}", msg);
			if let Ok(msg) = update_context.update(&view_data, &mut model.write(), msg) {
				current_msg = msg;
			} else {
				current_msg = None;
			}
		}
	}

	tui::restore_terminal()?;

	Ok(())
}

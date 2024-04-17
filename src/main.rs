use std::{
	env,
	fs::{create_dir_all, File},
	io::{stdout, Read},
	path::{Path, PathBuf},
	sync::Arc,
};

use clap::Parser;
use color_eyre::eyre::OptionExt;
use crossterm::{
	terminal::{
		disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen,
		SetSize,
	},
	ExecutableCommand,
};
use key_handler::register_key_handler;
use logging::{initialize_logging, project_directory};
use model::{Bookmark, BrowserPath, BrowserStack, BrowserStackItem, Message, Model, RunningState};
use ratatui::{backend::CrosstermBackend, widgets::ListState, Terminal};
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

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Config {
	bookmarks: Vec<Bookmark>,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
	#[arg(short, long)]
	path: Option<String>,
	#[arg(short, long)]
	expr: Option<String>,
}

pub fn find_in_nix_path() -> color_eyre::Result<String> {
	let x = env::var("NIX_PATH")?;
	Ok(x.split(":")
		.filter_map(|x| x.split_once("="))
		.find(|(x, _)| x == &"nixos-config")
		.ok_or_eyre("nixos-config not found in path")?
		.1
		.to_string())
}

fn load_config(args: &Args) -> color_eyre::Result<String> {
	if let Some(expr) = &args.expr {
		Ok(expr.to_string())
	} else if let Some(path) = &args.path {
		let path = Path::new(path).canonicalize()?;
		let is_file = path.is_file();
		let is_flake = is_file && path.ends_with("flake.nix") || path.join("flake.nix").exists();

		Ok(if is_flake {
			format!(r#"builtins.getFlake "{}""#, path.display())
		} else {
			format!("(import <nixpkgs/nixos>) {{ system = builtins.currentSystem; configuration = import {}; }}", path.display())
		})
	} else {
		let nixos_path = Path::new("/etc/nixos").canonicalize()?;
		let etc_nixos_flake = nixos_path.join("flake.nix");
		if etc_nixos_flake.exists() {
			Ok(format!(r#"builtins.getFlake "{}""#, nixos_path.display()))
		} else {
			let path = find_in_nix_path().unwrap_or("/etc/nixos".to_string());
			let path = Path::new(&path).canonicalize()?;
			Ok(format!("(import <nixpkgs/nixos>) {{ system = builtins.currentSystem; configuration = import {}; }}", path.display()))
		}
	}
}

pub fn read_config(p: PathBuf) -> anyhow::Result<Config> {
	let config = std::fs::read_to_string(p)?;
	let cfg: Config = serde_json::from_str(&config)?;
	Ok(cfg)
}

fn main() -> color_eyre::Result<()> {
	let args = Args::parse();
	let (cols, rows) = size()?;
	enable_raw_mode()?;
	stdout().execute(EnterAlternateScreen)?;
	let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
	initialize_logging()?;
	tui::install_panic_hook();

	let project_dirs = &project_directory().unwrap();
	let config_path = project_dirs.config_local_dir().join("config.json");
	let config = if let Ok(c) = read_config(config_path.clone()) {
		c
	} else {
		// Same behavior as nixos-rebuild
		let hostname = nix::unistd::gethostname()?;
		let hostname = hostname.to_string_lossy();
		let hostname_path = format!(".nixosConfigurations.{}", hostname);

		let user = env::var("USER")?;
		let user_path = format!("{hostname_path}.config.home-manager.users.{user}");

		let config = Config {
			bookmarks: vec![
				Bookmark {
					display: hostname.to_string(),
					path: BrowserPath::from(hostname_path),
				},
				Bookmark {
					display: user.to_string(),
					path: BrowserPath::from(user_path.to_string()),
				},
			],
		};
		create_dir_all(config_path.parent().unwrap())?;
		let x = serde_json::to_string_pretty(&config)?;
		std::fs::write(config_path.clone(), x)?;

		config
	};

	let expr = load_config(&args)?;
	tracing::debug!("{}", expr);

	let worker_host = WorkerHost::new(expr);
	let mut model = Model {
		running_state: RunningState::Running,
		visit_stack: BrowserStack(vec![BrowserStackItem::Root]),
		root_view_state: ListState::default().with_selected(Some(0)),
		bookmark_view_state: ListState::default().with_selected(Some(0)),
		config,
		..Default::default()
	};

	let mut update_context = UpdateContext {
		req_tx: worker_host.tx.clone(),
		config_path,
	};

	let (tx, rx) = kanal::unbounded::<Message>();
	register_key_handler(&tx);

	{
		let worker_rx = worker_host.rx.clone();
		std::thread::spawn(move || loop {
			match worker_rx.recv() {
				Ok((p, v)) => {
					let _ = tx.send(Message::Data(p, v));
				}
				Err(_) => break,
			}
		});
	}

	while model.running_state != RunningState::Stopped {
		// Render the current view
		let mut view_data: ViewData = ViewData::default();
		terminal.draw(|f| {
			view_data = view(&mut model, f);
		})?;

		let mut current_msg = Some(rx.recv()?);

		// Process updates as long as they return a non-None message
		while let Some(msg) = current_msg {
			tracing::info!("{:?}", msg);
			if let Ok(msg) = update_context.update(&view_data, &mut model, msg) {
				current_msg = msg;
			} else {
				current_msg = None;
			}
		}
	}

	stdout().execute(LeaveAlternateScreen)?;
	disable_raw_mode()?;
	stdout().execute(SetSize(cols, rows))?;

	Ok(())
}

use std::sync::Arc;

use key_handler::register_key_handler;
use logging::initialize_logging;
use model::{Bookmark, BrowserPath, BrowserStack, BrowserStackItem, Message, Model, RunningState};
use parking_lot::RwLock;
use ratatui::widgets::ListState;
use update::UpdateContext;
use view::view;
use workers::WorkerHost;

pub mod key_handler;
pub mod logging;
pub mod model;
pub mod tui;
pub mod update;
pub mod view;
pub mod workers;

fn main() -> color_eyre::Result<()> {
	let mut terminal = tui::init_terminal()?;
	initialize_logging()?;
	tui::install_panic_hook();
	let worker_host = WorkerHost::new();
	let model = Arc::new(RwLock::new(Model {
		running_state: RunningState::Running,
		visit_stack: BrowserStack(vec![BrowserStackItem::Root]),
		bookmarks: vec![Bookmark {
			display: "blusk".to_string(),
			path: BrowserPath::from(
				"nixosConfigurations.felys.config.home-manager.users.blusk".to_string(),
			),
		}],
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
		terminal.draw(|f| view(&model.read(), f))?;

		let mut current_msg = Some(rx.recv()?);

		// Process updates as long as they return a non-None message
		while let Some(msg) = current_msg {
			tracing::info!("{:?}", msg);
			if let Ok(msg) = update_context.update(&mut model.write(), msg) {
				current_msg = msg;
			} else {
				current_msg = None;
			}
		}
	}

	tui::restore_terminal()?;

	Ok(())
}

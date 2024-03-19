use std::{sync::Arc, time::Duration};

use crossterm::event::{self, Event, KeyCode};
use parking_lot::RwLock;

use crate::model::{InputState, Message, Model};

pub fn register_key_handler(tx: &kanal::Sender<Message>, model: Arc<RwLock<Model>>) {
	let tx = tx.clone();
	std::thread::spawn(move || -> anyhow::Result<()> {
		loop {
			if let Ok(true) = event::poll(Duration::from_millis(100)) {
				if let Event::Key(key) = event::read()? {
					if key.kind == event::KeyEventKind::Press {
						if let Some(msg) = handle_key(key, &model.read()) {
							let _ = tx.send(msg);
						}
					}
				}
			}
		}
	});
}

pub fn handle_key(key: event::KeyEvent, model: &Model) -> Option<Message> {
	if let InputState::Active(_) = model.search_input {
		handle_search_input(key)
	} else if let InputState::Active(_) = model.path_navigator_input {
		handle_navigator_input(key)
	} else {
		handle_normal_input(key)
	}
}

fn handle_search_input(key: event::KeyEvent) -> Option<Message> {
	match key.code {
		KeyCode::Esc => Some(Message::SearchExit),
		KeyCode::Enter => Some(Message::Enter),
		_ => Some(Message::SearchInput(key)),
	}
}

fn handle_navigator_input(key: event::KeyEvent) -> Option<Message> {
	match key.code {
		KeyCode::Esc => Some(Message::NavigatorExit),
		KeyCode::Enter => Some(Message::Enter),
		_ => Some(Message::NavigatorInput(key)),
	}
}

fn handle_normal_input(key: event::KeyEvent) -> Option<Message> {
	match key.code {
		KeyCode::Char('q') => Some(Message::Quit),
		KeyCode::Char('h') | KeyCode::Left => Some(Message::Back),
		KeyCode::Char('j') | KeyCode::Down => Some(Message::ListDown),
		KeyCode::Char('k') | KeyCode::Up => Some(Message::ListUp),
		KeyCode::Char('l') | KeyCode::Right => Some(Message::Enter),
		KeyCode::Char('f') | KeyCode::Char('/') => Some(Message::SearchEnter),
		KeyCode::Char('.') => Some(Message::NavigatorEnter),
		_ => None,
	}
}

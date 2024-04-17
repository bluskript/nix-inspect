use std::time::Duration;

use crossterm::event::{self, KeyCode, KeyModifiers};

use crate::model::{InputModel, InputState, Message, Model};

pub fn register_key_handler(tx: &kanal::Sender<Message>) {
	let tx = tx.clone();
	std::thread::spawn(move || -> anyhow::Result<()> {
		loop {
			if let Ok(true) = event::poll(Duration::from_millis(100)) {
				let _ = tx.send(Message::TermEvent(event::read()?));
			}
		}
	});
}

pub fn handle_key(key: event::KeyEvent, model: &Model) -> Option<Message> {
	if let InputState::Active(state) = &model.search_input {
		handle_search_input(state, key)
	} else if let InputState::Active(state) = &model.path_navigator_input {
		handle_navigator_input(state, key)
	} else if let InputState::Active(state) = &model.new_bookmark_input {
		handle_bookmark_input(state, key)
	} else {
		handle_normal_input(key)
	}
}

fn handle_search_input(state: &InputModel, key: event::KeyEvent) -> Option<Message> {
	if !state.typing {
		match key.code {
			KeyCode::Char('n') => Some(Message::SearchNext),
			KeyCode::Char('N') => Some(Message::SearchPrev),
			KeyCode::Esc => Some(Message::SearchExit),
			_ => None,
		}
	} else {
		match key.code {
			KeyCode::Esc => Some(Message::SearchExit),
			_ => Some(Message::SearchInput(key)),
		}
	}
}

pub fn handle_bookmark_input(_: &InputModel, key: event::KeyEvent) -> Option<Message> {
	match key.code {
		KeyCode::Esc => Some(Message::BookmarkInputExit),
		KeyCode::Enter => Some(Message::CreateBookmark),
		_ => Some(Message::BookmarkInput(key)),
	}
}

pub fn handle_navigator_input(state: &InputModel, key: event::KeyEvent) -> Option<Message> {
	if !state.typing {
		match key.code {
			KeyCode::Char('n') => Some(Message::NavigatorNext),
			KeyCode::Char('N') => Some(Message::NavigatorPrev),
			KeyCode::Esc => Some(Message::NavigatorExit),
			_ => Some(Message::NavigatorInput(key)),
		}
	} else {
		match key.code {
			KeyCode::Esc => Some(Message::NavigatorExit),
			_ => Some(Message::NavigatorInput(key)),
		}
	}
}

pub fn handle_normal_input(key: event::KeyEvent) -> Option<Message> {
	match key.code {
		KeyCode::Char('q') => Some(Message::Quit),
		KeyCode::Char('h') | KeyCode::Left => Some(Message::Back),
		KeyCode::Char('j') | KeyCode::Down => Some(Message::ListDown),
		KeyCode::Char('k') | KeyCode::Up => Some(Message::ListUp),
		KeyCode::Char('l') | KeyCode::Right => Some(Message::EnterItem),
		KeyCode::Char('f') | KeyCode::Char('/') => Some(Message::SearchEnter),
		KeyCode::Char('s') => Some(Message::BookmarkInputEnter),
		KeyCode::Char('r') => Some(Message::Refresh),
		KeyCode::Char('d') => {
			if key.modifiers.contains(KeyModifiers::CONTROL) {
				Some(Message::PageDown)
			} else {
				Some(Message::DeleteBookmark)
			}
		}
		KeyCode::Char('u') => {
			if key.modifiers.contains(KeyModifiers::CONTROL) {
				return Some(Message::PageUp);
			}
			None
		}
		KeyCode::Char('.') => Some(Message::NavigatorEnter),
		_ => None,
	}
}

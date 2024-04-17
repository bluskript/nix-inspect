use std::path::PathBuf;

use crossterm::event::{self, Event, KeyCode};

use crate::{
	key_handler::handle_key,
	model::{
		next, prev, select_next, select_prev, Bookmark, BrowserPath, BrowserStackItem, InputModel,
		InputState, Message, Model, PathData, RunningState,
	},
	view::ViewData,
	Config,
};

pub struct UpdateContext {
	pub req_tx: kanal::Sender<BrowserPath>,
	pub config_path: PathBuf,
}

pub fn save_config(path: PathBuf, config: Config) {
	std::thread::spawn(move || {
		let _ = std::fs::write(&path, &serde_json::to_string_pretty(&config).unwrap());
	});
}

impl UpdateContext {
	pub fn queue_reeval(&self, path: &BrowserPath) {
		let path = path.clone();
		let req_tx = self.req_tx.clone();
		std::thread::spawn(move || {
			let _ = req_tx.send(path.clone());
		});
	}

	pub fn maybe_reeval_path(&self, path: &BrowserPath, model: &Model) {
		if model.path_data.get(&path).is_none() {
			let path = path.clone();
			self.queue_reeval(&path);
		}
	}

	pub fn maybe_reeval_parent(&self, model: &Model) {
		if let Some(BrowserStackItem::BrowserPath(path)) = model.visit_stack.prev_item() {
			self.maybe_reeval_path(path, model);
		}
	}

	pub fn maybe_reeval_selection_browser(&self, p: &BrowserPath, model: &Model) {
		let current_value = match model.path_data.get(&p) {
			Some(x) => x,
			None => return,
		};
		let list = match current_value {
			PathData::List(x) => x,
			_ => return,
		};
		let selected = match list.selected(&p) {
			Some(x) => x,
			None => return,
		};
		if list.list.contains(selected.0.last().unwrap()) {
			self.maybe_reeval_path(&selected, model);
		}
	}

	pub fn maybe_reeval_current_selection(
		&self,
		current_location: &BrowserStackItem,
		model: &Model,
	) {
		match current_location {
			BrowserStackItem::BrowserPath(p) => self.maybe_reeval_selection_browser(p, model),
			BrowserStackItem::Bookmarks => {
				if let Some(b) = model.selected_bookmark() {
					self.maybe_reeval_path(&b.path, model);
				}
			}
			BrowserStackItem::Recents => {
				if let Some(x) = model.selected_recent() {
					self.maybe_reeval_path(&x, model);
				}
			}
			BrowserStackItem::Root => {}
		}
	}

	pub fn maybe_reeval_selection(&self, model: &Model) {
		if let Some(x) = model.visit_stack.last() {
			self.maybe_reeval_current_selection(x, model);
		}
	}

	pub fn update(
		&mut self,
		view_data: &ViewData,
		model: &mut Model,
		msg: Message,
	) -> color_eyre::Result<Option<Message>> {
		match msg {
			Message::TermEvent(event) => match event {
				Event::Key(key) => {
					if key.kind == event::KeyEventKind::Press {
						if let Some(msg) = handle_key(key, &model) {
							return Ok(Some(msg));
						}
					}
				}
				_ => {}
			},
			Message::Data(p, d) => {
				let data = d.clone();
				model
					.path_data
					.entry(p)
					.and_modify(|x| match (x, data) {
						(PathData::List(p), PathData::List(d)) => {
							let cursor = p.state.selected().unwrap_or(0);
							p.state.select(Some(cursor.min(d.list.len()).max(0)));
							p.list = d.list;
						}
						x @ _ => *x.0 = x.1,
					})
					.or_insert(d.clone());
				self.maybe_reeval_selection(model);
			}
			Message::CurrentPath(p) => {
				model.visit_stack.push(BrowserStackItem::BrowserPath(p));
				self.maybe_reeval_selection(model);
			}
			Message::Refresh => {
				if let Some(path) = model.visit_stack.current() {
					self.queue_reeval(path);
					if let Some(data) = model.path_data.current_list(path) {
						if let Some(path) = data.selected(path) {
							self.queue_reeval(&path);
						}
					}
				}
				if let Some(BrowserStackItem::BrowserPath(path)) = model.visit_stack.prev_item() {
					self.queue_reeval(path);
				}
				if let Some(path) = model.visit_stack.current() {
					self.queue_reeval(path);
				}
			}
			Message::PageUp => {
				if let Some(x) = model.visit_stack.current() {
					if let Some(list) = model.path_data.current_list_mut(x) {
						let cursor = list.state.selected().unwrap_or(0);
						list.state.select(Some(
							cursor.saturating_sub(view_data.current_list_height.max(1) as usize / 2)
								as usize,
						));
					}
				}
			}
			Message::PageDown => {
				if let Some(x) = model.visit_stack.current() {
					if let Some(list) = model.path_data.current_list_mut(x) {
						let cursor = list.state.selected().unwrap_or(0);
						*list.state.selected_mut() = Some(
							(cursor + (view_data.current_list_height.max(1) / 2) as usize)
								.max(0)
								.min(list.list.len() - 1),
						);
					}
				}
			}
			Message::SearchNext | Message::SearchPrev => {
				if let InputState::Active(ref mut input_model) = model.search_input {
					let current_list = match model.visit_stack.current().cloned() {
						Some(x) => match model.path_data.current_list_mut(&x) {
							Some(x) => x,
							None => return Ok(None),
						},
						None => return Ok(None),
					};
					let cursor = current_list.state.selected().unwrap_or(0);
					if let Some((_, (i, _))) = match msg {
						Message::SearchNext => current_list
							.list
							.iter()
							.enumerate()
							.skip(cursor + 1)
							.closest_item(|(_, x)| x.contains(&input_model.input), 0),
						_ => current_list
							.list
							.iter()
							.enumerate()
							.take(cursor)
							.closest_item(|(_, x)| x.contains(&input_model.input), cursor),
					} {
						current_list.state.select(Some(i));
					}
					self.maybe_reeval_selection(model);
				}
			}
			Message::SearchEnter => {
				model.search_input = InputState::Active(InputModel {
					typing: true,
					input: "".to_string(),
					cursor_position: 0,
				});
			}
			Message::SearchExit => model.search_input = InputState::default(),
			Message::SearchInput(ev) => {
				if let InputState::Active(ref mut input_model) = model.search_input {
					input_model.handle_key_event(ev);
					match ev.code {
						KeyCode::Char(_) => {
							let current_list = match model.visit_stack.current().cloned() {
								Some(x) => match model.path_data.current_list_mut(&x) {
									Some(x) => x,
									None => return Ok(None),
								},
								None => return Ok(None),
							};
							if let Some((i, _)) = current_list.list.iter().closest_item(
								|x| x.contains(&input_model.input),
								current_list.state.selected().unwrap_or(0),
							) {
								*current_list.state.selected_mut() = Some(i);
							}
							self.maybe_reeval_selection(model);
						}
						KeyCode::Esc => return Ok(Some(Message::SearchExit)),
						KeyCode::Enter => input_model.typing = false,
						_ => {}
					}
				}
			}
			Message::BookmarkInputEnter => {
				let path_str = model
					.visit_stack
					.current()
					.and_then(|x| model.path_data.current_list(x))
					.and_then(|x| x.list.get(x.state.selected().unwrap_or(0)).cloned())
					.unwrap_or("".to_string());
				model.new_bookmark_input = InputState::Active(InputModel {
					typing: false,
					cursor_position: path_str.len(),
					input: path_str.to_string(),
				})
			}
			Message::BookmarkInputExit => {
				model.new_bookmark_input = InputState::Normal;
			}
			Message::BookmarkInput(key) => {
				if let InputState::Active(ref mut x) = model.new_bookmark_input {
					x.handle_key_event(key);
				}
			}
			Message::NavigatorNext | Message::NavigatorPrev => {
				if let InputState::Active(ref mut input_model) = model.path_navigator_input {
					let path = BrowserPath::from(input_model.input.clone());
					if let Some(parent) = path.parent() {
						if let Some(PathData::List(current_list)) = model.path_data.get_mut(&parent)
						{
							let cursor = current_list.state.selected().unwrap_or(0);
							let tab_prefix = path.0.last().unwrap();
							if let Some((_, (i, _))) = match msg {
								Message::NavigatorNext => current_list
									.list
									.iter()
									.enumerate()
									.skip(cursor + 1)
									.closest_item(|(_, x)| x.starts_with(tab_prefix), 0),
								_ => current_list
									.list
									.iter()
									.enumerate()
									.take(cursor)
									.rev()
									.closest_item(|(_, x)| x.contains(tab_prefix), cursor),
							} {
								*current_list.state.selected_mut() = Some(i);
							}
							self.maybe_reeval_selection(model);
						}
					}
				}
			}
			Message::NavigatorEnter => {
				let current_path = model.visit_stack.current();
				let path_str = ".".to_string()
					+ &current_path
						.map(|x| x.to_expr() + if x.0.len() > 1 { "." } else { "" })
						.unwrap_or("nixosConfigurations.".to_string());
				model.path_navigator_input = InputState::Active(InputModel {
					typing: true,
					cursor_position: path_str.len(),
					input: path_str,
				})
			}
			Message::NavigatorExit => model.path_navigator_input = InputState::default(),
			Message::NavigatorInput(ev) => {
				if let InputState::Active(ref mut x) = model.path_navigator_input {
					x.handle_key_event(ev);
					if ev.code != KeyCode::Tab && ev.code != KeyCode::BackTab {
						model.prev_tab_completion = None;
					}
					match ev.code {
						KeyCode::Char(_) | KeyCode::Backspace => {
							let path = BrowserPath::from(x.input.clone());
							if let Some(new_path) = path.parent() {
								self.maybe_reeval_path(&new_path, model);
								model.update_parent_selection(new_path);
								self.maybe_reeval_parent(model);
								self.maybe_reeval_selection(model);
							}
							if let Some(parent) = path.parent() {
								if let Some(PathData::List(parent_list)) =
									model.path_data.get_mut(&parent)
								{
									let tab_prefix = path.0.last().unwrap();
									let nearest_occurrence_index = parent_list
										.list
										.iter()
										.enumerate()
										.find(|(_, x)| x.starts_with(tab_prefix))
										.map(|(i, _)| i);

									if let Some(nearest_occurrence_index) = nearest_occurrence_index
									{
										parent_list.state.select(Some(nearest_occurrence_index));
									}
								}
							}
						}
						KeyCode::Tab | KeyCode::BackTab => {
							let path = BrowserPath::from(x.input.clone());
							if let Some(parent) = path.parent() {
								if let Some(PathData::List(parent_list)) =
									model.path_data.get_mut(&parent)
								{
									let cursor = parent_list.state.selected().unwrap_or(0);
									let tab_prefix = model
										.prev_tab_completion
										.as_ref()
										.unwrap_or(path.0.last().unwrap());
									let nearest_occurence_index = if ev.code == KeyCode::Tab {
										parent_list
											.list
											.iter()
											.enumerate()
											.skip(cursor + 1)
											.find(|(_, x)| x.starts_with(tab_prefix))
											.map(|(i, _)| i)
											.or_else(|| {
												parent_list
													.list
													.iter()
													.enumerate()
													.find(|(_, x)| x.starts_with(tab_prefix))
													.map(|(i, _)| i)
											})
									} else {
										parent_list
											.list
											.iter()
											.enumerate()
											.take(parent_list.state.selected().unwrap_or(0))
											.rev()
											.find(|(_, x)| x.starts_with(tab_prefix))
											.map(|(i, _)| i)
											.or_else(|| {
												parent_list
													.list
													.iter()
													.enumerate()
													.skip(cursor + 1)
													.rev()
													.find(|(_, x)| x.starts_with(tab_prefix))
													.map(|(i, _)| i)
											})
									};

									if let Some(nearest_occurrence_index) = nearest_occurence_index
									{
										if model.prev_tab_completion.is_none() {
											model.prev_tab_completion = Some(tab_prefix.clone());
										}
										parent_list.state.select(Some(nearest_occurrence_index));
										let nearest_occurrence =
											&parent_list.list[nearest_occurrence_index];
										let new_path =
											parent.child(nearest_occurrence.to_string()).to_expr();
										x.cursor_position = new_path.len() + 1;
										x.input = ".".to_string() + &new_path;
									}
								}
							}
							self.maybe_reeval_selection(model);
						}
						KeyCode::Esc => return Ok(Some(Message::NavigatorExit)),
						KeyCode::Enter => x.typing = false,
						_ => {}
					}
				}
			}
			Message::CreateBookmark => {
				if let Some(p) = model.visit_stack.current() {
					if let InputState::Active(state) = &model.new_bookmark_input {
						let name = &state.input;
						model.config.bookmarks.push(Bookmark {
							display: if name.len() > 0 {
								name.to_string()
							} else {
								p.0.last().unwrap_or(&"".to_string()).clone()
							},
							path: p.clone(),
						});
						model.new_bookmark_input = InputState::Normal;
						save_config(self.config_path.clone(), model.config.clone());
					}
				}
			}
			Message::DeleteBookmark => {
				if let Some(i) = model.bookmark_view_state.selected() {
					model.config.bookmarks.remove(i);
					let bookmarks_len = model.config.bookmarks.len();
					let selected = model.bookmark_view_state.selected_mut();
					let new = selected.map(|x| x.min(bookmarks_len - 1));
					*selected = new;
				}
				save_config(self.config_path.clone(), model.config.clone());
			}
			Message::Back => {
				if model.visit_stack.len() > 1 {
					model.visit_stack.pop();
					self.maybe_reeval_selection(model);
				}
			}
			Message::EnterItem => match model.visit_stack.last().unwrap_or(&BrowserStackItem::Root)
			{
				BrowserStackItem::Root => {
					match model.root_view_state.selected() {
						Some(0) => {
							model.visit_stack.push(BrowserStackItem::Bookmarks);
							self.maybe_reeval_current_selection(
								&BrowserStackItem::Bookmarks,
								model,
							);
						}
						Some(1) => {
							model.visit_stack.push(BrowserStackItem::Recents);
							self.maybe_reeval_current_selection(&BrowserStackItem::Recents, model);
						}
						Some(2) => {
							let x = BrowserPath::from("".to_string());
							self.maybe_reeval_selection_browser(&x, model);
							model.visit_stack.push_path(x);
						}
						_ => unreachable!(),
					};
				}
				BrowserStackItem::BrowserPath(p) => {
					if let Some(selected_item) = model
						.path_data
						.current_list(&p)
						.and_then(|list| list.state.selected().and_then(|i| list.list.get(i)))
					{
						let x = p.child(selected_item.clone());
						self.maybe_reeval_selection_browser(&x, model);
						model.visit_stack.push_path(x);
					}
				}
				BrowserStackItem::Bookmarks => {
					if let Some(x) = model.selected_bookmark() {
						self.maybe_reeval_selection_browser(&x.path, model);
						model.visit_stack.push_path(x.path.clone());
					}
				}
				BrowserStackItem::Recents => {
					if let Some(x) = model.selected_recent() {
						self.maybe_reeval_selection_browser(&x, model);
						model.visit_stack.push_path(x.clone());
					}
				}
			},
			Message::ListUp => {
				let x = model.visit_stack.last().unwrap_or(&BrowserStackItem::Root);
				match x {
					BrowserStackItem::Root => {
						select_prev(&mut model.root_view_state, 3);
					}
					BrowserStackItem::BrowserPath(p) => {
						if let Some(list) = model.path_data.current_list_mut(&p) {
							let cursor = list.state.selected().unwrap_or(0);
							list.state.select(Some(prev(cursor, list.list.len())));
						}
					}
					BrowserStackItem::Bookmarks => {
						select_prev(&mut model.bookmark_view_state, model.config.bookmarks.len());
					}
					BrowserStackItem::Recents => {
						select_prev(&mut model.recents_view_state, model.recents.len());
					}
				}
				self.maybe_reeval_current_selection(&x, model);
			}
			Message::ListDown => {
				let x = model.visit_stack.last().unwrap_or(&BrowserStackItem::Root);
				match x {
					BrowserStackItem::Root => {
						select_next(&mut model.root_view_state, 3);
						if let Some(1) = model.root_view_state.selected() {
							let req_tx = self.req_tx.clone();
							std::thread::spawn(move || {
								let _ = req_tx.send(BrowserPath::from("".to_string()));
							});
						}
					}
					BrowserStackItem::BrowserPath(p) => {
						if let Some(list) = model.path_data.current_list_mut(&p) {
							let cursor = list.state.selected().unwrap_or(0);
							list.state.select(Some(next(cursor, list.list.len())));
							let selected = list.selected(&p);
							if let Some(selected) = selected {
								if model.path_data.get(&selected).is_none() {
									let _ = self.req_tx.send(selected);
								}
							}
						}
						self.maybe_reeval_selection(model);
					}
					BrowserStackItem::Bookmarks => {
						select_next(&mut model.bookmark_view_state, model.config.bookmarks.len());
					}
					BrowserStackItem::Recents => {
						select_next(&mut model.recents_view_state, model.recents.len());
					}
				}
				self.maybe_reeval_current_selection(&x, model);
			}
			Message::Quit => model.running_state = RunningState::Stopped,
		};
		Ok(None)
	}
}

/// Returns the closest item to a specific index
/// Used for search in lists to make it not jump around as much
trait ClosestItem {
	type Item;

	fn closest_item<P>(self, predicate: P, pos: usize) -> Option<(usize, Self::Item)>
	where
		Self: Sized,
		P: FnMut(&Self::Item) -> bool;
}

impl<I> ClosestItem for I
where
	I: Iterator,
{
	type Item = I::Item;

	fn closest_item<P>(self, mut predicate: P, pos: usize) -> Option<(usize, Self::Item)>
	where
		P: FnMut(&Self::Item) -> bool,
	{
		self.enumerate()
			.filter(|(_, x)| predicate(x))
			.min_by(|(i, _), (j, _)| {
				let cmp_i = pos as i32 - *i as i32;
				let cmp_j = pos as i32 - *j as i32;
				cmp_i.abs().cmp(&cmp_j.abs())
			})
	}
}

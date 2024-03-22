use crossterm::event::KeyCode;

use crate::model::{
	next, prev, select_next, select_prev, BrowserPath, BrowserStackItem, InputModel, InputState,
	Message, Model, PathData, RunningState,
};

pub struct UpdateContext {
	pub req_tx: kanal::Sender<BrowserPath>,
}

impl UpdateContext {
	pub fn maybe_reeval_path(&self, path: &BrowserPath, model: &Model) {
		if model.path_data.get(&path).is_none() {
			let req_tx = self.req_tx.clone();
			let path = path.clone();
			std::thread::spawn(move || {
				let _ = req_tx.send(path);
			});
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
		let selected = list.selected(&p);
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
		model: &mut Model,
		msg: Message,
	) -> color_eyre::Result<Option<Message>> {
		match msg {
			Message::Data(p, d) => {
				model.path_data.insert(p, d);
				self.maybe_reeval_selection(model);
			}
			Message::CurrentPath(p) => {
				model.visit_stack.push(BrowserStackItem::BrowserPath(p));
				self.maybe_reeval_selection(model);
			}
			Message::SearchEnter => {
				model.search_input = InputState::Active(InputModel {
					typing: false,
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
							if let Some(position) = current_list
								.list
								.iter()
								.closest_item(&input_model.input, current_list.cursor)
							{
								current_list.cursor = position;
							}
							self.maybe_reeval_selection(model);
						}
						KeyCode::Esc => return Ok(Some(Message::SearchExit)),
						KeyCode::Enter => input_model.typing = false,
						_ => {}
					}
				}
			}
			Message::NavigatorEnter => {
				let current_path = model.visit_stack.current();
				let path_str = current_path
					.map(|x| x.to_expr())
					.unwrap_or("nixosConfigurations".to_string())
					+ ".";
				model.path_navigator_input = InputState::Active(InputModel {
					typing: false,
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
								tracing::debug!("{:?}", new_path);
								self.maybe_reeval_path(&new_path, model);
								model.update_parent_selection(new_path);
								self.maybe_reeval_parent(model);
								self.maybe_reeval_selection(model);
							}
						}
						KeyCode::Tab | KeyCode::BackTab => {
							let path = BrowserPath::from(x.input.clone());
							if let Some(parent) = path.parent() {
								if let Some(PathData::List(parent_list)) =
									model.path_data.get_mut(&parent)
								{
									let tab_prefix = model
										.prev_tab_completion
										.as_ref()
										.unwrap_or(path.0.last().unwrap());
									let nearest_occurence_index = if ev.code == KeyCode::Tab {
										parent_list
											.list
											.iter()
											.enumerate()
											.skip(parent_list.cursor + 1)
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
											.take(parent_list.cursor)
											.rev()
											.find(|(_, x)| x.starts_with(tab_prefix))
											.map(|(i, _)| i)
											.or_else(|| {
												parent_list
													.list
													.iter()
													.enumerate()
													.skip(parent_list.cursor + 1)
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
										parent_list.cursor = nearest_occurrence_index;
										let nearest_occurrence =
											&parent_list.list[nearest_occurrence_index];
										let new_path =
											parent.child(nearest_occurrence.to_string()).to_expr();
										x.cursor_position = new_path.len();
										x.input = new_path;
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
			Message::Back => {
				model.visit_stack.pop();
				self.maybe_reeval_selection(model);
			}
			Message::Enter => match model.visit_stack.last().unwrap_or(&BrowserStackItem::Root) {
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
							let x = BrowserPath::from("nixosConfigurations".to_string());
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
						.and_then(|list| list.list.get(list.cursor))
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
							list.cursor = prev(list.cursor, list.list.len());
						}
					}
					BrowserStackItem::Bookmarks => {
						select_prev(&mut model.bookmark_view_state, model.bookmarks.len());
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
								let _ = req_tx
									.send(BrowserPath::from("nixosConfigurations".to_string()));
							});
						}
					}
					BrowserStackItem::BrowserPath(p) => {
						if let Some(list) = model.path_data.current_list_mut(&p) {
							list.cursor = next(list.cursor, list.list.len());
							let selected = list.selected(&p);
							if model.path_data.get(&selected).is_none() {
								let _ = self.req_tx.send(selected);
							}
						}
						self.maybe_reeval_selection(model);
					}
					BrowserStackItem::Bookmarks => {
						select_next(&mut model.bookmark_view_state, model.bookmarks.len());
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
	fn closest_item(self, search_text: &String, pos: usize) -> Option<usize>
	where
		Self: Sized;
}

impl<I> ClosestItem for I
where
	I: Iterator,
	I::Item: AsRef<str>,
{
	fn closest_item(self, search_text: &String, pos: usize) -> Option<usize> {
		self.enumerate()
			.filter(|(_, x)| x.as_ref().contains(search_text))
			.map(|(i, _)| i)
			.min_by(|i, j| {
				let cmp_i = pos as i32 - *i as i32;
				let cmp_j = pos as i32 - *j as i32;
				cmp_i.abs().cmp(&cmp_j.abs())
			})
	}
}

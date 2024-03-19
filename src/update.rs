use crossterm::event::KeyCode;

use crate::model::{
	next, prev, select_next, select_prev, BrowserPath, BrowserStackItem, InputModel, InputState,
	Message, Model, PathData, RunningState,
};

pub struct UpdateContext {
	pub req_tx: kanal::Sender<BrowserPath>,
}

impl UpdateContext {
	pub fn maybe_reeval_parent(&self, model: &Model) {
		if let Some(BrowserStackItem::BrowserPath(path)) = model.visit_stack.prev_item() {
			if model.path_data.get(path).is_none() {
				let req_tx = self.req_tx.clone();
				let path = path.clone();
				std::thread::spawn(move || {
					let _ = req_tx.send(path);
				});
			}
		}
	}

	pub fn maybe_reeval_selection(&self, model: &Model) {
		self.maybe_reeval_parent(model);
		let current_path = match model.visit_stack.current() {
			Some(x) => x,
			None => return,
		};
		let current_value = match model.path_data.get(&current_path) {
			Some(x) => x,
			None => {
				let req_tx = self.req_tx.clone();
				let current_path = current_path.clone();
				std::thread::spawn(move || {
					let _ = req_tx.send(current_path);
				});
				return;
			}
		};
		let list = match current_value {
			PathData::List(x) => x,
			_ => return,
		};
		let selected = list.selected(&current_path);
		if model.path_data.get(&selected).is_none()
			&& list.list.contains(selected.0.last().unwrap())
		{
			let req_tx = self.req_tx.clone();
			std::thread::spawn(move || {
				let _ = req_tx.send(selected);
			});
		}
	}

	pub fn maybe_reeval_selected_bookmark(&self, model: &Model) {
		if let Some(bookmark) = model.selected_bookmark() {
			if model.path_data.get(&bookmark.path).is_none() {
				let tx = self.req_tx.clone();
				let path = bookmark.path.clone();
				std::thread::spawn(move || {
					let _ = tx.send(path);
				});
			}
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
				let current_path = match model.visit_stack.last() {
					Some(BrowserStackItem::BrowserPath(p)) => p.clone(),
					_ => return Ok(None),
				};
				if let InputState::Active(ref mut x) = model.path_navigator_input {
					x.handle_key_event(ev);
					if ev.code != KeyCode::Tab && ev.code != KeyCode::BackTab {
						model.prev_tab_completion = None;
					}
					match ev.code {
						KeyCode::Char(_) | KeyCode::Backspace => {
							let path = BrowserPath::from(x.input.clone());
							let path_len_diff = path.0.len() as i32 - current_path.0.len() as i32;
							match path_len_diff {
								diff if diff == 1 => {
									let current_list =
										match model.path_data.current_list_mut(&current_path) {
											Some(x) => x,
											None => return Ok(None),
										};
									if let Some(position) = current_list
										.list
										.iter()
										.closest_item(&x.input, current_list.cursor)
									{
										current_list.cursor = position;
									}

									if let Some(target) = path.0.last() {
										if current_list.list.contains(target) {
											model.visit_stack.push_path(path);
											model.update_parent_selection(current_path);
											self.maybe_reeval_selection(model);
										}
									}
								}
								diff if diff == -1 => {
									model.visit_stack.pop();
									model.update_parent_selection(current_path);
									self.maybe_reeval_selection(model);
								}
								_ => {
									if let Some(new_path) = path.parent() {
										model.update_parent_selection(new_path);
										self.maybe_reeval_selection(model);
										self.maybe_reeval_parent(model);
									}
								}
							}
							// let req_tx = self.req_tx.clone();
							// std::thread::spawn(move || {
							//     let _ = req_tx.send(path);
							// });
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
			Message::ListUp => match model.visit_stack.last().unwrap_or(&BrowserStackItem::Root) {
				BrowserStackItem::Root => {
					select_prev(&mut model.root_view_state, 2);
				}
				BrowserStackItem::BrowserPath(p) => {
					if let Some(list) = model.path_data.current_list_mut(&p) {
						list.cursor = prev(list.cursor, list.list.len());
					}
					self.maybe_reeval_selection(model);
				}
				BrowserStackItem::Bookmarks => {
					select_prev(&mut model.bookmark_view_state, model.bookmarks.len());
					self.maybe_reeval_selected_bookmark(model);
				}
			},
			Message::Back => {
				model.visit_stack.pop();
				self.maybe_reeval_selection(model);
			}
			Message::Enter => match model.visit_stack.last().unwrap_or(&BrowserStackItem::Root) {
				BrowserStackItem::Root => {
					match model.root_view_state.selected() {
						Some(0) => {
							model.visit_stack.push(BrowserStackItem::Bookmarks);
							self.maybe_reeval_selected_bookmark(model);
						}
						Some(1) => model
							.visit_stack
							.push_path(BrowserPath::from("nixosConfigurations".to_string())),
						_ => unreachable!(),
					};
				}
				BrowserStackItem::BrowserPath(p) => {
					if let Some(selected_item) = model
						.path_data
						.current_list(&p)
						.and_then(|list| list.list.get(list.cursor))
					{
						model.visit_stack.push_path(p.child(selected_item.clone()));
					}
					self.maybe_reeval_selection(model);
				}
				BrowserStackItem::Bookmarks => {
					if let Some(x) = model.selected_bookmark() {
						model.visit_stack.push_path(x.path.clone());
					}
				}
			},
			Message::ListDown => {
				match model.visit_stack.last().unwrap_or(&BrowserStackItem::Root) {
					BrowserStackItem::Root => {
						select_next(&mut model.root_view_state, 2);
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
						self.maybe_reeval_selected_bookmark(model);
					}
				}
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

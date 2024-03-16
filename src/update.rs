use crossterm::event::KeyCode;

use crate::model::{BrowserPath, InputModel, InputState, Message, Model, PathData, RunningState};

pub struct UpdateContext {
    pub req_tx: kanal::Sender<BrowserPath>,
}

impl UpdateContext {
    pub fn maybe_reeval_selection(&self, model: &Model) {
        let list = match model.path_data.current_list(&model.current_path) {
            Some(x) => x,
            None => return,
        };
        let selected = list.selected(&model.current_path);
        if model.path_data.get(&selected).is_none() {
            let req_tx = self.req_tx.clone();
            std::thread::spawn(move || {
                let _ = req_tx.send(selected);
            });
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
                model.current_path = p;
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
                if let InputState::Active(ref mut x) = model.search_input {
                    x.handle_key_event(ev);
                    let current_list = match model.path_data.current_list_mut(&model.current_path) {
                        Some(x) => x,
                        None => return Ok(None),
                    };
                    match ev.code {
                        KeyCode::Char(_) => {
                            if let Some(position) = current_list
                                .list
                                .iter()
                                .closest_item(&x.input, current_list.cursor)
                            {
                                current_list.cursor = position;
                            }
                            self.maybe_reeval_selection(model);
                        }
                        KeyCode::Esc => return Ok(Some(Message::SearchExit)),
                        KeyCode::Enter => x.typing = false,
                        _ => {}
                    }
                }
            }
            Message::NavigatorEnter => {
                let path_str = model.current_path.to_expr() + ".";
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
                            let path_len_diff =
                                path.0.len() as i32 - model.current_path.0.len() as i32;
                            match path_len_diff {
                                diff if diff == 1 => {
                                    let current_list =
                                        match model.path_data.current_list_mut(&model.current_path)
                                        {
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
                                            model.current_path = path.clone();
                                            model.path_data.update_parent_selection(
                                                model.current_path.clone(),
                                            );
                                            self.maybe_reeval_selection(model);
                                        }
                                    }
                                }
                                diff if diff == -1 => {
                                    model.current_path = path.clone();
                                    model
                                        .path_data
                                        .update_parent_selection(model.current_path.clone());
                                    self.maybe_reeval_selection(model);
                                }
                                _ => {
                                    if let Some(new_path) = path.parent() {
                                        model.current_path = new_path;
                                        model
                                            .path_data
                                            .update_parent_selection(model.current_path.clone());
                                        self.maybe_reeval_selection(model);
                                    }
                                }
                            }
                            let req_tx = self.req_tx.clone();
                            std::thread::spawn(move || {
                                let _ = req_tx.send(path);
                            });
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
                if let Some(x) = model.current_path.parent() {
                    model.current_path = x;
                }
                self.maybe_reeval_selection(model);
            }
            Message::Enter => {
                if let Some(x) = model
                    .path_data
                    .current_list(&model.current_path)
                    .and_then(|x| x.list.get(x.cursor))
                {
                    model.current_path.0.push(x.clone());
                }
                self.maybe_reeval_selection(model);
            }
            Message::ListUp => {
                if let Some(list) = model.path_data.current_list_mut(&model.current_path) {
                    list.cursor = if list.cursor == 0 {
                        list.list.len() - 1
                    } else {
                        list.cursor - 1
                    };
                }
                self.maybe_reeval_selection(model);
            }
            Message::ListDown => {
                if let Some(list) = model.path_data.current_list_mut(&model.current_path) {
                    list.cursor = if list.cursor >= list.list.len() - 1 {
                        0
                    } else {
                        list.cursor + 1
                    };
                    let selected = list.selected(&model.current_path);
                    if model.path_data.get(&selected).is_none() {
                        let _ = self.req_tx.send(selected);
                    }
                }
                self.maybe_reeval_selection(model);
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

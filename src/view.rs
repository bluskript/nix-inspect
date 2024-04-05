use color_eyre::owo_colors::OwoColorize;
use lazy_static::lazy_static;
use ratatui::widgets::{ListState, Wrap};
use ratatui::Frame;

use ratatui::{
	layout::{Alignment, Constraint, Direction, Layout, Rect},
	style::{Color, Style, Stylize},
	symbols,
	text::{Line, Span},
	widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::model::{BrowserPath, BrowserStackItem, InputState, ListData, Model, PathData};

/// View data that should be provided to the update handler (for page-up / page-down behavior)
#[derive(Default)]
pub struct ViewData {
	pub current_list_height: u16,
}

pub fn view(model: &Model, f: &mut Frame) -> ViewData {
	let miller_layout = Layout::default()
		.direction(Direction::Horizontal)
		.constraints(Constraint::from_percentages([20, 40, 20]))
		.split(f.size());

	let previous_list_block =
		Block::default().borders(Borders::TOP | Borders::BOTTOM | Borders::LEFT);
	let previous_inner = previous_list_block.inner(miller_layout[0]);
	f.render_widget(previous_list_block, miller_layout[0]);

	render_previous_stack(model, f, previous_inner);

	let mut view_data = ViewData::default();

	match model.visit_stack.last().unwrap_or(&BrowserStackItem::Root) {
		BrowserStackItem::BrowserPath(p) => match model.path_data.get(&p) {
			Some(data) if !matches!(data, PathData::List(_)) => {
				let block = Block::new()
					.borders(Borders::ALL)
					.border_set(symbols::border::Set {
						top_left: symbols::line::NORMAL.horizontal_down,
						bottom_left: symbols::line::NORMAL.horizontal_up,
						..symbols::border::PLAIN
					})
					.title_style(Style::new().blue())
					.title(data.get_type());
				let outer = miller_layout[2].union(miller_layout[1]);
				let inner = block.inner(outer);
				view_data.current_list_height = inner.height;
				f.render_widget(block, outer);
				let _ = render_value_preview(f, data, inner);
			}
			x @ _ => {
				let current_list_block = current_frame();
				let inner = current_list_block.inner(miller_layout[1]);
				view_data.current_list_height = inner.height;
				f.render_widget(current_list_block, miller_layout[1]);
				if let Some(PathData::List(current_path_data)) = x {
					let _ = render_list(
						f,
						&current_path_data,
						inner,
						Some(&model.search_input),
						Some(&model.path_navigator_input),
						&model.prev_tab_completion,
					);
				}
				let _ = render_preview(f, model, miller_layout[2], p);
			}
		},
		x @ _ => {
			let current_list_block = current_frame();
			let current_inner = current_list_block.inner(miller_layout[1]);
			view_data.current_list_height = current_inner.height;
			f.render_widget(current_list_block, miller_layout[1]);

			let preview_frame = preview_frame();
			let preview_inner = preview_frame.inner(miller_layout[2]);
			f.render_widget(preview_frame, miller_layout[2]);

			match x {
				BrowserStackItem::Root => {
					render_root(model, f, current_inner);

					match model.root_view_state.selected() {
						// Bookmarks
						Some(0) => {
							render_bookmarks(model, f, preview_inner);
						}
						Some(1) => {
							render_recents(model, f, preview_inner);
						}
						// Root
						Some(2) => {
							if let Some(PathData::List(current_list_data)) =
								model.path_data.get(&BrowserPath::from("".to_string()))
							{
								render_list(
									f,
									current_list_data,
									preview_inner,
									Some(&model.search_input),
									Some(&model.path_navigator_input),
									&model.prev_tab_completion,
								);
							}
						}
						_ => {}
					}
				}
				BrowserStackItem::Bookmarks => {
					render_bookmarks(model, f, current_inner);

					if let Some(data) = model
						.selected_bookmark()
						.and_then(|x| model.path_data.get(&x.path))
					{
						render_value_preview(f, data, preview_inner);
					}
				}
				BrowserStackItem::Recents => {
					render_recents(model, f, current_inner);
					if let Some(data) = model
						.selected_bookmark()
						.and_then(|x| model.path_data.get(&x.path))
					{
						render_value_preview(f, data, preview_inner);
					}
				}
				BrowserStackItem::BrowserPath(_) => unreachable!(),
			}
		}
	}

	render_inputs(f, model, f.size());

	view_data
}

pub fn render_previous_stack(model: &Model, f: &mut Frame, inner: Rect) {
	match model.visit_stack.prev_item() {
		Some(BrowserStackItem::BrowserPath(p)) => render_previous_list(f, model, inner, p),
		Some(BrowserStackItem::Bookmarks) => render_bookmarks(model, f, inner),
		Some(BrowserStackItem::Root) => render_root(model, f, inner),
		Some(BrowserStackItem::Recents) => render_recents(model, f, inner),
		None => {}
	}
}

pub fn with_selected_style(x: List) -> List {
	x.highlight_symbol(">>").highlight_style(*SELECTED_STYLE)
}

pub fn render_root(model: &Model, f: &mut Frame, inner: Rect) {
	f.render_stateful_widget(
		with_selected_style(List::new(["Bookmarks", "Recents", "Root"])),
		inner,
		&mut model.root_view_state.clone(),
	);
}

pub fn render_recents(model: &Model, f: &mut Frame, inner: Rect) {
	f.render_stateful_widget(
		with_selected_style(List::new(model.recents.iter().map(|x| x.to_expr()))),
		inner,
		&mut model.recents_view_state.clone(),
	)
}

pub fn render_bookmarks(model: &Model, f: &mut Frame, inner: Rect) {
	f.render_stateful_widget(
		with_selected_style(List::new(model.config.bookmarks.clone())),
		inner,
		&mut model.bookmark_view_state.clone(),
	)
}

pub fn current_frame<'a>() -> Block<'a> {
	Block::default()
		.borders(Borders::ALL)
		.border_set(symbols::border::Set {
			top_left: symbols::line::NORMAL.horizontal_down,
			top_right: symbols::line::NORMAL.horizontal_down,
			bottom_left: symbols::line::NORMAL.horizontal_up,
			bottom_right: symbols::line::NORMAL.horizontal_up,
			..symbols::border::PLAIN
		})
}

lazy_static! {
	pub static ref SELECTED_STYLE: ratatui::style::Style =
		Style::default().bg(Color::Yellow).fg(Color::Black);
}

pub fn render_list(
	f: &mut Frame,
	list: &ListData,
	inner: Rect,
	search_input: Option<&InputState>,
	path_navigator_input: Option<&InputState>,
	prev_tab_completion: &Option<String>,
) {
	let selected_style = *SELECTED_STYLE;
	let render_list: Vec<_> = list
		.list
		.iter()
		.enumerate()
		.map(|(i, x)| {
			let highlight_style = if i == list.cursor {
				selected_style
			} else {
				Style::default()
			};
			match (path_navigator_input, search_input) {
				(Some(_), Some(InputState::Active(search_model))) => {
					ListItem::new(highlight_on_match(x.as_str(), search_model.input.as_str()))
						.style(highlight_style)
				}
				(Some(InputState::Active(nav_model)), Some(_)) => {
					let search_str = prev_tab_completion
						.as_deref()
						.or_else(|| nav_model.input.split('.').last())
						.filter(|x| !x.is_empty());
					ListItem::new(x.as_str()).style(search_str.map_or(
						highlight_style,
						|search_str| {
							if x.starts_with(search_str) {
								Style::default().on_green().fg(Color::Black)
							} else {
								highlight_style
							}
						},
					))
				}
				_ => ListItem::new(x.clone()).style(highlight_style),
			}
		})
		.collect();

	f.render_stateful_widget(
		List::new(render_list),
		inner,
		&mut ListState::default().with_selected(Some(list.cursor)),
	);
}

/// TODO: unify with other list code
pub fn render_previous_list(f: &mut Frame, model: &Model, inner: Rect, p: &BrowserPath) {
	let list = match model.path_data.get(&p) {
		Some(PathData::List(list)) => list,
		_ => return,
	};

	f.render_stateful_widget(
		with_selected_style(List::new(list.list.clone())),
		inner,
		&mut ListState::default().with_selected(Some(list.cursor)),
	);
}

pub fn render_inputs(f: &mut Frame, model: &Model, inner: Rect) {
	// Offset from the bottom, in case there are two parallel inputs being displayed
	let mut offset = 1;

	// Render the search string in the bottom right corner of the container
	if let InputState::Active(search_model) = &model.search_input {
		let render_text = format!("Search: {}", search_model.input.clone());
		// ratatui does not have a concept of a "right overflow" to my understanding, so clip the
		// text from the left manually if it starts overflowing
		f.render_widget(
			Paragraph::new(&render_text[render_text.len().saturating_sub(inner.width as usize)..])
				.alignment(Alignment::Left),
			Rect::new(inner.left(), inner.bottom() - offset, inner.width, 1),
		);
		offset += 1;
	}
	if let InputState::Active(navigator_state) = &model.path_navigator_input {
		let render_text = format!("Goto: {}", navigator_state.input.clone());
		f.render_widget(
			Paragraph::new(&render_text[render_text.len().saturating_sub(inner.width as usize)..])
				.alignment(Alignment::Left)
				.fg(Color::Gray),
			Rect::new(inner.left(), inner.bottom() - offset, inner.width, 1),
		);
		offset += 1;
	}

	if let InputState::Active(bookmark_input_state) = &model.new_bookmark_input {
		let render_text = format!("bookmark name: {}", bookmark_input_state.input.clone());
		f.render_widget(
			Paragraph::new(&render_text[render_text.len().saturating_sub(inner.width as usize)..])
				.alignment(Alignment::Left)
				.fg(Color::Gray),
			Rect::new(inner.left(), inner.bottom() - offset, inner.width, 1),
		);
	}
}

pub fn render_value_preview(f: &mut Frame, value: &PathData, inner: Rect) {
	match value {
		// NixValue::Attrs(list) => {
		//     let items = list.iter().map(|(k, _v)| {
		//         model
		//             .values
		//             .get(&path.child(k.clone()))
		//             .map(|x| {
		//                 let value_type = x.value.get_preview_symbol();
		//                 let highlight_color = color_from_type(&x.value);
		//                 ListItem::new(format!("{: ^5} {} = {}", value_type, k, x.value))
		//                     .fg(highlight_color)
		//             })
		//             .unwrap_or(ListItem::new(format!("? {}", k)))
		//     });
		//     f.render_widget(List::new(items), inner);
		// }
		// NixValue::List(ref list) => {
		//     let items = list.iter().map(|x| format!("{:?}", x)).collect::<Vec<_>>();
		//     f.render_widget(
		//         List::new(items).style(Style::new().fg(color_from_type(&value))),
		//         inner,
		//     );
		// }
		PathData::List(list) => {
			render_list(f, list, inner, None, None, &None);
		}
		_ => {
			f.render_widget(
				Paragraph::new(value.to_string())
					.style(Style::new().fg(color_from_type(value)))
					.wrap(Wrap { trim: true }),
				inner,
			);
		}
	}
}

pub fn preview_frame<'a>() -> Block<'a> {
	Block::new()
		.borders(Borders::RIGHT | Borders::TOP | Borders::BOTTOM)
		.title_style(Style::new().blue())
}

pub fn render_preview(f: &mut Frame, model: &Model, outer: Rect, current_path: &BrowserPath) {
	let mut block = preview_frame();

	let selected_path = model
		.path_data
		.current_list(&current_path)
		.map(|list| list.selected(&current_path));

	if let Some(selected_path) = selected_path {
		if let Some(value) = model.path_data.get(&selected_path) {
			block = block.title(value.get_type());
			let inner = block.inner(outer);
			f.render_widget(block, outer);
			render_value_preview(f, &value, inner);
			return;
		}
	}

	f.render_widget(block, outer);
}

fn color_from_type(value: &PathData) -> Color {
	match value {
		// PathData::Attrs(_) => Color::Yellow,
		PathData::List(_) => Color::Cyan,
		PathData::Int(_) | PathData::Float(_) => Color::LightBlue,
		PathData::String(_) => Color::LightRed,
		PathData::Path(_) => Color::Red,
		PathData::Bool(_) => Color::Green,
		PathData::Function => Color::Magenta,
		PathData::Thunk => Color::LightMagenta,
		_ => Color::Gray,
	}
}

fn highlight_on_match<'a>(haystack: &'a str, needle: &'a str) -> Line<'a> {
	let mut spans = Vec::new();
	let mut last_index = 0;

	for (index, _) in haystack.match_indices(needle) {
		if index > last_index {
			spans.push(Span::raw(&haystack[last_index..index]));
		}
		spans.push(Span::styled(
			needle,
			Style::new().fg(Color::Black).bg(Color::Blue),
		));
		last_index = index + needle.len();
	}

	if last_index < haystack.len() {
		spans.push(Span::raw(&haystack[last_index..]));
	}

	Line::from(spans)
}

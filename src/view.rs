use color_eyre::owo_colors::OwoColorize;
use ratatui::widgets::ListState;
use ratatui::Frame;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};

use crate::model::{InputState, ListData, Model, PathData};
use crate::workers::NixValue;

pub fn view(model: &Model, f: &mut Frame) {
    let miller_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(Constraint::from_percentages([20, 40, 20]))
        .split(f.size());

    let previous_list_block =
        Block::default().borders(Borders::TOP | Borders::BOTTOM | Borders::LEFT);
    let inner = previous_list_block.inner(miller_layout[0]);
    f.render_widget(previous_list_block, miller_layout[0]);

    render_previous_list(f, model, inner);

    match model.path_data.get(&model.current_path) {
        Some(PathData::List(current_path_data)) => {
            let current_list_block =
                Block::default()
                    .borders(Borders::ALL)
                    .border_set(symbols::border::Set {
                        top_left: symbols::line::NORMAL.horizontal_down,
                        top_right: symbols::line::NORMAL.horizontal_down,
                        bottom_left: symbols::line::NORMAL.horizontal_up,
                        bottom_right: symbols::line::NORMAL.horizontal_up,
                        ..symbols::border::PLAIN
                    });
            let inner = current_list_block.inner(miller_layout[1]);
            f.render_widget(current_list_block, miller_layout[1]);
            let _ = render_current_list(f, model, &current_path_data, inner);
            let _ = render_preview(f, model, miller_layout[2]);
        }
        Some(data) => {
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
            f.render_widget(block, outer);
            // let _ = render_value_preview(f, model, &current_path, &data.value, inner);
        }
        _ => {}
    }
    render_search(f, model, f.size());
}

pub fn render_current_list(f: &mut Frame, model: &Model, list: &ListData, inner: Rect) {
    let selected_style = Style::default().bg(Color::Yellow).fg(Color::Black);
    // Special rendering logic for search highlighting
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
            match (&model.path_navigator_input, &model.search_input) {
                (_, InputState::Active(search_model)) => {
                    return ListItem::new(highlight_on_match(
                        x.as_str(),
                        search_model.input.as_str(),
                    ))
                    .style(highlight_style);
                }
                (InputState::Active(nav_model), _) => {
                    if let Some(search_str) = model
                        .prev_tab_completion
                        .as_deref()
                        .or(nav_model.input.split(".").last())
                        .and_then(|x| if x.len() == 0 { None } else { Some(x) })
                    {
                        return ListItem::new(x.as_str()).style(if x.starts_with(search_str) {
                            Style::default().on_green().black()
                        } else {
                            highlight_style
                        });
                    }
                }
                _ => {}
            }
            ListItem::new(x.clone()).style(highlight_style)
        })
        .collect();

    f.render_stateful_widget(
        List::new(render_list).highlight_symbol(">>"),
        inner,
        &mut ListState::default().with_selected(Some(list.cursor)),
    );
}

pub fn render_previous_list(f: &mut Frame, model: &Model, inner: Rect) {
    let list = match model
        .current_path
        .parent()
        .and_then(|p| model.path_data.get(&p))
    {
        Some(PathData::List(list)) => list,
        _ => return,
    };

    f.render_stateful_widget(
        List::new(list.list.clone())
            .highlight_style(Style::default().bg(Color::Yellow).fg(Color::Black))
            .highlight_symbol(">>"),
        inner,
        &mut ListState::default().with_selected(Some(list.cursor)),
    );
}

pub fn render_search(f: &mut Frame, model: &Model, inner: Rect) {
    // Offset from the bottom, in case there are two parallel inputs being displayed
    let mut offset = 1;

    // Render the search string in the bottom right corner of the container
    if let InputState::Active(search_model) = &model.search_input {
        f.render_widget(
            Paragraph::new(format!("Search: {}", search_model.input.clone()))
                .alignment(Alignment::Right),
            Rect::new(inner.left(), inner.bottom() - offset, inner.width, 1),
        );
        offset += 1;
    }
    if let InputState::Active(navigator_state) = &model.path_navigator_input {
        f.render_widget(
            Paragraph::new(format!("Goto: {}", navigator_state.input.clone()))
                .alignment(Alignment::Right)
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
            f.render_widget(List::new(list.list.clone()), inner);
        }
        _ => {
            let item = vec![value.to_string()];
            f.render_widget(List::new(item).style(Style::new()), inner);
        }
    }
}

pub fn render_preview(f: &mut Frame, model: &Model, outer: Rect) {
    let mut block = Block::new()
        .borders(Borders::RIGHT | Borders::TOP | Borders::BOTTOM)
        .title_style(Style::new().blue());
    let inner = block.inner(outer);
    let selected_path = match model
        .path_data
        .current_list(&model.current_path)
        .map(|list| list.selected(&model.current_path))
    {
        Some(x) => x,
        None => return,
    };
    let value = match model.path_data.get(&selected_path) {
        Some(x) => x,
        None => return,
    };
    block = block.title(value.get_type());
    f.render_widget(block, outer);

    render_value_preview(f, &value, inner);
}

fn color_from_type(value: &NixValue) -> Color {
    match value {
        NixValue::Attrs(_) => Color::Yellow,
        NixValue::List(_) => Color::Cyan,
        NixValue::Int(_) | NixValue::Float(_) => Color::LightBlue,
        NixValue::String(_) => Color::LightRed,
        NixValue::Path(_) => Color::Red,
        NixValue::Bool(_) => Color::Green,
        NixValue::Function => Color::Magenta,
        NixValue::Thunk => Color::LightMagenta,
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

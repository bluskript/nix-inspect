use std::{
	collections::HashMap,
	fmt,
	ops::{Deref, DerefMut},
};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{text::Text, widgets::ListState};

use crate::workers::NixValue;

#[derive(Default, Debug)]
pub struct Model {
	pub running_state: RunningState,

	pub path_data: PathDataMap,
	pub bookmarks: Vec<Bookmark>,

	pub visit_stack: BrowserStack,

	pub search_input: InputState,
	pub path_navigator_input: InputState,

	/// TODO: things that the architecture doesnt handle all that well
	pub prev_tab_completion: Option<String>,

	pub root_view_state: ListState,
	pub bookmark_view_state: ListState,
}

impl Model {
	pub fn selected_bookmark(&self) -> Option<&Bookmark> {
		self.bookmark_view_state
			.selected()
			.and_then(|i| self.bookmarks.get(i))
	}

	/// Update the selection of the parent to match the current path
	pub fn update_parent_selection(&mut self, current_path: BrowserPath) {
		let mut new_stack = vec![];
		let mut path = current_path;
		new_stack.push(BrowserStackItem::BrowserPath(path.clone()));
		while let Some(parent) = path.parent() {
			new_stack.push(BrowserStackItem::BrowserPath(parent.clone()));
			if let Some(PathData::List(list)) = self.path_data.get_mut(&parent) {
				if let Some(pos) = list.list.iter().position(|x| x == path.0.last().unwrap()) {
					list.cursor = pos;
				}
			}
			path = parent;
		}
		new_stack.push(BrowserStackItem::Root);
		new_stack.reverse();
		*self.visit_stack = new_stack;
	}
}

#[derive(Default, Debug)]
pub struct PathDataMap(HashMap<BrowserPath, PathData>);

impl Deref for PathDataMap {
	type Target = HashMap<BrowserPath, PathData>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl DerefMut for PathDataMap {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl PathDataMap {
	pub fn current_list(&self, current_path: &BrowserPath) -> Option<&ListData> {
		self.get(current_path).and_then(|x| match x {
			PathData::List(data) => Some(data),
			_ => None,
		})
	}
	pub fn current_list_mut(&mut self, current_path: &BrowserPath) -> Option<&mut ListData> {
		self.get_mut(&current_path).and_then(|x| match x {
			PathData::List(data) => Some(data),
			_ => None,
		})
	}
}

#[derive(Debug, Default)]
pub struct BrowserStack(pub Vec<BrowserStackItem>);

impl Deref for BrowserStack {
	type Target = Vec<BrowserStackItem>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl DerefMut for BrowserStack {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl BrowserStack {
	pub fn push_path(&mut self, path: BrowserPath) {
		self.0.push(BrowserStackItem::BrowserPath(path))
	}
	pub fn prev_item(&self) -> Option<&BrowserStackItem> {
		self.0.len().checked_sub(2).and_then(|i| self.0.get(i))
	}
	pub fn current(&self) -> Option<&BrowserPath> {
		match self.0.last() {
			Some(BrowserStackItem::BrowserPath(p)) => Some(p),
			_ => None,
		}
	}
	pub fn current_force(&self) -> &BrowserPath {
		match self.0.last() {
			Some(BrowserStackItem::BrowserPath(p)) => p,
			_ => panic!("current visit stack item is not a path"),
		}
	}
}

#[derive(Debug)]
pub enum Message {
	Data(BrowserPath, PathData),
	CurrentPath(BrowserPath),
	SearchEnter,
	SearchExit,
	SearchInput(KeyEvent),
	NavigatorEnter,
	NavigatorExit,
	NavigatorInput(KeyEvent),
	Back,
	Enter,
	ListUp,
	ListDown,
	Quit,
}

#[derive(Debug, Clone)]
pub enum BrowserStackItem {
	Root,
	Bookmarks,
	Recents,
	BrowserPath(BrowserPath),
}

#[derive(Debug, Default, Eq, Hash, PartialEq, Clone)]
pub struct BrowserPath(pub Vec<String>);

impl BrowserPath {
	pub fn parent(&self) -> Option<BrowserPath> {
		if self.0.len() > 1 {
			Some(BrowserPath(self.0[..self.0.len() - 1].to_vec()))
		} else {
			None
		}
	}
	pub fn child(&self, name: String) -> BrowserPath {
		let mut clone = self.0.clone();
		clone.push(name);
		BrowserPath(clone)
	}
	pub fn extend(mut self, other: &BrowserPath) -> BrowserPath {
		self.0.extend_from_slice(&other.0);
		self
	}
	pub fn to_expr(&self) -> String {
		self.0.join(".")
	}
}

impl From<String> for BrowserPath {
	fn from(value: String) -> Self {
		BrowserPath(value.split(".").map(|x| x.to_string()).collect())
	}
}

#[derive(Default, Debug, PartialEq, Eq)]
pub enum RunningState {
	#[default]
	Running,
	Stopped,
}

#[derive(Debug)]
pub struct ListData {
	pub cursor: usize,
	pub list: Vec<String>,
}

impl ListData {
	pub fn selected(&self, current_path: &BrowserPath) -> BrowserPath {
		let x = &self.list[self.cursor];
		current_path.child(x.to_string())
	}
}

#[derive(Debug)]
pub enum PathData {
	List(ListData),
	Thunk,
	Int(i64),
	Float(f64),
	Bool(bool),
	String(String),
	Path(String),
	Null,
	Function,
	External,
}

impl fmt::Display for PathData {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			PathData::List(list_data) => write!(f, "{:?}", list_data),
			PathData::Thunk => write!(f, "Thunk"),
			PathData::Int(value) => write!(f, "{}", value),
			PathData::Float(value) => write!(f, "{}", value),
			PathData::Bool(value) => write!(f, "{}", value),
			PathData::String(value) => write!(f, "\"{}\"", value),
			PathData::Path(value) => write!(f, "Path(\"{}\")", value),
			PathData::Null => write!(f, "Null"),
			PathData::Function => write!(f, "Function"),
			PathData::External => write!(f, "External"),
		}
	}
}

impl From<NixValue> for PathData {
	fn from(value: NixValue) -> Self {
		match value {
			NixValue::Thunk => PathData::Thunk,
			NixValue::Int(i) => PathData::Int(i),
			NixValue::Float(f) => PathData::Float(f),
			NixValue::Bool(b) => PathData::Bool(b),
			NixValue::String(s) => PathData::String(s),
			NixValue::Path(p) => PathData::Path(p),
			NixValue::Null => PathData::Null,
			NixValue::Attrs(attrs) => PathData::List(ListData {
				cursor: 0,
				list: attrs,
			}),
			NixValue::List(size) => PathData::List(ListData {
				cursor: 0,
				list: (0..size).map(|i| format!("{}", i)).collect(),
			}),
			NixValue::Function => PathData::Function,
			NixValue::External => PathData::External,
		}
	}
}

impl PathData {
	pub fn get_type(&self) -> String {
		match self {
			PathData::List(_) => "List",
			PathData::Thunk => "Thunk",
			PathData::Int(_) => "Int",
			PathData::Float(_) => "Float",
			PathData::Bool(_) => "Bool",
			PathData::String(_) => "String",
			PathData::Path(_) => "Path",
			PathData::Null => "Null",
			PathData::Function => "Function",
			PathData::External => "External",
		}
		.to_string()
	}
}

#[derive(Debug, Clone)]
pub struct Bookmark {
	pub display: String,
	pub path: BrowserPath,
}

impl<'a> Into<Text<'a>> for Bookmark {
	fn into(self) -> Text<'a> {
		Text::raw(self.display)
	}
}

#[derive(Debug, Default)]
pub enum InputState {
	#[default]
	Normal,
	Active(InputModel),
}

#[derive(Debug)]
pub struct InputModel {
	pub typing: bool,
	pub input: String,
	pub cursor_position: usize,
}

impl InputModel {
	pub fn handle_key_event(&mut self, key: KeyEvent) {
		match key.code {
			KeyCode::Char(c) => {
				self.insert(c);
			}
			KeyCode::Backspace => {
				self.backspace();
			}
			KeyCode::Left => {
				self.move_cursor_left();
			}
			KeyCode::Right => {
				self.move_cursor_right();
			}
			_ => {}
		}
	}

	pub fn insert(&mut self, c: char) {
		self.input.insert(self.cursor_position, c);
		self.cursor_position += 1;
	}

	pub fn backspace(&mut self) {
		if self.cursor_position == 0 {
			return;
		}

		let current_index = self.cursor_position;
		let from_left_to_current_index = current_index - 1;
		let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
		let after_char_to_delete = self.input.chars().skip(current_index);
		self.input = before_char_to_delete.chain(after_char_to_delete).collect();
		self.move_cursor_left();
	}

	pub fn move_cursor_left(&mut self) {
		self.cursor_position = self.clamp_cursor(self.cursor_position - 1);
	}

	pub fn move_cursor_right(&mut self) {
		self.cursor_position = self.clamp_cursor(self.cursor_position + 1);
	}

	fn clamp_cursor(&mut self, pos: usize) -> usize {
		pos.clamp(0, self.input.len())
	}
}

pub fn next(i: usize, len: usize) -> usize {
	if i >= len - 1 {
		0
	} else {
		i + 1
	}
}

pub fn select_next(list_state: &mut ListState, len: usize) {
	list_state.select(list_state.selected().map(|i| next(i, len)).or(Some(0)));
}

pub fn prev(i: usize, len: usize) -> usize {
	if i == 0 {
		len - 1
	} else {
		i - 1
	}
}

pub fn select_prev(list_state: &mut ListState, len: usize) {
	list_state.select(list_state.selected().map(|i| prev(i, len)).or(Some(0)));
}

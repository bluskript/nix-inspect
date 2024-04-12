use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use crate::model::{BrowserPath, PathData};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum NixValue {
	#[serde(rename = "0")]
	Thunk,
	#[serde(rename = "1")]
	Int(i64),
	#[serde(rename = "2")]
	Float(f64),
	#[serde(rename = "3")]
	Bool(bool),
	#[serde(rename = "4")]
	String(String),
	#[serde(rename = "5")]
	Path(String),
	#[serde(rename = "6")]
	Null,
	#[serde(rename = "7")]
	Attrs(Vec<String>),
	#[serde(rename = "8")]
	List(usize),
	#[serde(rename = "9")]
	Function,
	#[serde(rename = "10")]
	External,
	#[serde(rename = "11")]
	Error(String),
}

pub const WORKER_BINARY_PATH: &str = env!("WORKER_BINARY_PATH");

pub struct WorkerHost {
	pub tx: kanal::Sender<BrowserPath>,
	pub rx: kanal::Receiver<(BrowserPath, PathData)>,
}

impl WorkerHost {
	pub fn new(expr: String) -> WorkerHost {
		let (tx, rx) = kanal::unbounded::<BrowserPath>();
		let (result_tx, result_rx) = kanal::unbounded();

		let rx = rx.clone();
		let result_tx = result_tx.clone();
		std::thread::spawn(move || {
			let mut child = Command::new(WORKER_BINARY_PATH)
				.stdin(Stdio::piped())
				.stdout(Stdio::piped())
				// .stderr(Stdio::piped())
				.spawn()
				.expect("Failed to spawn worker");

			let mut stdin = child.stdin.take().expect("Failed to open stdin");
			let stdout = child.stdout.take().expect("Failed to open stdout");
			let mut reader = BufReader::new(stdout);

			let _ = writeln!(stdin, "{}", expr);

			loop {
				let received = rx.recv();
				tracing::info!("{:?}", received);
				match received {
					Ok(path) => {
						result_tx
							.send((path.clone(), PathData::Loading))
							.expect("Failed to send loading state");
						if let Err(e) = writeln!(stdin, "{}", path.to_expr()) {
							tracing::error!("Failed to send path, {e}");
							break;
						}

						let mut response = String::new();
						if let Err(e) = reader.read_line(&mut response) {
							tracing::error!("Failed to read response: {e}");
							let _ = result_tx.send((
								path,
								PathData::Error(format!("Failed to read response: {e}")),
							));
							continue;
						}

						let value: NixValue = match serde_json::from_str(&response) {
							Ok(v) => v,
							Err(e) => {
								tracing::error!("{response}");
								tracing::error!("Failed to deserialize response: {e}");
								let _ = result_tx.send((
									path,
									PathData::Error(format!("Failed to deserialize response: {e}")),
								));
								continue;
							}
						};

						result_tx
							.send((path, value.into()))
							.expect("Failed to send result");
					}
					Err(_) => {
						// Channel closed, exit the loop
						break;
					}
				}
			}

			child.kill().expect("killing child failed");
		});

		WorkerHost { tx, rx: result_rx }
	}
}

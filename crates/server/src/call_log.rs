//! Every tool call of every seat, one JSON line each: with the match record, what the assessment reads.

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::{Deserialize, Serialize};

/// One tool call as logged.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CallEntry {
    /// Milliseconds since the server started.
    pub at_ms: u64,
    /// The seat calling: `home` or `away`.
    pub seat: String,
    /// The tool called.
    pub tool: String,
    /// The arguments as sent.
    pub arguments: serde_json::Value,
    /// Milliseconds the call took, waiting included.
    pub latency_ms: u64,
    /// The refusal returned to the agent, if the call failed.
    pub error: Option<String>,
    /// Version of the match after the call.
    pub version: u64,
}

/// Where tool calls are written, shared by every request.
#[derive(Clone)]
pub struct CallLog {
    sink: Arc<Mutex<Box<dyn Write + Send>>>,
    started: Instant,
}

impl CallLog {
    /// A log writing one JSON line per call to `writer`, timed from now.
    pub fn to_writer(writer: impl Write + Send + 'static) -> Self {
        Self {
            sink: Arc::new(Mutex::new(Box::new(writer))),
            started: Instant::now(),
        }
    }

    /// A log that keeps nothing.
    pub fn discard() -> Self {
        Self::to_writer(std::io::sink())
    }

    /// Milliseconds since the log was opened.
    pub fn elapsed_ms(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Writes `entry` as one line; a failing disk never fails the call being logged.
    pub fn record(&self, entry: &CallEntry) {
        let written = serde_json::to_string(entry)
            .map_err(std::io::Error::other)
            .and_then(|line| {
                let mut sink = self
                    .sink
                    .lock()
                    .map_err(|_| std::io::Error::other("the call log was poisoned"))?;
                writeln!(sink, "{line}")?;
                sink.flush()
            });
        if let Err(problem) = written {
            eprintln!("call log: {problem}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn entry(tool: &str) -> CallEntry {
        CallEntry {
            at_ms: 1_200,
            seat: "home".to_string(),
            tool: tool.to_string(),
            arguments: serde_json::json!({"end_turn": true}),
            latency_ms: 35,
            error: None,
            version: 4,
        }
    }

    fn written(buffer: &SharedBuffer) -> String {
        String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap()
    }

    #[test]
    fn given_two_calls_when_logged_then_two_json_lines_in_order() {
        let buffer = SharedBuffer::default();
        let log = CallLog::to_writer(buffer.clone());
        log.record(&entry("get_state"));
        log.record(&entry("play"));
        let tools: Vec<String> = written(&buffer)
            .lines()
            .map(|line| serde_json::from_str::<CallEntry>(line).unwrap().tool)
            .collect();
        assert_eq!(tools, ["get_state", "play"]);
    }

    #[test]
    fn given_a_call_when_logged_then_the_line_reads_back_as_the_same_entry() {
        let buffer = SharedBuffer::default();
        CallLog::to_writer(buffer.clone()).record(&entry("play"));
        let line = written(&buffer);
        assert_eq!(
            serde_json::from_str::<CallEntry>(line.trim_end()).unwrap(),
            entry("play")
        );
    }
}

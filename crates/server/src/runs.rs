//! Files of one run, side by side: the match record and the log of every tool call.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::Context;
use goblins_rules::state::Phase;

use crate::call_log::CallLog;
use crate::match_server::MatchClient;

const FINAL_WHISTLE_POLL: Duration = Duration::from_secs(3_600);

/// Where a run keeps its match record and its tool calls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunFiles {
    /// The match record, rewritten whenever it is saved.
    pub record: PathBuf,
    /// The tool calls, one JSON line each, appended as they happen.
    pub calls: PathBuf,
}

impl RunFiles {
    /// Creates `folder` if needed, names the files of a run started now with `seed`, and opens the
    /// call log for appending.
    ///
    /// # Errors
    /// The folder cannot be created or the call log cannot be opened.
    pub fn open(folder: &Path, seed: u64) -> anyhow::Result<(Self, CallLog)> {
        std::fs::create_dir_all(folder)
            .with_context(|| format!("cannot create {}", folder.display()))?;
        let name = format!("{}-seed{seed}", seconds_since_epoch());
        let files = Self {
            record: folder.join(format!("{name}.jsonl")),
            calls: folder.join(format!("{name}-calls.jsonl")),
        };
        let calls = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&files.calls)
            .with_context(|| format!("cannot open {}", files.calls.display()))?;
        Ok((files, CallLog::to_writer(std::io::LineWriter::new(calls))))
    }
}

/// Seconds since the Unix epoch, zero if the clock is before it.
pub fn seconds_since_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default()
}

/// Saves the record of the match so far to `file`, telling the console which `moment` it was.
pub async fn save_record(client: &MatchClient, file: &Path, moment: &str) {
    let saved = async {
        let record = client.record().await?;
        std::fs::write(file, record.to_json_lines() + "\n")?;
        anyhow::Ok(())
    }
    .await;
    match saved {
        Ok(()) => println!("{moment}: record saved to {}", file.display()),
        Err(error) => eprintln!("{moment}: the record could not be saved: {error}"),
    }
}

/// Waits for the final whistle, then saves the record to `file`.
pub async fn save_record_at_final_whistle(client: MatchClient, file: PathBuf) {
    loop {
        let snapshot = client
            .wait_until(FINAL_WHISTLE_POLL, |snapshot| {
                snapshot.state.phase == Phase::Finished
            })
            .await;
        if snapshot.state.phase == Phase::Finished {
            break;
        }
    }
    save_record(&client, &file, "Final whistle").await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_folder(name: &str) -> PathBuf {
        let folder =
            std::env::temp_dir().join(format!("goblinsbowl-runs-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        folder
    }

    #[test]
    fn given_a_missing_folder_when_opening_a_run_then_the_call_log_is_created_inside() {
        let folder = fresh_folder("calls");
        let (files, _) = RunFiles::open(&folder, 42).unwrap();
        assert!(files.calls.starts_with(&folder) && files.calls.exists());
    }

    #[test]
    fn given_a_seed_when_opening_a_run_then_the_record_is_named_after_it() {
        let folder = fresh_folder("record");
        let (files, _) = RunFiles::open(&folder, 42).unwrap();
        assert!(
            files
                .record
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("-seed42.jsonl"))
        );
    }
}

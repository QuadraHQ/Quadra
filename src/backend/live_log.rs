use crate::state::AppState;
use gtk::glib;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const CHECK_FOR_NEW_OUTPUT_EVERY: Duration = Duration::from_millis(200);
const READ_CHUNK_SIZE: usize = 8192;

pub fn follow(state: &AppState, log: File, writer_finished: Arc<AtomicBool>) {
    state.clear_log();

    let (output_sender, output_receiver) = async_channel::unbounded();
    std::thread::spawn(move || read_until_writer_finishes(log, &writer_finished, &output_sender));

    let state = state.clone();
    glib::spawn_future_local(async move {
        while let Ok(mut output) = output_receiver.recv().await {
            while let Ok(more_output) = output_receiver.try_recv() {
                output.push_str(&more_output);
            }
            state.append_log(&output);
        }
    });
}

fn read_until_writer_finishes(
    mut log: File,
    writer_finished: &AtomicBool,
    output_sender: &async_channel::Sender<String>,
) {
    let mut unfinished_line = Vec::new();
    let mut chunk = [0; READ_CHUNK_SIZE];

    loop {
        let writer_had_finished = writer_finished.load(Ordering::Acquire);
        let read = match log.read(&mut chunk) {
            Ok(read) => read,
            Err(error) => {
                eprintln!("Stopped following the Roblox log: {error}");
                break;
            }
        };

        if read == 0 {
            if writer_had_finished {
                break;
            }
            std::thread::sleep(CHECK_FOR_NEW_OUTPUT_EVERY);
            continue;
        }

        unfinished_line.extend_from_slice(&chunk[..read]);
        if let Some(last_newline) = unfinished_line.iter().rposition(|&byte| byte == b'\n') {
            let complete_lines: Vec<u8> = unfinished_line.drain(..=last_newline).collect();
            if output_sender
                .send_blocking(as_text(&complete_lines))
                .is_err()
            {
                return;
            }
        }
    }

    if !unfinished_line.is_empty() {
        let _ = output_sender.send_blocking(as_text(&unfinished_line));
    }
}

fn as_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

use crate::protocol::Request;
use serde_json::Value;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc;
use std::thread;

const OWNER_ONLY: u32 = 0o600;

pub enum Incoming {
    Connected(Reply),
    Request(Request),
    Disconnected,
}

#[derive(Clone)]
pub struct Reply(mpsc::Sender<String>);

impl Reply {
    pub fn send(&self, message: Value) {
        let _ = self.0.send(format!("{message}\n"));
    }
}

pub fn listen(socket: &Path) -> io::Result<async_channel::Receiver<Incoming>> {
    if socket.exists() {
        fs::remove_file(socket)?;
    }
    let listener = UnixListener::bind(socket)?;
    fs::set_permissions(socket, fs::Permissions::from_mode(OWNER_ONLY))?;

    let (sender, receiver) = async_channel::unbounded();
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            if let Err(error) = serve(stream, &sender) {
                eprintln!("quadra-web-window: the connection failed: {error}");
            }
            if sender.send_blocking(Incoming::Disconnected).is_err() {
                break;
            }
        }
    });

    Ok(receiver)
}

fn serve(stream: UnixStream, sender: &async_channel::Sender<Incoming>) -> io::Result<()> {
    let (reply, queued) = mpsc::channel::<String>();
    let mut writer = stream.try_clone()?;
    thread::spawn(move || {
        for line in queued {
            if writer.write_all(line.as_bytes()).is_err() {
                break;
            }
        }
    });
    if sender
        .send_blocking(Incoming::Connected(Reply(reply)))
        .is_err()
    {
        return Ok(());
    }

    for line in BufReader::new(stream).lines() {
        match serde_json::from_str(&line?) {
            Ok(request) => {
                if sender.send_blocking(Incoming::Request(request)).is_err() {
                    break;
                }
            }
            Err(error) => eprintln!(
                "quadra-web-window: ignored a request: {:?} error",
                error.classify()
            ),
        }
    }

    Ok(())
}

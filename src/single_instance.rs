use crate::command_line::Invocation;
use crate::utils::runtime_directory;
use iced::futures::Stream;
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const SOCKET_NAME: &str = "quadra-instance";
const FIELD_SEPARATOR: char = '\0';

static FORWARDED: OnceLock<async_channel::Receiver<Invocation>> = OnceLock::new();

pub enum Claim {
    Primary,
    Forwarded,
}

pub fn claim_or_forward(invocation: &Invocation) -> Claim {
    let socket = socket_path();
    match UnixStream::connect(&socket) {
        Ok(primary) => match forward(primary, invocation) {
            Ok(()) => return Claim::Forwarded,
            Err(error) => eprintln!("Couldn't hand over to the running Quadra: {error}"),
        },
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::ConnectionRefused | ErrorKind::NotFound
            ) =>
        {
            let _ = std::fs::remove_file(&socket);
        }
        Err(error) => eprintln!("Couldn't reach a running Quadra: {error}"),
    }

    if let Err(error) = listen(&socket) {
        eprintln!("Quadra can't receive links while it's open: {error}");
    }
    Claim::Primary
}

pub fn forwarded_invocations() -> impl Stream<Item = Invocation> {
    FORWARDED
        .get()
        .cloned()
        .unwrap_or_else(|| async_channel::unbounded().1)
}

fn forward(mut primary: UnixStream, invocation: &Invocation) -> io::Result<()> {
    let mut fields = vec![invocation.directory.display().to_string()];
    fields.extend(invocation.arguments.iter().cloned());
    primary.write_all(fields.join(&FIELD_SEPARATOR.to_string()).as_bytes())?;
    primary.shutdown(std::net::Shutdown::Write)
}

fn listen(socket: &Path) -> io::Result<()> {
    if let Some(directory) = socket.parent() {
        std::fs::create_dir_all(directory)?;
    }
    let listener = UnixListener::bind(socket)?;
    let (sender, receiver) = async_channel::unbounded();
    let _ = FORWARDED.set(receiver);

    std::thread::spawn(move || {
        for connection in listener.incoming() {
            let invocation = connection.and_then(|mut connection| {
                let mut received = String::new();
                connection.read_to_string(&mut received)?;
                Ok(invocation_from(&received))
            });
            match invocation {
                Ok(invocation) => {
                    if sender.send_blocking(invocation).is_err() {
                        return;
                    }
                }
                Err(error) => eprintln!("Couldn't read what another Quadra sent: {error}"),
            }
        }
    });
    Ok(())
}

fn invocation_from(received: &str) -> Invocation {
    let mut fields = received.split(FIELD_SEPARATOR).map(str::to_owned);
    Invocation {
        directory: fields.next().map(PathBuf::from).unwrap_or_default(),
        arguments: fields.collect(),
    }
}

fn socket_path() -> PathBuf {
    let directory = match std::env::var("FLATPAK_ID") {
        Ok(flatpak_id) => runtime_directory().join("app").join(flatpak_id),
        Err(_) => runtime_directory(),
    };
    directory.join(SOCKET_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_invocation_survives_the_trip() {
        let invocation = invocation_from("/home/someone");
        assert_eq!(invocation.directory, PathBuf::from("/home/someone"));
        assert!(invocation.arguments.is_empty());
    }

    #[test]
    fn arguments_survive_the_trip() {
        let invocation = invocation_from("/tmp\0--studio\0My Place.rbxl");
        assert_eq!(invocation.arguments, ["--studio", "My Place.rbxl"]);
    }
}

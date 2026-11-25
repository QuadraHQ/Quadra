mod browser;
mod connection;
mod cookies;
mod page;
mod protocol;

use anyhow::Context;
use browser::Browser;
use connection::Incoming;
use gtk::glib;
use std::env;
use std::path::PathBuf;

const APP_ID: &str = "org.quadrahq.Quadra";
const USAGE: &str = "Usage: quadra-web-window <socket> <data directory>";

fn main() -> anyhow::Result<()> {
    let mut arguments = env::args_os().skip(1).map(PathBuf::from);
    let (Some(socket), Some(data_directory)) = (arguments.next(), arguments.next()) else {
        anyhow::bail!(USAGE);
    };

    glib::set_prgname(Some(APP_ID));
    adw::init().context("Couldn't open a display for web pages")?;

    let incoming = connection::listen(&socket)
        .with_context(|| format!("Couldn't listen on {}", socket.display()))?;
    let browser = Browser::new(&data_directory)?;
    let main_loop = glib::MainLoop::new(None, false);

    glib::spawn_future_local(async move {
        while let Ok(incoming) = incoming.recv().await {
            match incoming {
                Incoming::Connected(reply) => browser.connected(reply),
                Incoming::Request(request) => browser.handle(request),
                Incoming::Disconnected => browser.disconnected(),
            }
        }
    });

    main_loop.run();
    Ok(())
}

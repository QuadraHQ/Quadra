use anyhow::Context;
use flate2::Compression;
use flate2::write::GzEncoder;
use std::fs::File;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tar::{Builder, Header};

pub struct Archive {
    builder: Builder<GzEncoder<File>>,
}

impl Archive {
    pub fn create(path: &Path) -> anyhow::Result<Self> {
        let file =
            File::create(path).with_context(|| format!("Couldn't create {}", path.display()))?;
        Ok(Self {
            builder: Builder::new(GzEncoder::new(file, Compression::default())),
        })
    }

    pub fn add(&mut self, name: &str, contents: &str) -> anyhow::Result<()> {
        let mut header = Header::new_gnu();
        header.set_size(contents.len() as u64);
        header.set_mode(0o644);
        header.set_mtime(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_secs()),
        );
        self.builder
            .append_data(&mut header, name, contents.as_bytes())
            .with_context(|| format!("Couldn't add {name} to the archive"))
    }

    pub fn finish(self) -> anyhow::Result<()> {
        self.builder.into_inner()?.finish()?;
        Ok(())
    }
}

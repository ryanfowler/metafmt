use std::{
    env, fs,
    io::{self, IsTerminal, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use flate2::read::GzDecoder;
use rand::distr::{Alphanumeric, SampleString};
use serde::Deserialize;
use tar::Archive;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};
use ureq::{Agent, AgentBuilder};

type Error = Box<dyn std::error::Error>;

static TARGET: &str = env!("TARGET");
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn update() -> i32 {
    let writer = new_writer();
    if let Err(err) = update_in_place(&writer) {
        _ = write_error(&writer, &err.to_string());
        1
    } else {
        0
    }
}

fn update_in_place(writer: &BufferWriter) -> Result<(), Error> {
    let agent = AgentBuilder::new()
        .user_agent(&format!("metafmt/{VERSION}"))
        .build();

    let latest = get_latest_info(writer, &agent)?;
    if latest.trim_start_matches('v') == VERSION {
        let msg = format!("  already using the latest version (v{VERSION})");
        return write_raw(writer, &msg);
    }

    let temp_dir = TempDir::new()?;
    let reader = download_artifact(writer, &agent, &latest)?;
    unpack_artifact(&temp_dir.0, reader)?;

    let exe_path = env::current_exe()?;
    let src = Path::new(&temp_dir.0).join("metafmt");
    fs::rename(src, exe_path)?;

    let msg = format!("  metafmt successfully updated ({latest})");
    write_raw(writer, &msg)
}

fn new_writer() -> BufferWriter {
    BufferWriter::stderr(if io::stderr().is_terminal() {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    })
}

fn get_latest_info(writer: &BufferWriter, agent: &Agent) -> Result<String, Error> {
    #[derive(Deserialize)]
    struct Release {
        tag_name: String,
    }

    _ = write_info(writer, "fetching latest release metadata");
    let res = agent
        .get("https://api.github.com/repos/ryanfowler/metafmt/releases/latest")
        .timeout(Duration::from_secs(30))
        .call()?;

    let status = res.status();
    if status != 200 {
        Err(format!("fetching release metadata: received status: {status}").into())
    } else {
        let out: Release = res.into_json()?;
        Ok(out.tag_name)
    }
}

fn download_artifact(writer: &BufferWriter, agent: &Agent, tag: &str) -> Result<impl Read, Error> {
    _ = write_info(writer, &format!("downloading artifact for version: {tag}"));
    let url = format!("https://github.com/ryanfowler/metafmt/releases/download/{tag}/metafmt-{tag}-{TARGET}.tar.gz");
    let response = agent.get(&url).timeout(Duration::from_secs(300)).call()?;

    let status = response.status();
    if status != 200 {
        Err(format!("downloading artifact: received status: {status}").into())
    } else {
        Ok(response.into_reader())
    }
}

fn unpack_artifact(temp_dir: &PathBuf, r: impl Read) -> Result<(), io::Error> {
    let gz = GzDecoder::new(r);
    let mut archive = Archive::new(gz);
    archive.unpack(temp_dir)
}

fn write_raw(writer: &BufferWriter, msg: &str) -> Result<(), Error> {
    let mut buf = writer.buffer();
    writeln!(&mut buf, "{msg}")?;
    Ok(writer.print(&buf)?)
}

fn write_info(writer: &BufferWriter, msg: &str) -> Result<(), Error> {
    let mut buf = writer.buffer();
    buf.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true))?;
    write!(buf, "info:")?;
    buf.reset()?;
    writeln!(&mut buf, " {msg}")?;
    Ok(writer.print(&buf)?)
}

fn write_error(writer: &BufferWriter, msg: &str) -> Result<(), Error> {
    let mut buf = writer.buffer();
    buf.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
    write!(&mut buf, "error:")?;
    buf.reset()?;
    writeln!(&mut buf, " {msg}")?;
    Ok(writer.print(&buf)?)
}

/// TempDir represents a new temporary directory created with a random name.
/// Upon being dropped, the directory is removed.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self, io::Error> {
        let mut dir = env::temp_dir();
        let sample = Alphanumeric.sample_string(&mut rand::rng(), 16);
        dir.push(format!("metafmt-{sample}"));
        fs::create_dir(&dir)?;
        Ok(Self(dir))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        _ = fs::remove_dir_all(&self.0);
    }
}

use std::{
    env, fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use flate2::read::GzDecoder;
use rand::distributions::{Alphanumeric, DistString};
use serde::Deserialize;
use tar::Archive;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};
use ureq::{Agent, AgentBuilder};

type Error = Box<dyn std::error::Error>;

static TARGET: &str = env!("TARGET");
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn update() -> i32 {
    let writer = new_writer();
    update_in_place(&writer).map_or_else(
        |err| {
            write_error(&writer, &err.to_string());
            1
        },
        |new_version| {
            let mut buf = writer.buffer();
            _ = writeln!(&mut buf);
            if let Some(new_version) = new_version {
                _ = writeln!(&mut buf, "  metafmt successfully updated ({new_version})");
            } else {
                _ = writeln!(&mut buf, "  already using the latest version (v{VERSION})");
            }
            _ = writer.print(&buf);
            0
        },
    )
}

fn update_in_place(writer: &BufferWriter) -> Result<Option<String>, Error> {
    let agent = AgentBuilder::new()
        .timeout(Duration::from_secs(300))
        .user_agent(&format!("metafmt/{VERSION}"))
        .build();

    let latest = get_latest_info(writer, &agent)?;
    if latest.trim_start_matches('v') == VERSION {
        return Ok(None);
    }

    let temp_dir = TempDir::new()?;
    let reader = download_artifact(writer, &agent, &latest)?;
    unpack_artifact(&temp_dir.0, reader)?;

    let exe_path = env::current_exe()?;
    let src = Path::new(&temp_dir.0).join("metafmt");
    fs::rename(src, exe_path)?;

    Ok(Some(latest))
}

fn new_writer() -> BufferWriter {
    let is_atty = atty::is(atty::Stream::Stderr);
    BufferWriter::stderr(if is_atty {
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

    write_info(writer, "fetching latest release metadata");
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
    write_info(writer, &format!("downloading artifact for version: {tag}"));
    let url = format!("https://github.com/ryanfowler/metafmt/releases/download/{tag}/metafmt-{tag}-{TARGET}.tar.gz");
    let response = agent.get(&url).call()?;

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

fn write_info(writer: &BufferWriter, msg: &str) {
    let mut buf = writer.buffer();
    _ = buf.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
    _ = write!(buf, "info:");
    _ = buf.reset();
    _ = writeln!(&mut buf, " {msg}");
    _ = writer.print(&buf);
}

fn write_error(writer: &BufferWriter, msg: &str) {
    let mut buf = writer.buffer();
    _ = buf.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
    _ = write!(&mut buf, "error:");
    _ = buf.reset();
    _ = writeln!(&mut buf, " {msg}");
    _ = writer.print(&buf);
}

/// TempDir represents a new temporary directory created with a random name.
/// Upon being dropped, the directory is removed.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self, io::Error> {
        let mut dir = env::temp_dir();
        let sample = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
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

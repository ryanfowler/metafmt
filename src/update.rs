use std::{
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use flate2::read::GzDecoder;
use rand::distributions::{Alphanumeric, DistString};
use serde::Deserialize;
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};
use ureq::{Agent, AgentBuilder};

#[cfg(not(target_os = "windows"))]
static ARTIFACT: &str = "metafmt";

#[cfg(target_os = "windows")]
static ARTIFACT: &str = "metafmt.exe";

#[cfg(not(target_os = "windows"))]
static EXTENSION: &str = "tar.gz";

#[cfg(target_os = "windows")]
static EXTENSION: &str = "zip";

static TARGET: &str = env!("TARGET");
static VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn update() -> i32 {
    let writer = new_writer();
    match update_inner(&writer) {
        Ok(version) => {
            let mut buf = writer.buffer();
            _ = writeln!(&mut buf);
            if let Some(version) = version {
                _ = writeln!(&mut buf, "  metafmt updated to version: {version}");
            } else {
                _ = writeln!(&mut buf, "  metafmt already on latest version: v{VERSION}");
            }
            _ = writer.print(&buf);
            0
        }
        Err(err) => {
            write_error(&writer, &err.to_string());
            1
        }
    }
}

type Error = Box<dyn std::error::Error>;

pub(crate) fn update_inner(writer: &BufferWriter) -> Result<Option<String>, Error> {
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
    let src = Path::new(&temp_dir.0).join(ARTIFACT);
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
    struct Response {
        tag_name: String,
    }

    write_info(writer, "fetching latest version metadata");
    let res: Response = agent
        .get("https://api.github.com/repos/ryanfowler/metafmt/releases/latest")
        .call()?
        .into_json()?;

    Ok(res.tag_name)
}

fn download_artifact(writer: &BufferWriter, agent: &Agent, tag: &str) -> Result<impl Read, Error> {
    write_info(writer, &format!("downloading artifact for version: {tag}"));
    let url = format!("https://github.com/ryanfowler/metafmt/releases/download/{tag}/metafmt-{tag}-{TARGET}.{EXTENSION}");
    let response = agent.get(&url).call()?;
    let status = response.status();
    if status != 200 {
        Err(format!("downloading artifact: received status: {status}").into())
    } else {
        Ok(response.into_reader())
    }
}

#[cfg(not(target_os = "windows"))]
fn unpack_artifact(temp_dir: &PathBuf, r: impl Read) -> Result<(), std::io::Error> {
    let gz = GzDecoder::new(r);
    let mut archive = tar::Archive::new(gz);
    archive.unpack(temp_dir)
}

#[cfg(target_os = "windows")]
fn unpack_artifact(temp_dir: &PathBuf, r: impl Read) -> Result<(), std::io::Error> {
    let mut z = zip::ZipArchive::new(r);
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

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<Self, std::io::Error> {
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

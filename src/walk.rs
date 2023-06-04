use std::{
    env::temp_dir,
    ffi::OsString,
    fmt::Display,
    fs::{remove_file, rename, OpenOptions},
    io::{Read, Result, Write},
    iter::repeat_with,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use crossbeam::channel::Sender;
use diffy::{create_patch, PatchFormatter};
use ignore::{overrides::OverrideBuilder, WalkBuilder, WalkState};
use termcolor::{Buffer, BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

use crate::types::{json::Json, markdown::Markdown, sql::Sql, toml::Toml, yaml::Yaml, Format};

#[derive(Default, Clone)]
pub(crate) struct Options {
    pub(crate) hidden: bool,
    pub(crate) globs: Vec<String>,
    pub(crate) parallel: Option<usize>,
    pub(crate) diff: bool,
    pub(crate) list_all: bool,
    pub(crate) no_ignore: bool,
    pub(crate) quiet: bool,
    pub(crate) write: bool,
}

pub(crate) fn walk(root: String, ops: Options) -> i32 {
    let js = Json::default();
    let md = Markdown::default();
    let sql = Sql::default();
    let tm = Toml::default();
    let ym = Yaml::default();

    let (tx, rx) = crossbeam::channel::unbounded();

    let is_atty = atty::is(atty::Stream::Stderr);
    let writer = Arc::new(BufferWriter::stderr(if is_atty {
        ColorChoice::Always
    } else {
        ColorChoice::Never
    }));

    let walkbuilder = match build_walk(&root, &ops, writer.clone()) {
        Some(walk) => walk,
        None => return 1,
    };
    walkbuilder.build_parallel().run(|| {
        let root = root.clone();
        let ops = ops.clone();
        let writer = writer.clone();
        let mut buf = writer.buffer();
        let mut counts = ThreadCounts::new(tx.clone());
        let mut in_buf = String::with_capacity(1 << 12);
        Box::new(move |result| {
            let entry = match result {
                Ok(entry) => entry,
                Err(_) => return WalkState::Continue,
            };

            let path = entry.path();
            if !path.is_file() {
                return WalkState::Continue;
            }
            buf.clear();
            let outcome = match path.extension().and_then(std::ffi::OsStr::to_str) {
                Some("json") | Some("jsonc") | Some("hjson") | Some("jwcc") => {
                    check_file(&root, path, &mut in_buf, &mut buf, js, &ops, is_atty)
                }
                Some("md") => check_file(&root, path, &mut in_buf, &mut buf, md, &ops, is_atty),
                Some("sql") => check_file(&root, path, &mut in_buf, &mut buf, sql, &ops, is_atty),
                Some("toml") => check_file(&root, path, &mut in_buf, &mut buf, tm, &ops, is_atty),
                Some("yaml") | Some("yml") => {
                    check_file(&root, path, &mut in_buf, &mut buf, ym, &ops, is_atty)
                }
                _ => {
                    return WalkState::Continue;
                }
            };
            counts.incr_outcome(outcome);
            _ = writer.print(&buf);

            WalkState::Continue
        })
    });
    drop(tx);

    let counts = rx.into_iter().fold(Counts::default(), |mut s, c| {
        s.ok += c.ok;
        s.warn += c.warn;
        s.err += c.err;
        s
    });
    let mut buf = writer.buffer();
    let out = output(&mut buf, counts, &ops);
    _ = writer.print(&buf);
    out
}

fn build_walk(root: &str, ops: &Options, writer: Arc<BufferWriter>) -> Option<WalkBuilder> {
    let mut builder = WalkBuilder::new(root);
    let num_threads = ops.parallel.unwrap_or_else(num_cpus::get).max(1);
    builder
        .hidden(!ops.hidden)
        .threads(num_threads)
        .ignore(!ops.no_ignore)
        .git_ignore(!ops.no_ignore);
    if num_threads == 1 {
        builder.sort_by_file_path(|p1, p2| p1.cmp(p2));
    }
    if !ops.no_ignore {
        builder.add_custom_ignore_filename(".metafmtignore");
    }
    if !ops.globs.is_empty() {
        let mut override_builder = OverrideBuilder::new(root);
        for glob in &ops.globs {
            match override_builder.add(glob) {
                Ok(_) => {}
                Err(err) => {
                    let mut buf = writer.buffer();
                    print_error(&mut buf, err);
                    _ = writer.print(&buf);
                    return None;
                }
            }
        }
        let overrides = match override_builder.build() {
            Ok(ovr) => ovr,
            Err(err) => {
                let mut buf = writer.buffer();
                print_error(&mut buf, err);
                _ = writer.print(&buf);
                return None;
            }
        };
        builder.overrides(overrides);
    }
    Some(builder)
}

fn check_file(
    root: &str,
    path: &Path,
    in_buf: &mut String,
    buf: &mut Buffer,
    formatter: impl Format,
    ops: &Options,
    is_atty: bool,
) -> Outcome {
    let mut ppath = path.strip_prefix(root).unwrap_or(path);
    if ppath.as_os_str().eq_ignore_ascii_case("") {
        ppath = path;
    }
    if let Err(err) = read_file(path, in_buf) {
        if !ops.quiet {
            print_path_error(buf, ppath, &err);
        }
        return Outcome::Err;
    }

    let out = match formatter.format(in_buf) {
        Ok(out) => out,
        Err(err) => {
            if !ops.quiet {
                print_path_error(buf, ppath, &err);
            }
            return Outcome::Err;
        }
    };

    if &out == in_buf {
        if !ops.quiet && ops.list_all {
            _ = buf.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
            _ = write!(buf, "info:");
            _ = buf.reset();
            _ = writeln!(buf, "  {ppath:?}");
        }
        return Outcome::Ok;
    }

    if ops.write {
        if let Err(err) = write_file(path, out.as_bytes()) {
            if !ops.quiet {
                print_path_error(buf, ppath, &format!("writing file: {err}"));
            }
            return Outcome::Err;
        }
    }
    if !ops.quiet {
        _ = buf.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true));
        _ = write!(buf, "warn:");
        _ = buf.reset();
        _ = writeln!(buf, "  {ppath:?}");
        if ops.diff {
            let df = if is_atty {
                PatchFormatter::new().with_color()
            } else {
                PatchFormatter::default()
            };
            print_diffs(buf, &df, in_buf, &out);
        }
    }
    Outcome::Warn
}

fn read_file(path: &Path, buf: &mut String) -> std::io::Result<usize> {
    buf.clear();
    let mut file = std::fs::File::open(path)?;
    file.read_to_string(buf)
}

fn print_error(buf: &mut Buffer, err: impl Display) {
    _ = buf.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
    _ = write!(buf, "error:");
    _ = buf.reset();
    _ = write!(buf, " {err}: ");
}

fn print_path_error(buf: &mut Buffer, path: &Path, err: &impl Display) {
    _ = buf.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
    _ = write!(buf, "error:");
    _ = buf.reset();
    _ = write!(buf, " {path:?}: ");
    _ = buf.set_color(ColorSpec::new().set_dimmed(true));
    _ = writeln!(buf, "{err}");
    _ = buf.reset();
}

fn output(buf: &mut Buffer, counts: Counts, ops: &Options) -> i32 {
    if counts.err == 0 && counts.warn == 0 && counts.ok == 0 {
        _ = writeln!(buf, "No files to format");
        return 0;
    }

    if !ops.quiet && (counts.err > 0 || counts.warn > 0 || ops.list_all) {
        _ = writeln!(buf);
    }

    if counts.err > 0 {
        _ = buf.set_color(ColorSpec::new().set_fg(Some(Color::Red)));
        _ = writeln!(
            buf,
            "✗ {} error{}",
            counts.err,
            if counts.err != 1 { "s" } else { "" }
        );
        _ = buf.reset();
    }
    if counts.warn > 0 {
        _ = buf.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)));
        _ = writeln!(
            buf,
            "⚠ {} warning{}{}",
            counts.warn,
            if counts.warn != 1 { "s" } else { "" },
            if ops.write { " (rewritten)" } else { "" }
        );
        _ = buf.reset();
    }
    _ = writeln!(buf, "✓ {} okay", counts.ok);
    i32::from(counts.err > 0 || (counts.warn > 0 && !ops.write))
}

#[derive(Copy, Clone, Debug)]
enum Outcome {
    Ok,
    Warn,
    Err,
}

struct ThreadCounts {
    tx: Sender<Counts>,
    counts: Counts,
}

impl ThreadCounts {
    fn new(tx: Sender<Counts>) -> Self {
        ThreadCounts {
            tx,
            counts: Counts::default(),
        }
    }

    fn incr_outcome(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Ok => self.counts.ok += 1,
            Outcome::Warn => self.counts.warn += 1,
            Outcome::Err => self.counts.err += 1,
        }
    }
}

impl Drop for ThreadCounts {
    fn drop(&mut self) {
        self.tx.send(self.counts.clone()).unwrap();
    }
}

#[derive(Clone, Debug, Default)]
struct Counts {
    ok: usize,
    warn: usize,
    err: usize,
}

fn print_diffs(buf: &mut Buffer, f: &PatchFormatter, orig: &str, out: &str) {
    _ = buf.set_color(ColorSpec::new().set_dimmed(true));
    _ = writeln!(buf, "{:-^1$}", "-", 40);
    _ = buf.reset();
    let patch = create_patch(orig, out);
    _ = writeln!(buf, "{}", f.fmt_patch(&patch));
}

fn write_file(path: &Path, content: &[u8]) -> Result<()> {
    let temp_path = write_to_temp_file(content)?;
    if let Err(err) = rename(&temp_path, path) {
        _ = remove_file(&temp_path);
        Err(err)
    } else {
        Ok(())
    }
}

fn write_to_temp_file(content: &[u8]) -> Result<PathBuf> {
    let name: String = repeat_with(fastrand::alphanumeric).take(16).collect();
    let name = OsString::from_str(&name).unwrap();
    let temp_path = temp_dir().join(name);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(&temp_path)?;
    if let Err(err) = file.write(content) {
        _ = remove_file(&temp_path);
        Err(err)
    } else {
        Ok(temp_path)
    }
}

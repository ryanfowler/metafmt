mod fmt;
mod types;
mod update;

use clap::Parser;

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// A file or directory to format.
    #[clap(default_value = "./")]
    path: String,

    /// Show a diff for each non-formatted file.
    #[clap(short, long, default_missing_value = "true")]
    diff: bool,

    /// Include or exclude files to format.
    #[clap(short, long)]
    glob: Vec<String>,

    /// Include hidden files and directories.
    #[clap(short = '.', long, default_missing_value = "true")]
    hidden: bool,

    /// List all files processed, including formatted ones.
    #[clap(short, long, default_missing_value = "true")]
    list_all: bool,

    /// Disable all ignore-related filtering.
    #[clap(long, default_missing_value = "true")]
    no_ignore: bool,

    /// The approximate number of threads to use.
    #[clap(short, long)]
    parallel: Option<usize>,

    /// Do not print info to stderr.
    #[clap(short, long, default_missing_value = "true")]
    quiet: bool,

    /// Update metafmt to the latest version.
    #[clap(short, long, default_missing_value = "true")]
    update: bool,

    /// Rewrite files in-place.
    #[clap(short, long, default_missing_value = "true")]
    write: bool,
}

fn main() {
    let cli = Cli::parse();

    let exit_code = if cli.update {
        update::update()
    } else {
        fmt::format(
            cli.path,
            fmt::Options {
                hidden: cli.hidden,
                globs: cli.glob,
                parallel: cli.parallel,
                diff: cli.diff,
                list_all: cli.list_all,
                no_ignore: cli.no_ignore,
                quiet: cli.quiet,
                write: cli.write,
            },
        )
    };

    std::process::exit(exit_code);
}

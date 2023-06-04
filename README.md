# metafmt

`metafmt` is a highly performant and opinionated formatter for the following
configuration and text formats:

- json (`.json`, `.jsonc`, `.hjson`, `.jwcc`)
- markdown (`.md`)
- sql (`.sql`)
- toml (`.toml`)
- yaml (`.yaml`, `.yml`)

### Install from source

Requirements:

- Go >= 1.20
- Rust >= 1.70

```sh
cargo install metafmt --locked --force
```

### Usage

`> metafmt -h`

```
A CLI for formatting configuration files

Usage: metafmt [OPTIONS] [PATH]

Arguments:
  [PATH]  A file or directory to format [default: ./]

Options:
  -d, --diff                 Show a diff for each non-formatted file
  -g, --glob <GLOB>          Include or exclude files to format
  -., --hidden               Include hidden files and directories
  -l, --list-all             List all files processed, including formatted ones
      --no-ignore            Disable all ignore-related filtering
  -p, --parallel <PARALLEL>  The approximate number of threads to use
  -q, --quiet                Do not print info to stderr
  -w, --write                Rewrite files in-place
  -h, --help                 Print help
  -V, --version              Print version
```

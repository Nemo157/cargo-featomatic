extern crate cargo;
extern crate rustc_serialize;
extern crate itertools;

use std::iter::FromIterator;

use cargo::core::Workspace;
use cargo::util::{ human, CliResult, CliError, Config };
use cargo::util::important_paths::find_root_manifest_for_wd;
use cargo::util::process_builder::process;
use itertools::Itertools;

const USAGE: &'static str = "
Run some cargo command over all combinations of features

TODO: This usage string is buggy, can't actually parse the options.....
Usage:
    cargo featomatic [options] [--] <args>...

Options:
    -h, --help              Print this message
    -V, --version           Print version info and exit
    -v, --verbose ...       Use verbose output (-vv very verbose/build.rs output)
    -q, --quiet             No output printed to stdout
    --manifest-path PATH    Path to the manifest to analyze
    --color WHEN            Coloring: auto, always, never
    --frozen                Require Cargo.lock and cache are up to date
    --locked                Require Cargo.lock is up to date

All of the trailing arguments are passed to a series of invocations of cargo
with a different combination of features appended each time. It is assumed that
the command specified will support --no-default-features and --features command
line arguments.
";

#[derive(RustcDecodable)]
struct Options {
    arg_args: Vec<String>,
    flag_version: bool,
    flag_verbose: u32,
    flag_quiet: Option<bool>,
    flag_manifest_path: Option<String>,
    flag_color: Option<String>,
    flag_frozen: bool,
    flag_locked: bool,
}

fn main() {
    cargo::execute_main_without_stdin(real_main, true, USAGE);
}

fn real_main(options: Options, config: &Config) -> CliResult<Option<()>> {
    config.configure(
        options.flag_verbose,
        options.flag_quiet,
        &options.flag_color,
        options.flag_frozen,
        options.flag_locked)?;

    if options.flag_version {
        config.shell().say(format!("cargo-featomatic {}", env!("CARGO_PKG_VERSION")), 0)?;
        return Ok(None);
    }

    let base_args = {
        let mut base_args = options.arg_args;
        for _ in 0..options.flag_verbose {
            base_args.push("--verbose".to_owned());
        }
        if options.flag_quiet == Some(true) {
            base_args.push("--quiet".to_owned());
        }
        if let Some(ref manifest_path) = options.flag_manifest_path {
            base_args.push("--manifest-path".to_owned());
            base_args.push(manifest_path.clone());
        }
        if let Some(ref color) = options.flag_color {
            base_args.push("--color".to_owned());
            base_args.push(color.clone());
        }
        if options.flag_frozen {
            base_args.push("--frozen".to_owned());
        }
        if options.flag_locked {
            base_args.push("--locked".to_owned());
        }
        base_args.push("--no-default-features".to_owned());
        base_args
    };

    let root = find_root_manifest_for_wd(options.flag_manifest_path, config.cwd())?;
    let workspace = Workspace::new(&root, config)?;
    let current = workspace.current()?;
    let features = Vec::from_iter(current.summary().features().keys().map(|s| s as &str).filter(|s| s != &"default"));

    let set_to_process = |set| {
        let mut process = process("cargo");
        process.args(&base_args);
        if set != "" {
            process.arg("--features").arg(set);
        }
        process
    };

    let feature_sets = (1..features.len()).flat_map(|n| features.iter().combinations(n).map(|combination| combination.iter().join(" ")));

    let mut failed = false;
    for process in feature_sets.map(|set| set_to_process(set)) {
        config.shell().status("Running", process.to_string())?;
        match process.exec() {
            Ok(()) => (),
            Err(err) => {
                config.shell().error(err)?;
                failed = true;
            }
        }
    }

    if failed {
        Err(CliError::new(human("at least one subcommand failed"), 7))
    } else {
        Ok(None)
    }
}

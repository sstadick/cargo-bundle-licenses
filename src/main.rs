use std::{
    env,
    fs::File,
    io::{self, BufReader, BufWriter, Write},
    path::PathBuf,
    process::exit,
};

use anyhow::{Error, Result};
use bundle_licenses_lib::{bundle::BundleBuilder, format::Format};
use env_logger::Env;
use structopt::{clap::AppSettings::ColoredHelp, StructOpt};
use strum::VariantNames;

use git_version::git_version;

pub const CARGO_BUNDLE_LICENSES_VERSION: &str = git_version!(
    cargo_prefix = "cargo:",
    prefix = "git:",
    // Note that on the CLI, the cargo-bundle-licenses* needs to be in single quotes
    // When passed here though there seems to be some magic quoting that happens.
    args = [
        "--always",
        "--dirty=-modified",
        "--match=cargo-bundle-licenses*"
    ]
);

/// Get a buffered output writer from stdout or a file
fn get_output(path: Option<PathBuf>) -> Result<Box<dyn Write + Send + 'static>> {
    let writer: Box<dyn Write + Send + 'static> = match path {
        Some(path) => {
            if path.as_os_str() == "-" {
                Box::new(BufWriter::new(io::stdout()))
            } else {
                Box::new(BufWriter::new(File::create(path)?))
            }
        }
        None => Box::new(BufWriter::new(io::stdout())),
    };
    Ok(writer)
}

/// Check if err is a broken pipe.
#[inline]
fn is_broken_pipe(err: &Error) -> bool {
    if let Some(io_err) = err.root_cause().downcast_ref::<io::Error>() {
        if io_err.kind() == io::ErrorKind::BrokenPipe {
            return true;
        }
    }
    false
}

#[derive(StructOpt, Debug)]
#[structopt(bin_name = "cargo bundle-licenses", author, global_setting(ColoredHelp), version = CARGO_BUNDLE_LICENSES_VERSION)]
pub struct Opts {
    /// The format to write the output in
    #[structopt(long, short, default_value = "toml", possible_values = Format::VARIANTS)]
    format: Format,

    /// The file to write the output to. None or "-" for STDOUT
    #[structopt(long, short)]
    output: Option<PathBuf>,

    /// A previous thirdparty file to use to check for differences / pull updates
    #[structopt(long, short)]
    previous: Option<PathBuf>,

    /// After filling in not-found licenses, check if new is a strict subset of previous.
    #[structopt(long, short)]
    check_previous: bool,
}

/// Parse args and set up logging / tracing
fn setup() -> Opts {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    // Remove the extra arg from cargo
    let args = env::args().filter(|x| x != "bundle-licenses");
    Opts::from_iter(args)
}

fn main() -> Result<()> {
    let opts = setup();
    let previous = if let Some(path) = opts.previous {
        let reader = BufReader::new(File::open(path)?);
        Some(opts.format.deserialize_from_reader(reader)?)
    } else {
        None
    };

    let bundle = BundleBuilder::exec_with_previous(previous.as_ref())?;

    let output = get_output(opts.output)?;

    if let Err(err) = opts
        .format
        .serialize_to_writer(output, &bundle)
        .map_err(Error::from)
    {
        if is_broken_pipe(&err) {
            exit(0);
        }
        return Err(err);
    }

    if previous.is_some()
        && opts.check_previous
        && !previous.as_ref().unwrap().check_subset(&bundle)
    {
        log::error!("Previous bundle does not match latest bundle.");
        exit(1);
    }
    Ok(())
}

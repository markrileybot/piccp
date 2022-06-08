use std::env;
use std::path::PathBuf;

use clap::Parser;

/// pic copy.  Copy files using pictures!
///
#[derive(Parser, Debug)]
#[clap(about, version=env!("VERSION_STRING"), author)]
pub struct Args {
    /// Read data from stdin
    #[clap(short='-', long, env="PICCP_READ_FROM_STDIN")]
    pub read_from_stdin: bool,

    // /// The max depth
    // #[clap(short='d', long, env="SS_MAX_DEPTH", default_value_t = 3)]
    // pub max_depth: usize,
    //
    // /// The max workers
    // #[clap(short='w', long, env="SS_MAX_WORKERS", default_value_t = 16)]
    // pub max_workers: u8,
    //
    // /// The extra rsync args
    // #[clap(short='x', long, env="SS_RSYNC_ARGS", default_value = "-aHAXxESW --no-compress --info=STATS")]
    // pub rsync_args: String,
    //
    // /// The extra cp args
    // #[clap(short='X', long, env="SS_CP_ARGS", default_value = "-ar")]
    // pub cp_args: String,
    //
    // /// Don't actually do anything, just print the commands
    // #[clap(short='D', long, env="SS_DRY_RUN")]
    // pub dry_run: bool,
    //
    // /// The src directory
    // pub src_dir: String,
    //
    // /// The dest directory
    // pub dest_dir: String
}
use clap::Parser;

/// pic copy.  Copy files using pictures!
///
#[derive(Parser, Debug)]
#[clap(about, version=env!("VERSION_STRING"), author)]
pub struct Args {
    /// Send data from stdin
    #[clap(short='s', long)]
    pub send: bool,

    /// The maximum size for each fragment
    #[clap(short='f', long, env="PICCP_FRAGMENT_SIZE", default_value_t = 512)]
    pub fragment_size: u16,

    /// Receive data and write to stdout
    #[clap(short='r', long)]
    pub receive: bool,
}
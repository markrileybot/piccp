use clap::Parser;

/// pic copy.  Copy files using pictures!
///
#[derive(Parser, Debug)]
#[clap(about, version=env!("VERSION_STRING"), author)]
pub struct Args {
    /// Send data from stdin
    #[clap(short='s', long)]
    pub send: bool,

    /// Send data from this file
    #[clap(short='i', long, default_value = "")]
    pub input_file: String,

    /// The maximum size for each fragment
    #[clap(short='f', long, env="PICCP_FRAGMENT_SIZE", default_value_t = 128)]
    pub fragment_size: u16,

    /// The width of a block
    #[clap(short='W', long, env="PICCP_BLOCK_WIDTH", default_value_t = 4)]
    pub scale_width: u8,

    /// The height of a block
    #[clap(short='H', long, env="PICCP_BLOCK_HEIGHT", default_value_t = 2)]
    pub scale_height: u8,

    /// Hide quiet zone?
    #[clap(short='Q', long, env="PICCP_HIDE_QUIET_ZONE")]
    pub hide_quiet_zone: bool,

    /// Receive data and write to stdout
    #[clap(short='r', long)]
    pub receive: bool,

    /// Receive data and write to this file
    #[clap(short='o', long, default_value = "")]
    pub output_file: String,
}

impl Args {
    pub fn is_sender(&self) -> bool {
        self.send || !self.input_file.is_empty()
    }
}
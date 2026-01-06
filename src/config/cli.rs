use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "redis-nav")]
#[command(about = "Terminal UI for browsing and editing Redis databases")]
pub struct Cli {
    /// Redis URL (redis://host:port) or profile name
    #[arg(value_name = "CONNECTION")]
    pub connection: Option<String>,

    /// Redis host
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    pub host: String,

    /// Redis port
    #[arg(short, long, default_value = "6379")]
    pub port: u16,

    /// Redis password (or use REDIS_PASSWORD env)
    #[arg(short = 'a', long)]
    pub password: Option<String>,

    /// Database number
    #[arg(short = 'n', long, default_value = "0")]
    pub db: u8,

    /// Key delimiter (can be specified multiple times)
    #[arg(short, long, default_value = ":")]
    pub delimiter: Vec<char>,

    /// Use named profile from config
    #[arg(long)]
    pub profile: Option<String>,

    /// Disable all write operations
    #[arg(long)]
    pub readonly: bool,

    /// Config file path
    #[arg(long)]
    pub config: Option<std::path::PathBuf>,
}

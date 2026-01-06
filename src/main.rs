use anyhow::Result;
use clap::Parser;
use redis_nav::config::cli::Cli;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    println!("redis-nav starting with: {:?}", cli);
    Ok(())
}

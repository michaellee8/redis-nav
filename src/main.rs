use anyhow::Result;
use clap::Parser;
use redis_nav::app::App;
use redis_nav::config::cli::Cli;
use redis_nav::config::file::ConfigFile;
use redis_nav::config::{AppConfig, ConnectionConfig, UiConfig};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load config file if it exists
    let config_path = cli.config.clone().unwrap_or_else(|| {
        dirs::config_dir()
            .unwrap_or_default()
            .join("redis-nav")
            .join("config.toml")
    });

    let file_config = if config_path.exists() {
        ConfigFile::load(&config_path).ok()
    } else {
        None
    };

    // Build connection URL
    let url = if let Some(ref conn) = cli.connection {
        if conn.starts_with("redis://") || conn.starts_with("rediss://") {
            conn.clone()
        } else if let Some(ref fc) = file_config {
            // Try to use as profile name
            if let Some(profile) = fc.profiles.get(conn) {
                build_url_from_profile(profile, &cli)?
            } else {
                conn.clone()
            }
        } else {
            conn.clone()
        }
    } else if let Some(ref profile_name) = cli.profile {
        if let Some(ref fc) = file_config {
            if let Some(profile) = fc.profiles.get(profile_name) {
                build_url_from_profile(profile, &cli)?
            } else {
                anyhow::bail!("Profile '{}' not found in config", profile_name);
            }
        } else {
            anyhow::bail!("No config file found");
        }
    } else {
        // Build from CLI args
        let password = cli
            .password
            .clone()
            .or_else(|| std::env::var("REDIS_PASSWORD").ok());

        if let Some(pass) = password {
            format!("redis://:{}@{}:{}", pass, cli.host, cli.port)
        } else {
            format!("redis://{}:{}", cli.host, cli.port)
        }
    };

    // Build delimiters
    let delimiters = if !cli.delimiter.is_empty() {
        cli.delimiter.clone()
    } else if let Some(ref fc) = file_config {
        fc.defaults
            .delimiters
            .iter()
            .filter_map(|s| s.chars().next())
            .collect()
    } else {
        vec![':']
    };

    // Build protected namespaces
    let protected_namespaces = if let Some(ref fc) = file_config {
        if let Some(ref profile_name) = cli.profile {
            fc.profiles
                .get(profile_name)
                .map(|p| p.protected_namespaces.clone())
                .unwrap_or_default()
        } else {
            vec![]
        }
    } else {
        vec![]
    };

    let config = AppConfig {
        connection: ConnectionConfig {
            url,
            db: cli.db,
            readonly: cli.readonly,
        },
        ui: UiConfig {
            delimiters,
            protected_namespaces,
        },
    };

    // Initialize terminal
    let mut terminal = ratatui::init();
    terminal.clear()?;

    // Run app
    let mut app = App::new(config).await?;
    let result = app.run(&mut terminal).await;

    // Restore terminal
    ratatui::restore();

    result
}

fn build_url_from_profile(
    profile: &redis_nav::config::file::Profile,
    cli: &Cli,
) -> Result<String> {
    if let Some(ref url) = profile.url {
        return Ok(url.clone());
    }

    let host = profile.host.as_deref().unwrap_or(&cli.host);
    let port = profile.port.unwrap_or(cli.port);

    let password = profile
        .password
        .clone()
        .or_else(|| {
            profile
                .password_env
                .as_ref()
                .and_then(|env| std::env::var(env).ok())
        })
        .or_else(|| cli.password.clone())
        .or_else(|| std::env::var("REDIS_PASSWORD").ok());

    if let Some(pass) = password {
        Ok(format!("redis://:{}@{}:{}", pass, host, port))
    } else {
        Ok(format!("redis://{}:{}", host, port))
    }
}

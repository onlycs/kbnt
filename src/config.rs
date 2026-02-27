use directories::ProjectDirs;
use serde::Deserialize;
use snafu::prelude::*;
use tokio::io;

#[derive(Debug, Deserialize)]
pub(crate) struct KBNTConfig {
    pub(crate) team_number: u16,
    pub(crate) capture_chars: String,
}

#[derive(Debug, Snafu)]
pub(crate) enum ConfigError {
    #[snafu(display("At {location}: Failed to get config directory"))]
    DirFind {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to create config directory\n{source}"))]
    DirCreate {
        source: io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to read config file\n{source}"))]
    FileRead {
        source: io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to create config file\n{source}"))]
    FileCreate {
        source: io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to parse config file\n{source}"))]
    Parse {
        source: ron::error::SpannedError,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

const CONFIG_EXAMPLE: &str = include_str!("../config.example.ron");

pub(crate) async fn parse() -> Result<KBNTConfig, ConfigError> {
    // read from windows appdata
    let config_dir = match ProjectDirs::from("org", "team2791", "kbnt") {
        Some(dirs) => dirs.config_dir().to_path_buf(),
        None => return Err(DirFindSnafu.build()),
    };

    if !config_dir.exists() {
        tokio::fs::create_dir_all(&config_dir)
            .await
            .context(DirCreateSnafu)?;
    }

    let config_path = config_dir.join("config.ron");

    if !config_path.exists() {
        tokio::fs::write(&config_path, CONFIG_EXAMPLE)
            .await
            .context(FileCreateSnafu)?;
    }

    let config_str = tokio::fs::read_to_string(&config_path)
        .await
        .context(FileReadSnafu)?;

    let config = ron::from_str(&config_str).context(ParseSnafu)?;

    Ok(config)
}

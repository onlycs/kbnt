use std::{fs, io, path::PathBuf};

use directories::ProjectDirs;
use serde::Deserialize;
use snafu::prelude::*;
use winreg::enums::HKEY_CURRENT_USER;

pub(crate) const APP_ID: &str = "org.team2791.kbnt";
pub(crate) const DISPLAY_NAME: &str = "KBNT";

#[derive(Debug, Snafu)]
pub(crate) enum InstallError {
    #[snafu(display("At {location}: Failed to find config directory"))]
    ConfigDirFind {
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to create directory\n{source}\nPath: {path:?}"))]
    DirCreate {
        source: io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
        path: PathBuf,
    },

    #[snafu(display("At {location}: Failed to read file\n{source}\nPath: {path:?}"))]
    FileRead {
        source: io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
        path: PathBuf,
    },

    #[snafu(display("At {location}: Failed to create file\n{source}\nPath: {path:?}"))]
    FileCreate {
        source: io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
        path: PathBuf,
    },

    #[snafu(display("At {location}: Failed to parse config file\n{source}"))]
    Parse {
        source: ron::error::SpannedError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to find current executable\n{source}"))]
    CurrentExe {
        source: io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Registry error\n{source}"))]
    Registry {
        source: io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: MS Link error\n{source}"))]
    MsLink {
        source: mslnk::MSLinkError,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to kill old KBNT\n{source}"))]
    ExecKillOld {
        source: io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },
}

pub(crate) fn dir() -> Result<PathBuf, InstallError> {
    match ProjectDirs::from("org", "team2791", "kbnt") {
        Some(dirs) => {
            let dir = dirs.config_dir().to_path_buf();

            if !dir.exists() {
                fs::create_dir_all(&dir).context(DirCreateSnafu { path: &dir })?;
            }

            Ok(dir)
        }
        None => Err(ConfigDirFindSnafu.build()),
    }
}

pub(crate) fn dir_infallible() -> PathBuf {
    match dir() {
        Ok(path) => path,
        Err(_) => PathBuf::from("C:\\Users\\Default"),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct KBNTConfig {
    pub(crate) robot_ip: String,
    pub(crate) capture_chars: String,
}

#[derive(Debug)]
pub(crate) struct KBNTConfigHandle {
    pub(crate) path: PathBuf,
}

impl KBNTConfigHandle {
    fn read(&self) -> Result<KBNTConfig, InstallError> {
        let config_str =
            fs::read_to_string(&self.path).context(FileReadSnafu { path: &self.path })?;

        let config = ron::from_str(&config_str).context(ParseSnafu)?;

        Ok(config)
    }

    pub(crate) fn robot_ip(&self) -> Result<String, InstallError> {
        Ok(self.read()?.robot_ip)
    }

    pub(crate) fn capture_chars(&self) -> Result<String, InstallError> {
        Ok(self.read()?.capture_chars)
    }
}

pub(crate) fn config() -> Result<KBNTConfigHandle, InstallError> {
    let install_dir = dir()?;
    let path = &install_dir.join("config.ron");

    if !path.exists() {
        fs::write(path, include_str!("../config.example.ron")).context(FileCreateSnafu { path })?;
    }

    Ok(KBNTConfigHandle {
        path: path.to_path_buf(),
    })
}

fn move_exe() -> Result<(), InstallError> {
    let install_dir = dir()?;
    let target_path = install_dir.join("kbnt.exe");
    let current_exe = std::env::current_exe().context(CurrentExeSnafu)?;

    if target_path.exists() {
        if current_exe == target_path {
            return Ok(());
        }

        // kill existing instance
        std::process::Command::new("taskkill")
            .args(["/F", "/IM", "kbnt.exe"])
            .status()
            .context(ExecKillOldSnafu)?;
    }

    fs::copy(&current_exe, &target_path).context(FileCreateSnafu { path: &target_path })?;
    Ok(())
}

fn add_startup() -> Result<(), InstallError> {
    let startup_dir = dirs::config_dir()
        .map(|d| d.join("Microsoft\\Windows\\Start Menu\\Programs\\Startup"))
        .ok_or_else(|| ConfigDirFindSnafu.build())?;

    let target_path = startup_dir.join("kbnt.lnk");
    if target_path.exists() {
        return Ok(());
    }

    let install_exe = dir()?.join("kbnt.exe");
    mslnk::ShellLink::new(&install_exe)
        .context(MsLinkSnafu)?
        .create_lnk(&target_path)
        .context(MsLinkSnafu)?;

    Ok(())
}

fn register_appid() -> Result<(), InstallError> {
    let hkcu = winreg::RegKey::predef(HKEY_CURRENT_USER);
    let appid_key = &format!("Software\\Classes\\AppUserModelId\\{APP_ID}");

    if hkcu.open_subkey(appid_key).is_ok() {
        return Ok(());
    }

    let (key, _) = hkcu.create_subkey(appid_key).context(RegistrySnafu)?;

    key.set_value("DisplayName", &DISPLAY_NAME)
        .context(RegistrySnafu)?;

    Ok(())
}

pub(crate) fn install() -> Result<(), InstallError> {
    move_exe()?;
    add_startup()?;
    register_appid()?;

    Ok(())
}

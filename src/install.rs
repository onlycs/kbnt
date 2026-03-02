use std::{
    env, fs, io,
    os::windows::ffi::OsStrExt,
    path::PathBuf,
    process::{self, Command},
};

use directories::ProjectDirs;
use serde::Deserialize;
use snafu::prelude::*;
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
        UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOW},
    },
    core::{PCWSTR, w},
};
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

    #[snafu(display("At {location}: Failed to kill old KBNT\n{source}"))]
    ExecKillOld {
        source: io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to schedule startup task\n{source}"))]
    TaskSchedule {
        source: io::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to elevate process\n{source}"))]
    Elevate {
        source: windows::core::Error,
        #[snafu(implicit)]
        location: snafu::Location,
    },

    #[snafu(display("At {location}: Failed to elevate process"))]
    ElevateNoSource {
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
    let current_exe = env::current_exe().context(CurrentExeSnafu)?;

    if target_path.exists() {
        if current_exe == target_path {
            return Ok(());
        }

        // kill existing instance
        Command::new("taskkill")
            .args(["/F", "/IM", "kbnt.exe"])
            .status()
            .context(ExecKillOldSnafu)?;
    }

    fs::copy(&current_exe, &target_path).context(FileCreateSnafu { path: &target_path })?;

    Command::new(&target_path)
        .args(env::args().skip(1))
        .spawn()
        .context(ExecKillOldSnafu)?;

    process::exit(0);
}

fn add_startup() -> Result<(), InstallError> {
    let exe = dir()?.join("kbnt.exe");

    #[rustfmt::skip]
    Command::new("schtasks")
        .args([
            "/Create",
            "/TN", "kbnt",
            "/TR", exe.to_string_lossy().as_str(),
            "/SC", "ONLOGON",
            "/RL", "HIGHEST",
            "/F",
        ])
        .status()
        .context(TaskScheduleSnafu)?;

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

pub(crate) fn elevate() -> Result<(), InstallError> {
    // check current permissions
    unsafe {
        let mut token = HANDLE::default();

        OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).context(ElevateSnafu)?;

        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0;

        GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        )
        .context(ElevateSnafu)?;

        CloseHandle(token).context(ElevateSnafu)?;

        if elevation.TokenIsElevated != 0 {
            return Ok(());
        }
    }

    let current_exe = env::current_exe().context(CurrentExeSnafu)?;

    let exe_wide: Vec<u16> = current_exe
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();

    let res = unsafe {
        ShellExecuteW(
            None,
            w!("runas"),
            PCWSTR(exe_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOW,
        )
    };

    if res.0 as isize <= 32 {
        return Err(ElevateNoSourceSnafu.build());
    }

    process::exit(0);
}

pub(crate) fn install() -> Result<(), InstallError> {
    elevate()?;
    move_exe()?;
    add_startup()?;
    register_appid()?;

    Ok(())
}

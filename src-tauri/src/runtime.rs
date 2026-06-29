use serde::Serialize;
use std::{
    fmt,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePathInput {
    executable_path: PathBuf,
    platform: String,
    appimage_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeInfo {
    schema_version: u32,
    version: String,
    executable_path: String,
    package_context: PackageContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageContext {
    platform: String,
    kind: PackageContextKind,
    package_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageContextKind {
    MacosAppBundle,
    LinuxAppimage,
    WindowsExecutable,
    StandaloneExecutable,
}

#[derive(Debug)]
pub enum RuntimeInfoError {
    CurrentExecutable(std::io::Error),
}

impl RuntimePathInput {
    pub fn current() -> Result<Self, RuntimeInfoError> {
        Ok(Self {
            executable_path: std::env::current_exe()
                .map_err(RuntimeInfoError::CurrentExecutable)?,
            platform: std::env::consts::OS.to_string(),
            appimage_path: std::env::var_os("APPIMAGE").map(PathBuf::from),
        })
    }

    pub fn new(
        executable_path: impl Into<PathBuf>,
        platform: impl Into<String>,
        appimage_path: Option<PathBuf>,
    ) -> Self {
        Self {
            executable_path: executable_path.into(),
            platform: platform.into(),
            appimage_path,
        }
    }
}

impl RuntimeInfo {
    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn executable_path(&self) -> &str {
        &self.executable_path
    }

    pub fn package_context(&self) -> &PackageContext {
        &self.package_context
    }
}

impl PackageContext {
    pub fn platform(&self) -> &str {
        &self.platform
    }

    pub fn kind(&self) -> PackageContextKind {
        self.kind
    }

    pub fn package_path(&self) -> Option<&str> {
        self.package_path.as_deref()
    }
}

impl fmt::Display for RuntimeInfoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentExecutable(error) => {
                write!(
                    formatter,
                    "failed to resolve current executable path: {error}"
                )
            }
        }
    }
}

impl std::error::Error for RuntimeInfoError {}

pub fn current_runtime_info(version: &str) -> Result<RuntimeInfo, RuntimeInfoError> {
    Ok(runtime_info_for_input(
        version,
        RuntimePathInput::current()?,
    ))
}

pub fn runtime_info_for_input(version: &str, input: RuntimePathInput) -> RuntimeInfo {
    let (kind, bundle_path) = package_context_for(&input);
    RuntimeInfo {
        schema_version: 1,
        version: version.to_string(),
        executable_path: input.executable_path.display().to_string(),
        package_context: PackageContext {
            platform: input.platform,
            kind,
            package_path: bundle_path.map(|path| path.display().to_string()),
        },
    }
}

fn package_context_for(input: &RuntimePathInput) -> (PackageContextKind, Option<PathBuf>) {
    if let Some(appimage_path) = &input.appimage_path {
        return (
            PackageContextKind::LinuxAppimage,
            Some(appimage_path.clone()),
        );
    }

    if let Some(bundle_path) = macos_app_bundle_path(&input.executable_path) {
        return (PackageContextKind::MacosAppBundle, Some(bundle_path));
    }

    if is_windows_executable_path(&input.executable_path) {
        return (
            PackageContextKind::WindowsExecutable,
            input.executable_path.parent().map(Path::to_path_buf),
        );
    }

    (PackageContextKind::StandaloneExecutable, None)
}

fn macos_app_bundle_path(executable_path: &Path) -> Option<PathBuf> {
    let macos_dir = executable_path.parent()?;
    if macos_dir.file_name()? != "MacOS" {
        return None;
    }

    let contents_dir = macos_dir.parent()?;
    if contents_dir.file_name()? != "Contents" {
        return None;
    }

    let app_dir = contents_dir.parent()?;
    if app_dir.extension()? != "app" {
        return None;
    }

    Some(app_dir.to_path_buf())
}

fn is_windows_executable_path(executable_path: &Path) -> bool {
    executable_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_macos_app_bundle_layout() {
        let info = runtime_info_for_input(
            "1.2.3",
            RuntimePathInput::new(
                "/Applications/SpielGantt.app/Contents/MacOS/spielgantt",
                "macos",
                None,
            ),
        );

        assert_eq!(info.version(), "1.2.3");
        assert_eq!(
            info.executable_path(),
            "/Applications/SpielGantt.app/Contents/MacOS/spielgantt"
        );
        assert_eq!(info.package_context().platform(), "macos");
        assert_eq!(
            info.package_context().kind(),
            PackageContextKind::MacosAppBundle
        );
        assert_eq!(
            info.package_context().package_path(),
            Some("/Applications/SpielGantt.app")
        );
    }

    #[test]
    fn detects_linux_appimage_package_from_runtime_environment() {
        let info = runtime_info_for_input(
            "1.2.3",
            RuntimePathInput::new(
                "/tmp/.mount_SpielGantt/AppRun",
                "linux",
                Some(PathBuf::from("/home/researcher/SpielGantt.AppImage")),
            ),
        );

        assert_eq!(
            info.package_context().kind(),
            PackageContextKind::LinuxAppimage
        );
        assert_eq!(
            info.package_context().package_path(),
            Some("/home/researcher/SpielGantt.AppImage")
        );
    }

    #[test]
    fn detects_windows_executable_package_directory() {
        let info = runtime_info_for_input(
            "1.2.3",
            RuntimePathInput::new(
                "C:/Users/researcher/AppData/Local/SpielGantt/spielgantt.exe",
                "windows",
                None,
            ),
        );

        assert_eq!(
            info.package_context().kind(),
            PackageContextKind::WindowsExecutable
        );
        assert_eq!(
            info.package_context().package_path(),
            Some("C:/Users/researcher/AppData/Local/SpielGantt")
        );
    }
}

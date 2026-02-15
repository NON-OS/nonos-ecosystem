use nonos_types::{NonosError, NonosResult};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

const ANON_VERSION: &str = "v0.4.9.11";
const GITHUB_RELEASE_BASE: &str =
    "https://github.com/anyone-protocol/ator-protocol/releases/download";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    LinuxAmd64,
    LinuxArm64,
    MacosAmd64,
    MacosArm64,
    WindowsAmd64,
}

impl Platform {
    pub fn detect() -> NonosResult<Self> {
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        return Ok(Platform::LinuxAmd64);

        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        return Ok(Platform::LinuxArm64);

        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        return Ok(Platform::MacosAmd64);

        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        return Ok(Platform::MacosArm64);

        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        return Ok(Platform::WindowsAmd64);

        #[cfg(not(any(
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "windows", target_arch = "x86_64"),
        )))]
        Err(NonosError::Config("Unsupported platform".into()))
    }

    pub fn archive_name(&self) -> &'static str {
        match self {
            Platform::LinuxAmd64 => "anon-live-linux-amd64.zip",
            Platform::LinuxArm64 => "anon-live-linux-arm64.zip",
            Platform::MacosAmd64 => "anon-live-macos-amd64.zip",
            Platform::MacosArm64 => "anon-live-macos-arm64.zip",
            Platform::WindowsAmd64 => "anon-live-windows-signed-amd64.zip",
        }
    }

    pub fn binary_name(&self) -> &'static str {
        match self {
            Platform::WindowsAmd64 => "anon.exe",
            _ => "anon",
        }
    }

    pub fn download_url(&self, version: &str) -> String {
        format!("{}/{}/{}", GITHUB_RELEASE_BASE, version, self.archive_name())
    }
}

pub struct AnonInstaller {
    install_dir: PathBuf,
    platform: Platform,
}

impl AnonInstaller {
    pub fn new(install_dir: PathBuf) -> NonosResult<Self> {
        let platform = Platform::detect()?;
        Ok(Self {
            install_dir,
            platform,
        })
    }

    pub fn with_platform(install_dir: PathBuf, platform: Platform) -> Self {
        Self {
            install_dir,
            platform,
        }
    }

    pub fn binary_path(&self) -> PathBuf {
        self.install_dir.join(self.platform.binary_name())
    }

    pub fn is_installed(&self) -> bool {
        self.binary_path().exists()
    }

    pub async fn install(&self) -> NonosResult<PathBuf> {
        self.install_version(ANON_VERSION).await
    }

    pub async fn install_version(&self, version: &str) -> NonosResult<PathBuf> {
        if !self.install_dir.exists() {
            tokio::fs::create_dir_all(&self.install_dir)
                .await
                .map_err(|e| NonosError::Config(format!("Failed to create install dir: {}", e)))?;
        }

        let url = self.platform.download_url(version);
        info!("Downloading anon binary from {}", url);

        let archive_data = download_file(&url).await?;
        info!("Downloaded {} bytes", archive_data.len());

        let binary_data = extract_binary_from_zip(&archive_data, self.platform.binary_name())?;
        info!("Extracted binary: {} bytes", binary_data.len());

        let binary_path = self.binary_path();
        let mut file = tokio::fs::File::create(&binary_path)
            .await
            .map_err(|e| NonosError::Config(format!("Failed to create binary file: {}", e)))?;

        file.write_all(&binary_data)
            .await
            .map_err(|e| NonosError::Config(format!("Failed to write binary: {}", e)))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = tokio::fs::metadata(&binary_path)
                .await
                .map_err(|e| NonosError::Config(format!("Failed to get permissions: {}", e)))?
                .permissions();
            perms.set_mode(0o755);
            tokio::fs::set_permissions(&binary_path, perms)
                .await
                .map_err(|e| NonosError::Config(format!("Failed to set permissions: {}", e)))?;
        }

        info!("Installed anon binary to {:?}", binary_path);
        Ok(binary_path)
    }

    pub async fn ensure_installed(&self) -> NonosResult<PathBuf> {
        if self.is_installed() {
            debug!("anon binary already installed at {:?}", self.binary_path());
            return Ok(self.binary_path());
        }
        self.install().await
    }

    pub async fn get_version(&self) -> NonosResult<String> {
        if !self.is_installed() {
            return Err(NonosError::Config("anon not installed".into()));
        }

        let output = tokio::process::Command::new(self.binary_path())
            .arg("--version")
            .output()
            .await
            .map_err(|e| NonosError::Config(format!("Failed to get version: {}", e)))?;

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }

    pub async fn uninstall(&self) -> NonosResult<()> {
        if self.is_installed() {
            tokio::fs::remove_file(self.binary_path())
                .await
                .map_err(|e| NonosError::Config(format!("Failed to remove binary: {}", e)))?;
            info!("Uninstalled anon binary");
        }
        Ok(())
    }
}

async fn download_file(url: &str) -> NonosResult<Vec<u8>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| NonosError::Network(format!("Failed to create HTTP client: {}", e)))?;

    let response = client
        .get(url)
        .header("User-Agent", "nonos-anyone/1.0")
        .send()
        .await
        .map_err(|e| NonosError::Network(format!("Download failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(NonosError::Network(format!(
            "Download failed with status: {}",
            response.status()
        )));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| NonosError::Network(format!("Failed to read response: {}", e)))?;

    Ok(bytes.to_vec())
}

fn extract_binary_from_zip(archive_data: &[u8], binary_name: &str) -> NonosResult<Vec<u8>> {
    use std::io::{Cursor, Read};

    let cursor = Cursor::new(archive_data);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|e| NonosError::Config(format!("Failed to read zip archive: {}", e)))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| NonosError::Config(format!("Failed to read zip entry: {}", e)))?;

        let name = file.name().to_string();
        if name.ends_with(binary_name) || name == binary_name {
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .map_err(|e| NonosError::Config(format!("Failed to extract binary: {}", e)))?;
            return Ok(data);
        }
    }

    Err(NonosError::Config(format!(
        "Binary '{}' not found in archive",
        binary_name
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::detect();
        assert!(platform.is_ok());
    }

    #[test]
    fn test_download_url() {
        let url = Platform::LinuxAmd64.download_url("v0.4.9.11");
        assert!(url.contains("anon-live-linux-amd64.zip"));
        assert!(url.contains("v0.4.9.11"));
    }

    #[test]
    fn test_binary_name() {
        assert_eq!(Platform::LinuxAmd64.binary_name(), "anon");
        assert_eq!(Platform::WindowsAmd64.binary_name(), "anon.exe");
    }
}

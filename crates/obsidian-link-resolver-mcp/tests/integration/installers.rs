//! Mocked integration coverage for the separate CLI and MCP installers.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

const REPOSITORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
const MOCK_CONTENT: &str = "mock executable contents";
const RELEASE_TAG: &str = "v9.8.7";

struct Platform {
    os: &'static str,
    os_token: &'static str,
    arch: &'static str,
    arch_token: &'static str,
    executable_suffix: &'static str,
}

const BASH_PLATFORMS: [Platform; 4] = [
    Platform {
        os: "Linux",
        os_token: "linux",
        arch: "x86_64",
        arch_token: "x86_64",
        executable_suffix: "",
    },
    Platform {
        os: "Linux",
        os_token: "linux",
        arch: "aarch64",
        arch_token: "aarch64",
        executable_suffix: "",
    },
    Platform {
        os: "Darwin",
        os_token: "darwin",
        arch: "x86_64",
        arch_token: "x86_64",
        executable_suffix: "",
    },
    Platform {
        os: "Darwin",
        os_token: "darwin",
        arch: "arm64",
        arch_token: "aarch64",
        executable_suffix: "",
    },
];

const POWERSHELL_PLATFORMS: [Platform; 5] = [
    Platform {
        os: "Linux",
        os_token: "linux",
        arch: "X64",
        arch_token: "x86_64",
        executable_suffix: "",
    },
    Platform {
        os: "Linux",
        os_token: "linux",
        arch: "Arm64",
        arch_token: "aarch64",
        executable_suffix: "",
    },
    Platform {
        os: "Darwin",
        os_token: "darwin",
        arch: "X64",
        arch_token: "x86_64",
        executable_suffix: "",
    },
    Platform {
        os: "Darwin",
        os_token: "darwin",
        arch: "Arm64",
        arch_token: "aarch64",
        executable_suffix: "",
    },
    Platform {
        os: "Windows",
        os_token: "windows",
        arch: "X64",
        arch_token: "x86_64",
        executable_suffix: ".exe",
    },
];

#[test]
fn bash_installers_download_the_component_asset_for_latest_and_pinned_versions() {
    for platform in &BASH_PLATFORMS {
        for (binary, script_name) in [
            ("obsidian-link-resolver", "install.sh"),
            ("obsidian-link-resolver-mcp", "install-mcp.sh"),
        ] {
            for version in [None, Some("4.5.6")] {
                run_bash_install(platform, binary, script_name, version);
            }
        }
    }
}

#[test]
fn powershell_installers_download_the_component_asset_for_latest_and_pinned_versions() {
    let temp = tempfile::tempdir().expect("temporary directory should be created");
    let harness = temp.path().join("mock-installer.ps1");
    fs::write(&harness, powershell_harness()).expect("PowerShell harness should be written");

    for platform in &POWERSHELL_PLATFORMS {
        for (binary, script_name) in [
            ("obsidian-link-resolver", "install.ps1"),
            ("obsidian-link-resolver-mcp", "install-mcp.ps1"),
        ] {
            for version in [None, Some("4.5.6")] {
                run_powershell_install(
                    &harness,
                    platform,
                    binary,
                    script_name,
                    version,
                    temp.path(),
                );
            }
        }
    }
}

#[test]
fn neither_installer_family_accepts_a_component_selector() {
    let directory = TempDir::new().expect("temporary directory should be created");
    let mock_bin = directory.path().join("bin");
    fs::create_dir(&mock_bin).expect("mock bin directory should be created");
    write_executable(&mock_bin.join("curl"), "#!/bin/sh\nexit 0\n");

    for script in ["install.sh", "install-mcp.sh"] {
        let output = Command::new("bash")
            .arg(repository_file(&format!("scripts/{script}")))
            .args(["--component", "mcp"])
            .env("PATH", prefixed_path(&mock_bin))
            .output()
            .expect("bash should launch the installer");
        assert!(!output.status.success(), "{script} accepted --component");
    }

    for script in ["install.ps1", "install-mcp.ps1"] {
        let source = fs::read_to_string(repository_file(&format!("scripts/{script}")))
            .expect("PowerShell installer should be readable");
        let parameters = source
            .split_once("param(")
            .expect("installer should declare its parameters")
            .1
            .split_once(')')
            .expect("installer parameter list should close")
            .0
            .to_ascii_lowercase();
        assert!(
            !parameters.contains("component"),
            "{script} declares a component-selection parameter"
        );
    }
}

fn run_bash_install(platform: &Platform, binary: &str, script_name: &str, version: Option<&str>) {
    let temp = tempfile::tempdir().expect("temporary directory should be created");
    let mock_bin = temp.path().join("bin");
    let install_dir = temp.path().join("installed");
    let log_path = temp.path().join("curl.log");
    fs::create_dir(&mock_bin).expect("mock bin directory should be created");
    write_executable(
        &mock_bin.join("uname"),
        "#!/bin/sh\ncase \"$1\" in -s) printf '%s\\n' \"$MOCK_OS\" ;; -m) printf '%s\\n' \"$MOCK_ARCH\" ;; *) exit 2 ;; esac\n",
    );
    write_executable(&mock_bin.join("curl"), &mock_curl_script());

    let mut command = Command::new("bash");
    command
        .arg(repository_file(&format!("scripts/{script_name}")))
        .args(["--install-dir"])
        .arg(&install_dir)
        .env("PATH", prefixed_path(&mock_bin))
        .env("HOME", temp.path())
        .env("MOCK_OS", platform.os)
        .env("MOCK_ARCH", platform.arch)
        .env("MOCK_BINARY", binary)
        .env("MOCK_TAG", RELEASE_TAG)
        .env("MOCK_ASSET", asset_name(binary, RELEASE_TAG, platform))
        .env("MOCK_DOWNLOAD_URL", "https://download.test/asset")
        .env("MOCK_CONTENT", MOCK_CONTENT)
        .env("MOCK_CURL_LOG", &log_path);
    if let Some(version) = version {
        command.args(["--version", version]);
    }

    let output = command.output().expect("bash installer should launch");
    assert_success(&output, script_name);
    assert_eq!(
        fs::read_to_string(install_dir.join(binary)).expect("installed binary should exist"),
        MOCK_CONTENT
    );

    let log = fs::read_to_string(log_path).expect("mock curl should record requests");
    let expected_api = match version {
        Some(version) => format!(
            "https://api.github.com/repos/sylvercode/obsidian-link-resolver/releases/tags/v{version}"
        ),
        None => "https://api.github.com/repos/sylvercode/obsidian-link-resolver/releases/latest"
            .to_owned(),
    };
    assert!(log.contains(&expected_api), "missing API request: {log}");
    assert!(log.contains("https://download.test/asset"), "{log}");
}

fn run_powershell_install(
    harness: &Path,
    platform: &Platform,
    binary: &str,
    script_name: &str,
    version: Option<&str>,
    base_dir: &Path,
) {
    let case_name = format!(
        "ps-{binary}-{}-{}-{}",
        platform.os_token,
        platform.arch_token,
        version.unwrap_or("latest")
    );
    let install_dir = base_dir.join(&case_name);
    let installer = base_dir.join(format!("{case_name}.ps1"));
    let mut source = fs::read_to_string(repository_file(&format!("scripts/{script_name}")))
        .expect("PowerShell installer should be readable");
    // Inject only runtime probes so the installer's platform mapping runs unchanged.
    for (probe, replacement) in [
        ("$IsWindows", format!("${}", platform.os == "Windows")),
        ("$IsLinux", format!("${}", platform.os == "Linux")),
        ("$IsMacOS", format!("${}", platform.os == "Darwin")),
        (
            "[System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture",
            format!("'{}'", platform.arch),
        ),
    ] {
        assert!(source.contains(probe), "{script_name} is missing {probe}");
        source = source.replace(probe, &replacement);
    }
    fs::write(&installer, source).expect("platform-injected installer should be written");
    let output = Command::new("pwsh")
        .args(["-NoProfile", "-File"])
        .arg(harness)
        .arg(&installer)
        .arg(&install_dir)
        .arg(version.unwrap_or_default())
        .arg(binary)
        .arg(asset_name(binary, RELEASE_TAG, platform))
        .arg("https://download.test/asset")
        .output()
        .expect("PowerShell installer harness should launch");
    assert_success(&output, script_name);

    let report = output
        .stdout
        .split(|byte| *byte == b'\n')
        .rfind(|line| !line.is_empty())
        .expect("harness should return a JSON report");
    let report: serde_json::Value =
        serde_json::from_slice(report).expect("harness report should be JSON");
    let expected_api = match version {
        Some(version) => format!(
            "https://api.github.com/repos/sylvercode/obsidian-link-resolver/releases/tags/v{version}"
        ),
        None => "https://api.github.com/repos/sylvercode/obsidian-link-resolver/releases/latest"
            .to_owned(),
    };
    assert_eq!(report["api_uri"], expected_api, "mock report: {report}");
    assert_eq!(report["download_uri"], "https://download.test/asset");
    assert_eq!(
        report["asset_name"],
        asset_name(binary, RELEASE_TAG, platform)
    );
    assert_eq!(
        fs::read_to_string(install_dir.join(format!("{binary}{}", platform.executable_suffix)))
            .expect("installed binary should exist"),
        MOCK_CONTENT
    );
}

fn asset_name(binary: &str, tag: &str, platform: &Platform) -> String {
    format!(
        "{binary}-{tag}-{}-{}{}",
        platform.os_token, platform.arch_token, platform.executable_suffix
    )
}

fn powershell_harness() -> &'static str {
    r#"param(
    [string] $Installer,
    [string] $InstallDir,
    [string] $Version,
    [string] $BinaryName,
    [string] $AssetName,
    [string] $DownloadUrl
)
$global:MockApiUri = $null
$global:MockDownloadUri = $null
$global:MockAssetName = $AssetName
$global:MockDownloadUrl = $DownloadUrl
function global:MockInvokeRestMethod {
    param([string] $Uri, [hashtable] $Headers)
    $global:MockApiUri = $Uri
    return [pscustomobject]@{
        tag_name = 'v9.8.7'
        assets = @([pscustomobject]@{ name = $global:MockAssetName; browser_download_url = $global:MockDownloadUrl })
    }
}
function global:MockInvokeWebRequest {
    param([string] $Uri, [string] $OutFile)
    $global:MockDownloadUri = $Uri
    Set-Content -LiteralPath $OutFile -Value 'mock executable contents' -NoNewline
}
Set-Alias -Name Invoke-RestMethod -Value MockInvokeRestMethod -Scope Global -Force
Set-Alias -Name Invoke-WebRequest -Value MockInvokeWebRequest -Scope Global -Force
$InstallerBlock = [scriptblock]::Create((Get-Content -LiteralPath $Installer -Raw))
if ($Version) {
    & $InstallerBlock -InstallDir $InstallDir -Version $Version
} else {
    & $InstallerBlock -InstallDir $InstallDir
}
[pscustomobject]@{
    api_uri = $global:MockApiUri
    download_uri = $global:MockDownloadUri
    asset_name = $AssetName
} | ConvertTo-Json -Compress
"#
}

fn mock_curl_script() -> String {
    r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> "$MOCK_CURL_LOG"
url=""
out=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        -o) out="$2"; shift 2 ;;
        https://*) url="$1"; shift ;;
        *) shift ;;
    esac
done
case "$url" in
    */releases/*)
          printf '{\n  "tag_name": "%s",\n  "assets": [\n    {\n      "name": "%s",\n      "browser_download_url": "%s"\n    }\n  ]\n}\n' \
            "$MOCK_TAG" "$MOCK_ASSET" "$MOCK_DOWNLOAD_URL"
        ;;
    *)
        [ -n "$out" ]
        printf '%s' "$MOCK_CONTENT" > "$out"
        ;;
esac
"#
    .to_owned()
}

fn repository_file(relative_path: &str) -> PathBuf {
    Path::new(REPOSITORY).join(relative_path)
}

fn prefixed_path(prefix: &Path) -> String {
    let mut paths = vec![prefix.to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    std::env::join_paths(paths)
        .expect("mock executable path should join with the existing PATH")
        .to_string_lossy()
        .into_owned()
}

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).expect("mock executable should be written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .expect("mock executable should be executable");
    }
}

fn assert_success(output: &Output, command: &str) {
    assert!(
        output.status.success(),
        "{command} failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#!/usr/bin/env pwsh
param(
	[ValidateSet("install", "update")]
	[string] $Mode = "install",
	[string] $Version,
	[string] $InstallDir
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$Repo = "sylvercode/obsidian-link-resolver"
$BinaryName = "obsidian-link-resolver"

function Fail([string] $Message) {
	Write-Error $Message
	exit 1
}

if ($PSVersionTable.PSVersion.Major -lt 7) {
	Fail "PowerShell 7+ is required."
}

if ($IsWindows) {
	$osToken = "windows"
	$binaryFileName = "$BinaryName.exe"
} elseif ($IsLinux) {
	$osToken = "linux"
	$binaryFileName = $BinaryName
} elseif ($IsMacOS) {
	$osToken = "darwin"
	$binaryFileName = $BinaryName
} else {
	Fail "Unsupported OS. Supported: Windows, Linux, macOS."
}

$arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
switch ($arch) {
	"X64" { $archToken = "x86_64" }
	"Arm64" { $archToken = "aarch64" }
	default { Fail "Unsupported architecture '$arch'. Supported: x86_64, aarch64." }
}

$tag = $null
$apiUrl = "https://api.github.com/repos/$Repo/releases/latest"
if ($Version) {
	$tag = if ($Version.StartsWith("v")) { $Version } else { "v$Version" }
	$apiUrl = "https://api.github.com/repos/$Repo/releases/tags/$tag"
}

$headers = @{ Accept = "application/vnd.github+json" }
if ($env:GH_TOKEN) {
	$headers.Authorization = "Bearer $($env:GH_TOKEN)"
}

try {
	$release = Invoke-RestMethod -Uri $apiUrl -Headers $headers
} catch {
	Fail "Failed to fetch release metadata from GitHub API: $($_.Exception.Message)"
}

if (-not $release.tag_name) {
	Fail "Release metadata is missing tag_name."
}

$tagName = [string] $release.tag_name
$assetName = "$BinaryName-$tagName-$osToken-$archToken"
$asset = $release.assets | Where-Object { $_.name -eq $assetName } | Select-Object -First 1
if (-not $asset) {
	Fail "Could not find asset '$assetName' in release '$tagName'."
}

$existingPath = $null
try {
	$existingPath = (Get-Command $binaryFileName -ErrorAction Stop).Source
} catch {
	$existingPath = $null
}

if (-not $InstallDir) {
	if ($Mode -eq "update") {
		if (-not $existingPath) {
			Fail "Cannot update: '$binaryFileName' is not installed in PATH."
		}
		$targetPath = $existingPath
	} else {
		if ($IsWindows) {
			$InstallDir = Join-Path $HOME ".local\bin"
		} else {
			$InstallDir = Join-Path $HOME ".local/bin"
		}
		$targetPath = Join-Path $InstallDir $binaryFileName
	}
} else {
	$targetPath = Join-Path $InstallDir $binaryFileName
}

$targetDir = Split-Path -Parent $targetPath
if (-not (Test-Path -LiteralPath $targetDir)) {
	New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
}

$tmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmpDir | Out-Null
$tmpFile = Join-Path $tmpDir $binaryFileName

try {
	Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $tmpFile
	Copy-Item -LiteralPath $tmpFile -Destination $targetPath -Force
	if (-not $IsWindows) {
		& chmod +x $targetPath
	}
} catch {
	Fail "Failed to install '$assetName' to '$targetPath': $($_.Exception.Message)"
} finally {
	if (Test-Path -LiteralPath $tmpDir) {
		Remove-Item -LiteralPath $tmpDir -Recurse -Force
	}
}

Write-Host "Installed $binaryFileName $tagName to $targetPath"
Write-Host "Tip: ensure '$targetDir' is in your PATH"

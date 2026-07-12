param(
    [switch]$InstallLinux = $false
)

$ErrorActionPreference = "Stop"

$workspaceRoot = Split-Path -Parent $MyInvocation.MyCommand.Path | Split-Path -Parent
$thirdpartyDir = Join-Path $workspaceRoot "thirdparty"
$ffmpegDir = Join-Path $thirdpartyDir "ffmpeg"

if (-not (Test-Path $thirdpartyDir)) {
    New-Item -ItemType Directory -Path $thirdpartyDir | Out-Null
}

if ($IsLinux -or $InstallLinux) {
    Write-Host "Linux detected. Usually ffmpeg is installed via package manager."
    Write-Host "If running in CI, ensure libavcodec-dev, libavformat-dev, etc. are installed."
    Write-Host "No FFMPEG_DIR needs to be set for pkg-config on Linux."
    exit 0
}

# Windows FFmpeg download (BtbN)
$url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl-shared.zip"
$zipPath = Join-Path $thirdpartyDir "ffmpeg.zip"

if (-not (Test-Path $ffmpegDir)) {
    Write-Host "Downloading FFmpeg from $url ..."
    Invoke-WebRequest -Uri $url -OutFile $zipPath
    Write-Host "Extracting FFmpeg..."
    Expand-Archive -Path $zipPath -DestinationPath $thirdpartyDir -Force
    Remove-Item $zipPath

    # The extracted folder is named something like ffmpeg-master-latest-win64-gpl-shared
    $extractedFolder = Get-ChildItem -Path $thirdpartyDir -Directory | Where-Object { $_.Name -like "ffmpeg-*" } | Select-Object -First 1
    Rename-Item -Path $extractedFolder.FullName -NewName "ffmpeg"
    
    Write-Host "FFmpeg extracted to $ffmpegDir"
} else {
    Write-Host "FFmpeg already exists at $ffmpegDir"
}

# Copy DLLs to target/debug and target/release so the binary can find them at runtime
$binDir = Join-Path $ffmpegDir "bin"
$dlls = Get-ChildItem -Path $binDir -Filter "*.dll"

foreach ($target in @("debug", "release")) {
    $targetDir = Join-Path (Join-Path $workspaceRoot "target") $target
    if (-not (Test-Path $targetDir)) {
        New-Item -ItemType Directory -Path $targetDir -Force | Out-Null
    }
    foreach ($dll in $dlls) {
        Copy-Item -Path $dll.FullName -Destination $targetDir -Force
    }
}

Write-Host "DLLs copied to target/debug and target/release."

$envValue = $ffmpegDir
Write-Host "Please set the FFMPEG_DIR environment variable to: $envValue"

if ($env:GITHUB_ENV) {
    Write-Host "Setting FFMPEG_DIR in GITHUB_ENV..."
    Add-Content -Path $env:GITHUB_ENV -Value "FFMPEG_DIR=$envValue"
}

Write-Host "Done."

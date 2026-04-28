# Hermes CLI installer for Windows (PowerShell)
# Run: irm https://raw.githubusercontent.com/nousresearch/hermes-rust-win/master/install.ps1 | iex

param(
    [string]$InstallDir = "$env:USERPROFILE\.hermes\bin"
)

$ErrorActionPreference = "Stop"
$Repo = "nousresearch/hermes-rust-win"
$Binary = "hermes.exe"

Write-Host "Installing Hermes CLI..." -ForegroundColor Cyan

# Detect architecture
$Arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "x86_64" }
$Target = "$Arch-pc-windows-msvc"

# Get latest release
try {
    $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -ErrorAction Stop
    $Latest = $Release.tag_name
} catch {
    Write-Host "Could not determine latest version. Install via cargo:" -ForegroundColor Yellow
    Write-Host "  cargo install --git https://github.com/$Repo"
    exit 1
}

$Archive = "hermes-$Latest-$Target.zip"
$Url = "https://github.com/$Repo/releases/download/$Latest/$Archive"

Write-Host "Downloading Hermes $Latest for $Target..." -ForegroundColor Cyan

$TmpDir = [System.IO.Path]::GetTempPath() + [System.IO.Path]::GetRandomFileName()
New-Item -ItemType Directory -Path $TmpDir | Out-Null

try {
    $ZipPath = "$TmpDir\hermes.zip"
    Invoke-WebRequest -Uri $Url -OutFile $ZipPath -ErrorAction Stop
    Expand-Archive -Path $ZipPath -DestinationPath $TmpDir -Force

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item "$TmpDir\$Binary" "$InstallDir\$Binary" -Force

    Write-Host ""
    Write-Host "Hermes $Latest installed to $InstallDir\$Binary" -ForegroundColor Green

    $PathEnv = [Environment]::GetEnvironmentVariable("PATH", "User")
    if ($PathEnv -notlike "*$InstallDir*") {
        Write-Host "Adding $InstallDir to your PATH..." -ForegroundColor Yellow
        [Environment]::SetEnvironmentVariable("PATH", "$PathEnv;$InstallDir", "User")
        $env:PATH = "$env:PATH;$InstallDir"
    }

    Write-Host ""
    Write-Host "Run 'hermes --help' to get started." -ForegroundColor Green
} finally {
    Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
}

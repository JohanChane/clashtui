<#
.SYNOPSIS
    ClashTui install script for Windows
.DESCRIPTION
    Downloads and installs clashtui binaries, core binaries (mihomo, sing-box),
    and default configuration files on Windows. Does NOT manage Windows services
    (use ClashTui's CoreSrvCtl for that).
.PARAMETER InstallDir
    Installation directory for binaries (default: D:\ClashTui)
.PARAMETER Core
    Core type to install: mihomo, sing-box, or all (default: all)
.PARAMETER Repo
    GitHub repository for clashtui and contrib (default: JohanChane/clashtui)
.PARAMETER Branch
    Branch for contrib resources (default: main)
.EXAMPLE
    .\install.ps1
    .\install.ps1 -InstallDir "D:\MyTools\ClashTui"
    .\install.ps1 -Core mihomo
#>

param(
    [string]$InstallDir = "D:\ClashTui",
    [ValidateSet("mihomo", "sing-box", "all")]
    [string]$Core = "all",
    [string]$Repo = "JohanChane/clashtui",
    [string]$Branch = "main",
    [switch]$IsTest
)

$ErrorActionPreference = "Stop"

# --- Constants ---
$MIHOMO_UPSTREAM = "MetaCubeX/mihomo"
$SINGBOX_UPSTREAM = "SagerNet/sing-box"

# --- Logging ---
function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-ErrorLog {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

# --- Validation ---
function Test-ValidInstallDir {
    param([string]$Path)

    if ($Path -match '\s') {
        Write-ErrorLog "InstallDir must not contain spaces: $Path"
        exit 1
    }

    # Normalize path for comparison
    $normalized = (Resolve-Path $Path -ErrorAction SilentlyContinue).Path
    if (-not $normalized) { $normalized = $Path }

    $restricted = @(
        [System.Environment]::GetFolderPath('ProgramFiles'),
        [System.Environment]::GetFolderPath('ProgramFilesX86')
    )

    foreach ($r in $restricted) {
        if ($normalized -and $normalized.StartsWith($r, [StringComparison]::OrdinalIgnoreCase)) {
            Write-ErrorLog "InstallDir must not be under a restricted directory: $r"
            Write-ErrorLog "Use a non-system directory like D:\ClashTui"
            exit 1
        }
    }

    # Check write permission
    try {
        $parent = Split-Path $Path -Parent
        if (-not (Test-Path $parent)) {
            New-Item -ItemType Directory -Path $parent -Force -ErrorAction Stop | Out-Null
        }
        $testFile = Join-Path $Path ".clashtui_write_test"
        New-Item -ItemType Directory -Path $Path -Force -ErrorAction Stop | Out-Null
        [System.IO.File]::WriteAllText($testFile, "test")
        Remove-Item $testFile -Force -ErrorAction SilentlyContinue
    } catch {
        Write-ErrorLog "Cannot write to $Path. Run as Administrator or choose a different directory."
        exit 1
    }
}

# --- Detection ---
function Get-Architecture {
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::ProcessArchitecture
    switch ($arch) {
        'X64'  { return "amd64" }
        'Arm64' { return "arm64" }
        default { return "unsupported" }
    }
}

function Get-OS {
    if ([System.Environment]::OSVersion.Platform -eq [System.PlatformID]::Win32NT) {
        return "windows"
    }
    return "unsupported"
}

# --- Resolve paths ---
function Resolve-Paths {
    $script:INSTALL_DIR = $InstallDir
    $script:INSTALL_DIR_MIHOMO = Join-Path $InstallDir "mihomo"
    $script:INSTALL_DIR_SINGBOX = Join-Path $InstallDir "sing-box"
    $script:MIHOMO_CONFIG_DIR = Join-Path $INSTALL_DIR_MIHOMO "config"
    $script:SINGBOX_CONFIG_DIR = Join-Path $INSTALL_DIR_SINGBOX "config"
    $script:INSTALL_BIN = Join-Path $InstallDir "bin"

    $script:CLASHTUI_CONFIG_DIR = Join-Path $env:APPDATA "clashtui"
    $script:MIHOMO_USER_CONFIG_DIR = Join-Path $CLASHTUI_CONFIG_DIR "mihomo"
    $script:SINGBOX_USER_CONFIG_DIR = Join-Path $CLASHTUI_CONFIG_DIR "sing-box"

    if ($IsTest) {
        $script:TestTmpDir = Join-Path $env:TEMP "clashtui-test"
        $script:INSTALL_DIR = Join-Path $TestTmpDir "opt/clashtui"
        $script:INSTALL_DIR_MIHOMO = Join-Path $INSTALL_DIR "mihomo"
        $script:INSTALL_DIR_SINGBOX = Join-Path $INSTALL_DIR "sing-box"
        $script:MIHOMO_CONFIG_DIR = Join-Path $INSTALL_DIR_MIHOMO "config"
        $script:SINGBOX_CONFIG_DIR = Join-Path $INSTALL_DIR_SINGBOX "config"
        $script:INSTALL_BIN = Join-Path $INSTALL_DIR "bin"
        $script:CLASHTUI_CONFIG_DIR = Join-Path $TestTmpDir "config/clashtui"
        $script:MIHOMO_USER_CONFIG_DIR = Join-Path $CLASHTUI_CONFIG_DIR "mihomo"
        $script:SINGBOX_USER_CONFIG_DIR = Join-Path $CLASHTUI_CONFIG_DIR "sing-box"
        Write-Info "Test mode: using temp directory $TestTmpDir"
    }

    $script:SCRIPT_DIR = Split-Path $MyInvocation.ScriptName -Parent
    $contribLocal = Join-Path $SCRIPT_DIR "contrib"
    $contribParent = Join-Path (Split-Path $SCRIPT_DIR -Parent) "contrib"
    if (Test-Path $contribLocal) {
        $script:CONTRIB_SOURCE = "local"
        $script:CONTRIB_DIR = $contribLocal
    } elseif (Test-Path $contribParent) {
        $script:CONTRIB_SOURCE = "local"
        $script:CONTRIB_DIR = $contribParent
    } else {
        $script:CONTRIB_SOURCE = "remote"
        $script:CONTRIB_DIR = $null
    }

    $script:CONTRIB_URL_PREFIX = "https://raw.githubusercontent.com/${Repo}/refs/heads/${Branch}/contrib"
}

# --- Helper: copy contrib (local or remote) ---
function Copy-Contrib {
    param(
        [string]$RelativePath,
        [string]$Destination
    )

    if ($CONTRIB_SOURCE -eq "local") {
        $src = Join-Path $CONTRIB_DIR $RelativePath
        if (Test-Path $src) {
            Copy-Item $src $Destination -Force
            return
        }
        Write-Warn "Local file not found: $src, falling back to remote"
    }

    $remoteUrl = "${CONTRIB_URL_PREFIX}/${RelativePath}"
    try {
        Invoke-WebRequest -Uri $remoteUrl -OutFile $Destination -UseBasicParsing
    } catch {
        Write-ErrorLog "Failed to download: $remoteUrl"
        throw
    }
}

# --- Backup ---
function Backup-File {
    param([string]$Path)
    if (-not (Test-Path $Path)) { return }
    $i = 1
    while (Test-Path "${Path}_$i") { $i++ }
    Move-Item $Path "${Path}_$i" -Force
    Write-Info "Backed up: $Path -> ${Path}_$i"
}

function Get-CommandPath {
    param([string]$Name)
    $cmd = Get-Command $Name -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    return $null
}

# --- Binary download ---
function Get-LatestGithubRelease {
    param([string]$Repo)
    $url = "https://api.github.com/repos/${Repo}/releases/latest"
    $response = Invoke-RestMethod -Uri $url -UseBasicParsing
    return $response
}

function Find-AssetUrl {
    param(
        [object]$Release,
        [string[]]$NamePatterns
    )
    foreach ($pattern in $NamePatterns) {
        foreach ($asset in $Release.assets) {
            if ($asset.name -like $pattern) {
                return $asset.browser_download_url
            }
        }
    }
    return $null
}

function Install-Mihomo {
    $destDir = $INSTALL_DIR_MIHOMO
    $destExe = Join-Path $destDir "mihomo.exe"
    Write-Info "Installing mihomo..."

    # Already installed at destination — skip
    if (Test-Path $destExe) {
        Write-Info "mihomo already exists at $destExe, skipping"
        return
    }

    # Found in PATH — link/copy to destination
    $existing = Get-CommandPath "mihomo.exe"
    if ($existing) {
        Write-Info "Found mihomo in PATH: $existing, linking..."
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        Copy-Item -Path $existing -Destination $destExe -Force
        Write-Info "Linked mihomo to: $destExe"
        return
    }

    # Not found — download
    if ($IsTest) {
        $downloadUrl = "https://github.com/$MIHOMO_UPSTREAM/releases/latest"
        Write-Info "[TEST] Would download mihomo from: $downloadUrl"
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        return
    }

    $arch = Get-Architecture
    $os = Get-OS

    if ($arch -eq "unsupported" -or $os -eq "unsupported") {
        Write-ErrorLog "Unsupported architecture or OS"
        exit 1
    }

    $release = Get-LatestGithubRelease $MIHOMO_UPSTREAM
    $version = $release.tag_name
    $assetUrl = Find-AssetUrl $release @(
        "*windows*amd64*compatible*",
        "*windows*amd64*",
        "*windows*x86_64*"
    )

    if (-not $assetUrl) {
        Write-ErrorLog "Could not find mihomo release asset for Windows amd64"
        exit 1
    }

    Write-Info "Detected: OS=$os, Arch=$arch, Version=$version"
    Write-Info "Downloading: $assetUrl"

    $tempDir = Join-Path $env:TEMP "clashtui_mihomo_$(Get-Random)"
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
    $zipPath = Join-Path $tempDir "mihomo.zip"

    try {
        Invoke-WebRequest -Uri $assetUrl -OutFile $zipPath -UseBasicParsing
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

        $binary = Get-ChildItem -Path $tempDir -Recurse -Name "mihomo.exe" | Select-Object -First 1
        if (-not $binary) {
            Write-ErrorLog "Could not find mihomo.exe in the downloaded archive"
            exit 1
        }
        $binaryPath = Join-Path $tempDir $binary

        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        Copy-Item $binaryPath $destExe -Force
        Write-Info "Successfully installed mihomo to: $destExe"
    } finally {
        Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Install-SingBox {
    $destDir = $INSTALL_DIR_SINGBOX
    $destExe = Join-Path $destDir "sing-box.exe"
    Write-Info "Installing sing-box..."

    # Already installed at destination — skip
    if (Test-Path $destExe) {
        Write-Info "sing-box already exists at $destExe, skipping"
        return
    }

    # Found in PATH — link/copy to destination
    $existing = Get-CommandPath "sing-box.exe"
    if ($existing) {
        Write-Info "Found sing-box in PATH: $existing, linking..."
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        Copy-Item -Path $existing -Destination $destExe -Force
        Write-Info "Linked sing-box to: $destExe"
        return
    }

    # Not found — download
    if ($IsTest) {
        $downloadUrl = "https://github.com/$SINGBOX_UPSTREAM/releases/latest"
        Write-Info "[TEST] Would download sing-box from: $downloadUrl"
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        return
    }

    $arch = Get-Architecture
    $os = Get-OS

    if ($arch -eq "unsupported" -or $os -eq "unsupported") {
        Write-ErrorLog "Unsupported architecture or OS"
        exit 1
    }

    $release = Get-LatestGithubRelease $SINGBOX_UPSTREAM
    $version = $release.tag_name
    $assetUrl = Find-AssetUrl $release @(
        "*windows*amd64*",
        "*windows*x86_64*"
    )

    if (-not $assetUrl) {
        Write-ErrorLog "Could not find sing-box release asset for Windows amd64"
        exit 1
    }

    Write-Info "Detected: OS=$os, Arch=$arch, Version=$version"
    Write-Info "Downloading: $assetUrl"

    $tempDir = Join-Path $env:TEMP "clashtui_singbox_$(Get-Random)"
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
    $zipPath = Join-Path $tempDir "sing-box.zip"

    try {
        Invoke-WebRequest -Uri $assetUrl -OutFile $zipPath -UseBasicParsing
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

        $binary = Get-ChildItem -Path $tempDir -Recurse -Name "sing-box.exe" | Select-Object -First 1
        if (-not $binary) {
            Write-ErrorLog "Could not find sing-box.exe in the downloaded archive"
            exit 1
        }
        $binaryPath = Join-Path $tempDir $binary

        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        Copy-Item $binaryPath $destExe -Force
        Write-Info "Successfully installed sing-box to: $destExe"
    } finally {
        Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Install-ClashTui {
    param([string]$CoreType)
    $destDir = $INSTALL_BIN
    $destExe = Join-Path $destDir "clashtui.exe"
    Write-Info "Installing clashtui..."

    # Already installed at destination — skip
    if (Test-Path $destExe) {
        Write-Info "clashtui already exists at $destExe, skipping"
        New-ClashTuiConfig $CoreType
        return
    }

    # Found in PATH — link/copy to destination
    $existing = Get-CommandPath "clashtui.exe"
    if ($existing) {
        Write-Info "Found clashtui in PATH: $existing, linking..."
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        Copy-Item -Path $existing -Destination $destExe -Force
        Write-Info "Linked clashtui to: $destExe"
        New-ClashTuiConfig $CoreType
        return
    }

    # Not found — download
    if ($IsTest) {
        $downloadUrl = "https://github.com/$Repo/releases/latest"
        Write-Info "[TEST] Would download clashtui from: $downloadUrl"
        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        New-ClashTuiConfig $CoreType
        return
    }

    $arch = Get-Architecture
    $os = Get-OS

    if ($arch -eq "unsupported" -or $os -eq "unsupported") {
        Write-ErrorLog "Unsupported architecture or OS"
        exit 1
    }

    $release = Get-LatestGithubRelease $Repo
    $version = $release.tag_name
    $assetUrl = Find-AssetUrl $release @("*windows*amd64*", "*windows*x86_64*")

    if (-not $assetUrl) {
        Write-ErrorLog "Could not find clashtui release asset for Windows amd64"
        exit 1
    }

    Write-Info "Detected: OS=$os, Arch=$arch, Version=$version"
    Write-Info "Downloading: $assetUrl"

    $tempDir = Join-Path $env:TEMP "clashtui_$(Get-Random)"
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
    $zipPath = Join-Path $tempDir "clashtui.zip"

    try {
        Invoke-WebRequest -Uri $assetUrl -OutFile $zipPath -UseBasicParsing
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

        $binary = Get-ChildItem -Path $tempDir -Recurse -Name "clashtui.exe" | Select-Object -First 1
        if (-not $binary) {
            Write-ErrorLog "Could not find clashtui.exe in the downloaded archive"
            exit 1
        }
        $binaryPath = Join-Path $tempDir $binary

        New-Item -ItemType Directory -Path $destDir -Force | Out-Null
        Copy-Item $binaryPath $destExe -Force
        Write-Info "Successfully installed clashtui to: $destExe"
    } finally {
        Remove-Item $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }

    New-ClashTuiConfig $CoreType
}

# --- Config generation ---
function New-ClashTuiConfig {
    param([string]$CoreType)
    Write-Info "Creating clashtui config..."

    New-Item -ItemType Directory -Path $CLASHTUI_CONFIG_DIR -Force | Out-Null

    # Copy default configs
    Copy-Contrib "default_configs/default_keymap.yaml" (Join-Path $CLASHTUI_CONFIG_DIR "default_keymap.yaml")
    Copy-Contrib "default_configs/default_theme.yaml" (Join-Path $CLASHTUI_CONFIG_DIR "default_theme.yaml")
    Write-Info "Copied default configs to: $CLASHTUI_CONFIG_DIR"

    # Generate config.yaml
    $configPath = Join-Path $CLASHTUI_CONFIG_DIR "config.yaml"
    Backup-File $configPath

    $mihomoBinDir = ($INSTALL_DIR_MIHOMO -replace '\\', '/')
    $singboxBinDir = ($INSTALL_DIR_SINGBOX -replace '\\', '/')
    $mihomoCfgDir = ($MIHOMO_CONFIG_DIR -replace '\\', '/')
    $singboxCfgDir = ($SINGBOX_CONFIG_DIR -replace '\\', '/')

    $configContent = @"
mihomo:
  core:
    config_dir: ${mihomoCfgDir}
    bin_path: ${mihomoBinDir}/mihomo.exe
    config_path: ${mihomoCfgDir}/config.yaml
  core_service:
    service_name: clashtui_mihomo
    is_user: false
singbox:
  core:
    bin_path: ${singboxBinDir}/sing-box.exe
    config_dir: ${singboxCfgDir}
    config_path: ${singboxCfgDir}/config.json
  core_service:
    service_name: clashtui_singbox
    is_user: false
timeout:
extra:
  edit_cmd:
  open_dir_cmd:
"@

    Set-Content -Path $configPath -Value $configContent -Encoding UTF8
    Write-Info "Config written to: $configPath"

    # Create directories and template_proxy_providers.yaml
    $tppContent = @'
# Define proxy-provider subscription URLs here, organized by group.
# In templates use ${PPG.<group>} to reference all providers in a group,
# or ${PPG.<group>.<provider>} for a specific one.
#
# Format:
#   <group-name>:
#     <provider-name>: "<subscription-url>"
#
# Clashtui's built-in templates only use the group level
# (e.g. ${PPG.pvd}, not ${PPG.pvd.pvd0}). Provider names
# (pvd0, pvd1, ...) are freely defined by the user.
#
# Example (following clashtui convention):
#   pvd:
#     pvd0: "https://example.com/sub1.yaml"
#     pvd1: "https://example.com/sub2.yaml"
'@

    if ($CoreType -eq "mihomo" -or $CoreType -eq "all") {
        New-Item -ItemType Directory -Path $MIHOMO_USER_CONFIG_DIR -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $MIHOMO_USER_CONFIG_DIR "profiles") -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $MIHOMO_USER_CONFIG_DIR "templates") -Force | Out-Null

        $tppPath = Join-Path $MIHOMO_USER_CONFIG_DIR "template_proxy_providers.yaml"
        if (-not (Test-Path $tppPath)) {
            Set-Content -Path $tppPath -Value $tppContent -Encoding UTF8
        }
    }

    if ($CoreType -eq "sing-box" -or $CoreType -eq "all") {
        New-Item -ItemType Directory -Path $SINGBOX_USER_CONFIG_DIR -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $SINGBOX_USER_CONFIG_DIR "profiles") -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $SINGBOX_USER_CONFIG_DIR "templates") -Force | Out-Null

        $tppPath = Join-Path $SINGBOX_USER_CONFIG_DIR "template_proxy_providers.yaml"
        if (-not (Test-Path $tppPath)) {
            Set-Content -Path $tppPath -Value $tppContent -Encoding UTF8
        }
    }
}

# --- Core config creation ---
function New-CoreConfigs {
    param([string]$CoreType)
    Write-Info "Creating core config files..."

    if ($CoreType -eq "mihomo" -or $CoreType -eq "all") {
        New-Item -ItemType Directory -Path $MIHOMO_CONFIG_DIR -Force | Out-Null

        $cfgSrc = "default_configs/mihomo/core_override_config.yaml"
        Copy-Contrib $cfgSrc (Join-Path $MIHOMO_CONFIG_DIR "config.yaml")
        Write-Info "Mihomo core config written to: $MIHOMO_CONFIG_DIR/config.yaml"

        New-Item -ItemType Directory -Path $MIHOMO_USER_CONFIG_DIR -Force | Out-Null
        Copy-Contrib $cfgSrc (Join-Path $MIHOMO_USER_CONFIG_DIR "core_override_config.yaml")
        Write-Info "Mihomo core override written to: $MIHOMO_USER_CONFIG_DIR/core_override_config.yaml"
    }

    if ($CoreType -eq "sing-box" -or $CoreType -eq "all") {
        New-Item -ItemType Directory -Path $SINGBOX_CONFIG_DIR -Force | Out-Null

        $cfgSrc = "default_configs/sing-box/core_override_config.json"
        Copy-Contrib $cfgSrc (Join-Path $SINGBOX_CONFIG_DIR "config.json")
        Write-Info "Sing-box core config written to: $SINGBOX_CONFIG_DIR/config.json"

        New-Item -ItemType Directory -Path $SINGBOX_USER_CONFIG_DIR -Force | Out-Null
        Copy-Contrib $cfgSrc (Join-Path $SINGBOX_USER_CONFIG_DIR "core_override_config.json")
        Write-Info "Sing-box core override written to: $SINGBOX_USER_CONFIG_DIR/core_override_config.json"
    }
}

# --- Optional downloads ---
function Invoke-OptionalDownloads {
    if ($IsTest) {
        Write-Info "[TEST] Skipping interactive template/rules-dat download prompts"
        return
    }

    if ($Core -eq "mihomo" -or $Core -eq "all") {
        $response = Read-Host "Do you want to download templates for mihomo? (y/N)"
        if ($response -eq "y" -or $response -eq "Y") {
            Write-Info "Downloading mihomo templates..."
            Copy-Contrib "templates/mihomo/common_tpl.yaml" (Join-Path $MIHOMO_USER_CONFIG_DIR "templates/common_tpl.yaml")
            Copy-Contrib "templates/mihomo/generic_tpl.yaml" (Join-Path $MIHOMO_USER_CONFIG_DIR "templates/generic_tpl.yaml")
            Copy-Contrib "templates/mihomo/generic_tpl_with_all.yaml" (Join-Path $MIHOMO_USER_CONFIG_DIR "templates/generic_tpl_with_all.yaml")
            Copy-Contrib "templates/mihomo/generic_tpl_with_filter.yaml" (Join-Path $MIHOMO_USER_CONFIG_DIR "templates/generic_tpl_with_filter.yaml")
            Copy-Contrib "templates/mihomo/generic_tpl_with_ruleset.yaml" (Join-Path $MIHOMO_USER_CONFIG_DIR "templates/generic_tpl_with_ruleset.yaml")
        }

        $response = Read-Host "Do you want to download rules-dat? (y/N)"
        if ($response -eq "y" -or $response -eq "Y") {
            Write-Info "Downloading rules-dat..."
            Invoke-WebRequest -Uri "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip.metadb" -OutFile (Join-Path $MIHOMO_CONFIG_DIR "geoip.metadb") -UseBasicParsing
            Invoke-WebRequest -Uri "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat" -OutFile (Join-Path $MIHOMO_CONFIG_DIR "GeoSite.dat") -UseBasicParsing
        }
    }

    if ($Core -eq "sing-box" -or $Core -eq "all") {
        $response = Read-Host "Do you want to download templates for sing-box? (y/N)"
        if ($response -eq "y" -or $response -eq "Y") {
            Write-Info "Downloading sing-box templates..."
            Copy-Contrib "templates/sing-box/v1.12-tun_common_tpl.json" (Join-Path $SINGBOX_USER_CONFIG_DIR "templates/v1.12-tun_common_tpl.json")
            Copy-Contrib "templates/sing-box/v1.12-tun_bypass.json" (Join-Path $SINGBOX_USER_CONFIG_DIR "templates/v1.12-tun_bypass.json")
        }
    }
}

# --- Main ---
function Main {
    if ((-not $IsTest) -and (Get-OS) -ne "windows") {
        Write-ErrorLog "This script is for Windows only. Use the bash install script for Linux/macOS."
        exit 1
    }

    if (-not $IsTest) {
        Test-ValidInstallDir $InstallDir
    }
    Resolve-Paths

    Write-Info "Install directory: $InstallDir"
    Write-Info "Core type: $Core"
    Write-Info "Repo: $Repo"
    Write-Info "Branch: $Branch"

    # Create necessary directories
    if ($Core -eq "mihomo" -or $Core -eq "all") {
        New-Item -ItemType Directory -Path $MIHOMO_CONFIG_DIR -Force | Out-Null
    }
    if ($Core -eq "sing-box" -or $Core -eq "all") {
        New-Item -ItemType Directory -Path $SINGBOX_CONFIG_DIR -Force | Out-Null
    }
    New-Item -ItemType Directory -Path $CLASHTUI_CONFIG_DIR -Force | Out-Null

    # Install cores
    switch ($Core) {
        "mihomo"   { Install-Mihomo }
        "sing-box" { Install-SingBox }
        "all"      { Install-Mihomo; Install-SingBox }
    }

    # Install clashtui (binary + config)
    Install-ClashTui $Core

    # Create core configs
    New-CoreConfigs $Core

    # Optional downloads
    Invoke-OptionalDownloads

    Write-Info "Installed cores: $Core"
    Write-Info "Clashtui binary: $INSTALL_BIN/clashtui.exe"
    Write-Info "Clashtui config: $CLASHTUI_CONFIG_DIR"
    Write-Info "Core configs written to core config directories"
    Write-Info "Install directory: $InstallDir"
    Write-Info "Use ClashTui's CoreSrvCtl to manage core services (install/start/stop via nssm)."
}

# Only execute Main when run directly (not dot-sourced)
if ($MyInvocation.InvocationName -ne '.') {
    Main
}

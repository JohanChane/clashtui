# ClashTui Windows Install Script
# ================================
#
# Installs clashtui and core proxies (mihomo / sing-box) on Windows.
# Uses clashtui binary's built-in service subcommand which delegates to
# nssm (Non-Sucking Service Manager) for Windows Service management.
# Requires: nssm (install via `scoop install nssm` or https://nssm.cc/download)
#
# Usage:
#   .\install.ps1                              # Default: system mode to C:\Program Files\clashtui
#   .\install.ps1 -InstallDir "D:\clashtui"    # Custom install directory
#   .\install.ps1 -UserMode                     # Install to %LOCALAPPDATA%\clashtui
#   .\install.ps1 -Core "mihomo"               # Install only mihomo
#   .\install.ps1 -Uninstall                   # Uninstall clashtui
#   .\install.ps1 -Help                        # Show help

param(
    [Parameter(HelpMessage = "Install directory (system mode)")]
    [string]$InstallDir,

    [Parameter(HelpMessage = "Install in user mode (%LOCALAPPDATA%\clashtui)")]
    [switch]$UserMode,

    [Parameter(HelpMessage = "Core type: mihomo, sing-box, or all")]
    [ValidateSet("mihomo", "sing-box", "all")]
    [string]$Core = "all",

    [Parameter(HelpMessage = "GitHub repo for clashtui and contrib (default: JohanChane/clashtui)")]
    [string]$Repo = "JohanChane/clashtui",

    [Parameter(HelpMessage = "Branch for contrib resources (default: main)")]
    [string]$Branch = "main",

    [Parameter(HelpMessage = "Uninstall clashtui")]
    [switch]$Uninstall,

    [Parameter(HelpMessage = "Show help")]
    [switch]$Help
)

# --- Configuration ---
# Parsed from params: $Repo (e.g. "JohanChane/clashtui"), $Branch
$Script:RepoOwner, $Script:RepoName = if ($Repo -match '^(.+)/(.+)$') { $matches[1], $matches[2] } else { "JohanChane", "clashtui" }
$Script:Branch = $Branch
$Script:ContribUrlPrefix = "https://raw.githubusercontent.com/$Script:RepoOwner/$Script:RepoName/refs/heads/$Script:Branch/contrib"
# Capture script-level PSBoundParameters for use in Invoke-Administrator (function scope has its own $PSBoundParameters)
$Script:BoundParameters = $PSBoundParameters

# --- Helper Functions ---

function Invoke-Administrator {
    <#
    .SYNOPSIS
    Relaunch the script with administrator privileges if not already elevated.
    .DESCRIPTION
    Uses Start-Process -Verb RunAs and passes all original parameters via the
    command line, which lets PowerShell's native parameter binding handle them
    correctly in the elevated instance.
    #>
    if (-NOT ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        Write-Host "[INFO] Requesting administrator privileges..." -ForegroundColor Yellow

        # Rebuild the original argument list from script-level bound parameters
        $argList = New-Object System.Collections.Generic.List[string]
        foreach ($key in $Script:BoundParameters.Keys) {
            $val = $Script:BoundParameters[$key]
            if ($val -is [switch]) {
                if ($val) { $argList.Add("-$key") }
            } elseif ($val -is [string] -and $val -match '\s') {
                $argList.Add("-$key")
                $argList.Add("`"$val`"")
            } else {
                $argList.Add("-$key")
                $argList.Add($val)
            }
        }

        $argsString = $argList -join ' '
        $psi = New-Object System.Diagnostics.ProcessStartInfo
        $psi.FileName = (Get-Process -Id $PID).Path
        $psi.Arguments = "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`" $argsString"
        $psi.UseShellExecute = $true
        $psi.Verb = "RunAs"
        [System.Diagnostics.Process]::Start($psi) | Out-Null
        exit 0
    }
}

function Get-NormalizedInstallDir {
    <#
    .SYNOPSIS
    Resolve the install root directory based on mode and parameters.
    #>
    if ($UserMode) {
        return Join-Path $env:LOCALAPPDATA "clashtui"
    }
    if ($InstallDir) {
        return $InstallDir
    }
    return "C:\Program Files\clashtui"
}

function Get-ClashtuiConfigDir {
    return Join-Path $env:APPDATA "clashtui"
}

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] $Message" -ForegroundColor Yellow
}

function Write-Error {
    param([string]$Message)
    Write-Host "[ERROR] $Message" -ForegroundColor Red
}

function Ensure-Directory {
    param([string]$Path)
    if (-not (Test-Path -LiteralPath $Path)) {
        New-Item -ItemType Directory -Path $Path -Force | Out-Null
    }
}

# --- Uninstall ---

function Invoke-Uninstall {
    Write-Warn "Please make sure mihomo and sing-box daemons have been stopped before uninstallation."
    $response = Read-Host "Continue with uninstallation? (y/N)"
    if ($response -ne "y" -and $response -ne "Y") {
        Write-Info "Uninstallation cancelled."
        return
    }

    $installDir = Get-NormalizedInstallDir
    $configDir = Get-ClashtuiConfigDir

    # Remove Windows Services via nssm
    foreach ($svc in @("clashtui_mihomo", "clashtui_singbox")) {
        $status = & nssm status $svc 2>$null
        if ($LASTEXITCODE -eq 0) {
            Write-Info "Removing service: $svc"
            & nssm remove $svc confirm 2>$null | Out-Null
        }
    }

    # Remove install directory
    if (Test-Path -LiteralPath $installDir) {
        Write-Info "Removing install directory: $installDir"
        Remove-Item -Recurse -Force -LiteralPath $installDir -ErrorAction SilentlyContinue
    }

    # Optionally remove config directory
    Write-Host ""
    $response = Read-Host "Remove config directory ($configDir)? (y/N)"
    if ($response -eq "y" -or $response -eq "Y") {
        if (Test-Path -LiteralPath $configDir) {
            Remove-Item -Recurse -Force -LiteralPath $configDir -ErrorAction SilentlyContinue
            Write-Info "Removed config directory."
        }
    }

    Write-Info "Uninstallation complete."
    exit 0
}

# --- Download helpers ---

function Get-GitHubLatestRelease {
    param(
        [string]$Owner,
        [string]$Repo
    )
    try {
        $url = "https://api.github.com/repos/$Owner/$Repo/releases/latest"
        $response = Invoke-RestMethod -Uri $url -TimeoutSec 15
        return $response.tag_name
    } catch {
        Write-Error "Failed to query latest release for $Owner/$Repo`: $_"
        return $null
    }
}

function Get-WindowsArch {
    $arch = $env:PROCESSOR_ARCHITECTURE
    if ($arch -eq "AMD64") { return "amd64" }
    if ($arch -eq "ARM64") { return "arm64" }
    return "386"
}

function Get-MihomoDownloadUrl {
    $arch = Get-WindowsArch
    $latestVer = Get-GitHubLatestRelease -Owner "MetaCubeX" -Repo "mihomo"
    if (-not $latestVer) { return $null }

    # mihomo release naming: mihomo-windows-amd64-v1.18.10.zip
    $name = "mihomo-windows-${arch}-${latestVer}.zip"
    return "https://github.com/MetaCubeX/mihomo/releases/latest/download/$name", $name, $latestVer
}

function Get-SingBoxDownloadUrl {
    $arch = Get-WindowsArch
    $latestVer = Get-GitHubLatestRelease -Owner "SagerNet" -Repo "sing-box"
    if (-not $latestVer) { return $null }

    # sing-box release naming: sing-box-1.13.11-windows-amd64.zip
    $name = "sing-box-${latestVer}-windows-${arch}.zip"
    return "https://github.com/SagerNet/sing-box/releases/latest/download/$name", $name, $latestVer
}

function Get-ClashtuiDownloadUrl {
    $arch = Get-WindowsArch
    $latestVer = Get-GitHubLatestRelease -Owner $Script:RepoOwner -Repo $Script:RepoName
    if (-not $latestVer) { return $null }

    $name = "clashtui-windows-${arch}-${latestVer}.zip"
    return "https://github.com/$Script:RepoOwner/$Script:RepoName/releases/latest/download/$name", $name, $latestVer
}

function Invoke-DownloadExtract {
    param(
        [string]$Url,
        [string]$DestDir,
        [string]$ExeName,
        [string]$Label
    )
    $tempDir = Join-Path $env:TEMP "clashtui_install_$(Get-Random)"
    Ensure-Directory $tempDir
    $zipPath = Join-Path $tempDir "$ExeName.zip"

    try {
        Write-Info "Downloading $Label from: $Url"
        Invoke-WebRequest -Uri $Url -OutFile $zipPath -UseBasicParsing

        Write-Info "Extracting $Label..."
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

        # Find the .exe file
        $exe = Get-ChildItem -Path $tempDir -Recurse -Filter "*.exe" |
            Where-Object { $_.Name -like "*$ExeName*" } |
            Select-Object -First 1

        if (-not $exe) {
            Write-Error "Could not find $ExeName.exe in the downloaded archive."
            return $false
        }

        Ensure-Directory $DestDir
        $destPath = Join-Path $DestDir "$ExeName.exe"
        Copy-Item -Path $exe.FullName -Destination $destPath -Force
        Write-Info "Installed $Label to: $destPath"
        return $true
    } catch {
        Write-Error "Failed to download/extract $Label`: $_"
        return $false
    } finally {
        Remove-Item -Recurse -Force -LiteralPath $tempDir -ErrorAction SilentlyContinue
    }
}

# --- Core installation ---

function Install-CoreMihomo {
    param([string]$InstallDir, [string]$ConfigDir)

    $mihomoDir = Join-Path $InstallDir "mihomo"
    $mihomoExe = Join-Path $mihomoDir "mihomo.exe"

    if (Get-Command "mihomo.exe" -ErrorAction SilentlyContinue) {
        Write-Info "mihomo.exe found in PATH, linking..."
        Ensure-Directory $mihomoDir
        $source = (Get-Command "mihomo.exe").Source
        try {
            New-Item -ItemType SymbolicLink -Path $mihomoExe -Target $source -Force -ErrorAction Stop | Out-Null
        } catch {
            Write-Warn "Cannot create symlink (no admin?), copying binary instead..."
            Copy-Item -Path $source -Destination $mihomoExe -Force
        }
    } elseif (Test-Path -LiteralPath $mihomoExe) {
        Write-Info "mihomo.exe already present at: $mihomoExe"
    } else {
        Write-Warn "mihomo.exe not found in PATH. You need to place mihomo.exe manually at:"
        Write-Warn "  $mihomoExe"
        Write-Warn "Or download from: https://github.com/MetaCubeX/mihomo/releases"

        $response = Read-Host "Download mihomo automatically from GitHub? (y/N)"
        if ($response -eq "y" -or $response -eq "Y") {
            $urlInfo = Get-MihomoDownloadUrl
            if ($urlInfo) {
                $url, $_, $ver = $urlInfo
                if (Invoke-DownloadExtract -Url $url -DestDir $mihomoDir -ExeName "mihomo" -Label "mihomo $ver") {
                    Write-Info "mihomo $ver installed successfully."
                }
            }
        }
    }

    # Create config directory
    $mihomoConfigDir = Join-Path $mihomoDir "config"
    Ensure-Directory $mihomoConfigDir
}

function Install-CoreSingBox {
    param([string]$InstallDir, [string]$ConfigDir)

    $singboxDir = Join-Path $InstallDir "sing-box"
    $singboxExe = Join-Path $singboxDir "sing-box.exe"

    if (Get-Command "sing-box.exe" -ErrorAction SilentlyContinue) {
        Write-Info "sing-box.exe found in PATH, linking..."
        Ensure-Directory $singboxDir
        $source = (Get-Command "sing-box.exe").Source
        try {
            New-Item -ItemType SymbolicLink -Path $singboxExe -Target $source -Force -ErrorAction Stop | Out-Null
        } catch {
            Write-Warn "Cannot create symlink (no admin?), copying binary instead..."
            Copy-Item -Path $source -Destination $singboxExe -Force
        }
    } elseif (Test-Path -LiteralPath $singboxExe) {
        Write-Info "sing-box.exe already present at: $singboxExe"
    } else {
        Write-Warn "sing-box.exe not found in PATH. You need to place sing-box.exe manually at:"
        Write-Warn "  $singboxExe"
        Write-Warn "Or download from: https://github.com/SagerNet/sing-box/releases"

        $response = Read-Host "Download sing-box automatically from GitHub? (y/N)"
        if ($response -eq "y" -or $response -eq "Y") {
            $urlInfo = Get-SingBoxDownloadUrl
            if ($urlInfo) {
                $url, $_, $ver = $urlInfo
                if (Invoke-DownloadExtract -Url $url -DestDir $singboxDir -ExeName "sing-box" -Label "sing-box $ver") {
                    Write-Info "sing-box $ver installed successfully."
                }
            }
        }
    }

    # Create config directory
    $singboxConfigDir = Join-Path $singboxDir "config"
    Ensure-Directory $singboxConfigDir
}

function Install-Clashtui {
    param([string]$InstallDir)

    $binDir = Join-Path $InstallDir "bin"
    $clashtuiExe = Join-Path $binDir "clashtui.exe"

    if (Get-Command "clashtui.exe" -ErrorAction SilentlyContinue) {
        Write-Info "clashtui.exe found in PATH, linking..."
        Ensure-Directory $binDir
        $source = (Get-Command "clashtui.exe").Source
        try {
            New-Item -ItemType SymbolicLink -Path $clashtuiExe -Target $source -Force -ErrorAction Stop | Out-Null
        } catch {
            Write-Warn "Cannot create symlink (no admin?), copying binary instead..."
            Copy-Item -Path $source -Destination $clashtuiExe -Force
        }
    } elseif (Test-Path -LiteralPath $clashtuiExe) {
        Write-Info "clashtui.exe already present at: $clashtuiExe"
    } else {
        Write-Warn "clashtui.exe not found. You need to place clashtui.exe manually at:"
        Write-Warn "  $clashtuiExe"
        Write-Warn "Or build from source: https://github.com/$Script:RepoOwner/$Script:RepoName"

        $response = Read-Host "Download clashtui automatically from GitHub? (y/N)"
        if ($response -eq "y" -or $response -eq "Y") {
            $urlInfo = Get-ClashtuiDownloadUrl
            if ($urlInfo) {
                $url, $_, $ver = $urlInfo
                if (Invoke-DownloadExtract -Url $url -DestDir $binDir -ExeName "clashtui" -Label "clashtui $ver") {
                    Write-Info "clashtui $ver installed successfully."
                }
            }
        }
    }
}

# --- Config file generation ---

function New-ClashtuiConfig {
    param(
        [string]$InstallDir,
        [string]$ConfigDir
    )

    Ensure-Directory $ConfigDir

    # Use forward slashes for YAML safety (Windows accepts both / and \)
    $mihomoCfgDir = ($InstallDir + "\mihomo\config") -replace '\\', '/'
    $mihomoBin     = ($InstallDir + "\mihomo\mihomo.exe") -replace '\\', '/'
    $mihomoCfgPath = ($InstallDir + "\mihomo\config\config.yaml") -replace '\\', '/'
    $singboxCfgDir = ($InstallDir + "\sing-box\config") -replace '\\', '/'
    $singboxBin     = ($InstallDir + "\sing-box\sing-box.exe") -replace '\\', '/'
    $singboxCfgPath = ($InstallDir + "\sing-box\config\config.json") -replace '\\', '/'

    # Backup existing config
    $configPath = Join-Path $ConfigDir "config.yaml"
    if (Test-Path -LiteralPath $configPath) {
        $i = 1
        while (Test-Path -LiteralPath "$configPath`_$i") { $i++ }
        Move-Item -LiteralPath $configPath -Destination "$configPath`_$i" -Force
        Write-Info "Existing config backed up to: $configPath`_$i"
    }

    $editorCmd = if ($env:EDITOR) { $env:EDITOR } else { "notepad" }
    $openDirCmd = "explorer"

    $configContent = @"
mihomo:
  core:
    config_dir: $mihomoCfgDir
    bin_path: $mihomoBin
    config_path: $mihomoCfgPath
  core_service:
    service_name: clashtui_mihomo
    is_user: false
singbox:
  core:
    bin_path: $singboxBin
    config_dir: $singboxCfgDir
    config_path: $singboxCfgPath
  core_service:
    service_name: clashtui_singbox
    is_user: false
timeout:
extra:
  edit_cmd: $editorCmd %s
  open_dir_cmd: $openDirCmd %s
"@

    $utf8 = [System.Text.UTF8Encoding]::new($false)
    [System.IO.File]::WriteAllText($configPath, $configContent, $utf8)
    Write-Info "Config written to: $configPath"
}

function New-CoreConfigs {
    param(
        [string]$InstallDir,
        [string]$CoreType
    )

    $clashtuiConfigDir = Get-ClashtuiConfigDir
    Ensure-Directory (Join-Path $clashtuiConfigDir "mihomo")
    Ensure-Directory (Join-Path $clashtuiConfigDir "sing-box")

    if ($CoreType -eq "mihomo" -or $CoreType -eq "all") {
        $mihomoDir = Join-Path $clashtuiConfigDir "mihomo"
        Ensure-Directory (Join-Path $mihomoDir "profiles")
        Ensure-Directory (Join-Path $mihomoDir "templates")

        # Copy default core override config from contrib if available, otherwise create default
        $overridePath = Join-Path $mihomoDir "core_override_config.yaml"
        if (-not (Test-Path -LiteralPath $overridePath)) {
            $defaultOverride = @"
external-controller: 127.0.0.1:9090
mixed-port: 7890
"@
            $utf8 = [System.Text.UTF8Encoding]::new($false)
            [System.IO.File]::WriteAllText($overridePath, $defaultOverride, $utf8)
            Write-Info "Created mihomo core override: $overridePath"
        }

        # Template proxy providers file
        $tppPath = Join-Path $mihomoDir "template_proxy_providers.yaml"
        if (-not (Test-Path -LiteralPath $tppPath)) {
            $tppContent = @"
# Define proxy-provider subscription URLs here, organized by group.
# In templates use `${PPG.<group>}` to reference all providers in a group,
# or `${PPG.<group>.<provider>}` for a specific one.
#
# Format:
#   <group-name>:
#     <provider-name>: "<subscription-url>"
#
# Example:
#   pvd:
#     pvd0: "https://example.com/sub1.yaml"
#     pvd1: "https://example.com/sub2.yaml"
"@
            $utf8 = [System.Text.UTF8Encoding]::new($false)
            [System.IO.File]::WriteAllText($tppPath, $tppContent, $utf8)
            Write-Info "Created mihomo template proxy providers: $tppPath"
        }

        # Core config.yaml (placeholder, managed by clashtui)
        $coreConfigPath = Join-Path $InstallDir "mihomo\config\config.yaml"
        if (-not (Test-Path -LiteralPath $coreConfigPath)) {
            $utf8 = [System.Text.UTF8Encoding]::new($false)
            [System.IO.File]::WriteAllText($coreConfigPath, $defaultOverride, $utf8)
            Write-Info "Created mihomo core config: $coreConfigPath"
        }
    }

    if ($CoreType -eq "sing-box" -or $CoreType -eq "all") {
        $singboxDir = Join-Path $clashtuiConfigDir "sing-box"
        Ensure-Directory (Join-Path $singboxDir "profiles")
        Ensure-Directory (Join-Path $singboxDir "templates")

        $overridePath = Join-Path $singboxDir "core_override_config.json"
        if (-not (Test-Path -LiteralPath $overridePath)) {
            $defaultOverride = @"
{
  "experimental": {
    "clash_api": {
      "external_controller": "127.0.0.1:9090",
      "secret": ""
    }
  },
  "inbounds": [
    {
      "type": "mixed",
      "tag": "mixed-in",
      "listen": "127.0.0.1",
      "listen_port": 7890
    }
  ],
  "log": {
    "level": "info"
  }
}
"@
            $utf8 = [System.Text.UTF8Encoding]::new($false)
            [System.IO.File]::WriteAllText($overridePath, $defaultOverride, $utf8)
            Write-Info "Created sing-box core override: $overridePath"
        }

        $tppPath = Join-Path $singboxDir "template_proxy_providers.yaml"
        if (-not (Test-Path -LiteralPath $tppPath)) {
            $tppContent = @"
# Define proxy-provider subscription URLs here, organized by group.
# In templates use `${PPG.<group>}` to reference all providers in a group,
# or `${PPG.<group>.<provider>}` for a specific one.
#
# Format:
#   <group-name>:
#     <provider-name>: "<subscription-url>"
#
# Example:
#   pvd:
#     pvd0: "https://example.com/sub1.yaml"
#     pvd1: "https://example.com/sub2.yaml"
"@
            $utf8 = [System.Text.UTF8Encoding]::new($false)
            [System.IO.File]::WriteAllText($tppPath, $tppContent, $utf8)
            Write-Info "Created sing-box template proxy providers: $tppPath"
        }

        $coreConfigPath = Join-Path $InstallDir "sing-box\config\config.json"
        if (-not (Test-Path -LiteralPath $coreConfigPath)) {
            $utf8 = [System.Text.UTF8Encoding]::new($false)
            [System.IO.File]::WriteAllText($coreConfigPath, $defaultOverride, $utf8)
            Write-Info "Created sing-box core config: $coreConfigPath"
        }
    }
}

function New-WindowsServices {
    param(
        [string]$InstallDir,
        [string]$CoreType
    )

    # Check nssm is available
    if (-not (Get-Command "nssm" -ErrorAction SilentlyContinue)) {
        Write-Warn "nssm not found. Install it first: scoop install nssm"
        Write-Warn "Or download from: https://nssm.cc/download"
        Write-Warn "Then register services manually:"
        Write-Warn "  clashtui service install mihomo"
        Write-Warn "  clashtui service install sing-box"
        return
    }

    $clashtuiExe = Join-Path $InstallDir "bin\clashtui.exe"
    if (-not (Test-Path -LiteralPath $clashtuiExe)) {
        Write-Warn "clashtui.exe not found at $clashtuiExe. Skipping service registration."
        Write-Warn "Run the following manually after placing clashtui.exe:"
        Write-Warn "  clashtui service install mihomo"
        Write-Warn "  clashtui service install sing-box"
        return
    }

    if ($CoreType -eq "mihomo" -or $CoreType -eq "all") {
        Write-Info "Registering mihomo Windows Service..."
        try {
            & $clashtuiExe service install mihomo 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Info "clashtui_mihomo service registered."
            } else {
                Write-Warn "clashtui_mihomo service registration may have failed (exit code: $LASTEXITCODE)"
                Write-Warn "You can register it manually: clashtui service install mihomo"
            }
        } catch {
            Write-Warn "Could not register mihomo service: $_"
            Write-Warn "You can register it manually: clashtui service install mihomo"
        }
    }

    if ($CoreType -eq "sing-box" -or $CoreType -eq "all") {
        Write-Info "Registering sing-box Windows Service..."
        try {
            & $clashtuiExe service install sing-box 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Info "clashtui_singbox service registered."
            } else {
                Write-Warn "clashtui_singbox service registration may have failed (exit code: $LASTEXITCODE)"
                Write-Warn "You can register it manually: clashtui service install sing-box"
            }
        } catch {
            Write-Warn "Could not register sing-box service: $_"
            Write-Warn "You can register it manually: clashtui service install sing-box"
        }
    }
}

# --- Template download (optional) ---

function Install-Templates {
    param([string]$CoreType)

    $clashtuiConfigDir = Get-ClashtuiConfigDir
    $mihomoTemplatesDir = Join-Path $clashtuiConfigDir "mihomo\templates"
    $singboxTemplatesDir = Join-Path $clashtuiConfigDir "sing-box\templates"

    if ($CoreType -eq "mihomo" -or $CoreType -eq "all") {
        $response = Read-Host "Download mihomo templates? (y/N)"
        if ($response -eq "y" -or $response -eq "Y") {
            Ensure-Directory $mihomoTemplatesDir
            $templates = @("common_tpl.yaml", "generic_tpl.yaml",
                "generic_tpl_with_all.yaml", "generic_tpl_with_filter.yaml",
                "generic_tpl_with_ruleset.yaml")
            foreach ($tpl in $templates) {
                try {
                    $url = "$Script:ContribUrlPrefix/templates/mihomo/$tpl"
                    $dest = Join-Path $mihomoTemplatesDir $tpl
                    Write-Info "Fetching: $url"
                    Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing
                    Write-Info "Downloaded: $tpl"
                } catch {
                    Write-Warn "Failed to download $tpl`: $_ ($url)"
                }
            }
        }
    }

    if ($CoreType -eq "sing-box" -or $CoreType -eq "all") {
        $response = Read-Host "Download sing-box templates? (y/N)"
        if ($response -eq "y" -or $response -eq "Y") {
            Ensure-Directory $singboxTemplatesDir
            $templates = @(
                "v1.12-common_tpl.json",
                "v1.12-tun_fakeip_bypass_dnsleak.json",
                "v1.12-tun_fakeip_bypass_no_dnsleak.json",
                "v1.12-tun_ipv4_ipv6.json",
                "v1.12-tun_ipv4_only.json"
            )
            foreach ($tpl in $templates) {
                try {
                    $url = "$Script:ContribUrlPrefix/templates/sing-box/$tpl"
                    $dest = Join-Path $singboxTemplatesDir $tpl
                    Write-Info "Fetching: $url"
                    Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing
                    Write-Info "Downloaded: $tpl"
                } catch {
                    Write-Warn "Failed to download $tpl`: $_ ($url)"
                }
            }
        }
    }
}

# --- Help ---

function Show-Help {
    @"
ClashTui Windows Install Script
================================

Usage:
  .\install.ps1 [options]

Options:
  -InstallDir <path>   Install directory (default: C:\Program Files\clashtui)
  -UserMode            Install to %LOCALAPPDATA%\clashtui (no admin required)
  -Core <type>         Core type: mihomo, sing-box, or all (default: all)
  -Repo <owner/repo>   GitHub repo for clashtui and contrib (default: JohanChane/clashtui)
  -Branch <name>       Branch for contrib resources (default: main)
  -Uninstall           Uninstall clashtui
  -Help                Show this help

Examples:
  # Default system mode install
  .\install.ps1

  # Custom install directory
  .\install.ps1 -InstallDir "D:\MyTools\clashtui"

  # User mode (no admin required)
  .\install.ps1 -UserMode

  # Install only mihomo core
  .\install.ps1 -Core mihomo

  # Uninstall
  .\install.ps1 -Uninstall

File structure after install:
  <InstallDir>\
    bin\clashtui.exe
    mihomo\
      config\config.yaml
      mihomo.exe
    sing-box\
      config\config.json
      sing-box.exe

Config files: %APPDATA%\clashtui\
"@
}

# --- Main ---

function Main {
    if ($Help) {
        Show-Help
        return
    }

    if ($Uninstall) {
        Invoke-Administrator
        Invoke-Uninstall
        return
    }

    # System mode requires administrator
    if (-not $UserMode) {
        Invoke-Administrator
    }

    $installDir = Get-NormalizedInstallDir
    $configDir = Get-ClashtuiConfigDir

    $modeLabel = if ($UserMode) { "user" } else { "system" }
    Write-Info "Install mode: $modeLabel"
    Write-Info "Install directory: $installDir"
    Write-Info "Config directory: $configDir"
    Write-Info "Core type: $Core"

    # Create directory structure
    Ensure-Directory (Join-Path $installDir "bin")
    Ensure-Directory (Join-Path $installDir "mihomo\config")
    Ensure-Directory (Join-Path $installDir "sing-box\config")

    # Install cores
    if ($Core -eq "mihomo" -or $Core -eq "all") {
        Install-CoreMihomo -InstallDir $installDir -ConfigDir $configDir
    }
    if ($Core -eq "sing-box" -or $Core -eq "all") {
        Install-CoreSingBox -InstallDir $installDir -ConfigDir $configDir
    }

    # Install clashtui
    Install-Clashtui -InstallDir $installDir

    # Generate configs
    New-ClashtuiConfig -InstallDir $installDir -ConfigDir $configDir
    New-CoreConfigs -InstallDir $installDir -CoreType $Core

    # Register Windows Services
    New-WindowsServices -InstallDir $installDir -CoreType $Core

    # Optional template downloads
    Install-Templates -CoreType $Core

    Write-Host ""
    Write-Info "Installation complete!"
    Write-Host ""

    Write-Info "Next steps:"
    Write-Info "  1. Place proxy core binaries (mihomo.exe / sing-box.exe) if not done above."
    Write-Info "  2. Edit config: notepad `"$configDir\config.yaml`""
    Write-Info "  3. Run clashtui:  `"$(Join-Path $installDir 'bin\clashtui.exe')`""
    Write-Host ""
    Write-Info "Windows Services are registered but NOT started."
    Write-Info "You can manage services from within clashtui (CoreSrvCtl tab, press 7)."
    Write-Info "Or use the clashtui CLI: clashtui service start"
    Write-Info "Or via nssm directly:"
    Write-Info "  nssm start clashtui_mihomo"
    Write-Info "  nssm stop clashtui_mihomo"
    Write-Info "  nssm status clashtui_mihomo"
}

Main

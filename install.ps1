<#
.SYNOPSIS
ClashTUI Installation and Management Script

.DESCRIPTION
Used to install, uninstall, and manage ClashTUI and Mihomo on Windows systems

.PARAMETER InstallDir
Installation directory path

.PARAMETER Uninstall
Uninstall ClashTUI and Mihomo

.PARAMETER Help
Show help information

.EXAMPLE
.\install.ps1 -InstallDir "D:\ClashTUI"
Install ClashTUI and Mihomo to the specified directory

.EXAMPLE
.\install.ps1 -Uninstall
Uninstall ClashTUI and Mihomo

.EXAMPLE
.\install.ps1 -Help
Show help information
#>

param(
    [Parameter(Mandatory=$false)]
    # [string]$InstallDir = "$env:LOCALAPPDATA\Programs\ClashTUI",
    [string]$InstallDir = "D:\ClashTUI",

    [Parameter(Mandatory=$false)]
    [switch]$Uninstall,

    [Parameter(Mandatory=$false)]
    [switch]$Help
)

# Global variables - Use user directory to avoid requiring admin privileges
$CLASHTUI_INSTALL_DIR = $InstallDir
$CLASHTUI_CONFIG_DIR = "$env:APPDATA\clashtui"
$MIHOMO_CONFIG_DIR = Join-Path $CLASHTUI_INSTALL_DIR "mihomo_config"
$INSTALL_RES_URL_PREFIX = "https://raw.githubusercontent.com/JohanChane/clashtui/refs/heads/main/InstallRes"

# Logging functions
function Log-Info {
    param([string]$message)
    Write-Host "[INFO] $message" -ForegroundColor Green
}

function Log-Warn {
    param([string]$message)
    Write-Host "[WARN] $message" -ForegroundColor Yellow
}

function Log-Error {
    param([string]$message)
    Write-Host "[ERROR] $message" -ForegroundColor Red
}

function Show-Help {
    Write-Host @"
ClashTUI Installation Management Script

Usage:
  .\install.ps1 [-InstallDir <path>] [-Help]

Parameters:
  -InstallDir <path>   Specify installation directory (default: $env:LOCALAPPDATA\ClashTUI)
  -Help                Show this help information

Examples:
  .\install.ps1 -InstallDir "$env:LOCALAPPDATA\ClashTUI"
  .\install.ps1 -Help

"@
}

function Detect-Architecture {
    $arch = $env:PROCESSOR_ARCHITECTURE

    switch ($arch) {
        "AMD64" { "amd64" }
        "ARM64" { "arm64" }
        "x86" { "386" }
        default { "unsupported" }
    }
}

function Detect-OS {
    "windows"
}

function Add-ToPath {
    param([string]$directory)

    # Modify user-level PATH, no admin privileges required
    $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($currentPath -notlike "*$directory*") {
        $newPath = $currentPath + ";" + $directory
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Log-Info "Added $directory to user PATH"
        return $true
    }
    return $false
}

function Get-CpuFeatures {
    $features = @{}

    # Get CPU features via registry
    try {
        $cpuReg = Get-ItemProperty "HKLM:\HARDWARE\DESCRIPTION\System\CentralProcessor\0"
        $features.ProcessorName = $cpuReg."ProcessorNameString"
        $features.Identifier = $cpuReg.Identifier
    } catch { }

    # Detect features using Windows API
    $features.SSE2 = [System.Runtime.Intrinsics.X86.Sse2]::IsSupported
    $features.SSE3 = [System.Runtime.Intrinsics.X86.Sse3]::IsSupported
    $features.SSSE3 = [System.Runtime.Intrinsics.X86.Ssse3]::IsSupported
    $features.SSE41 = [System.Runtime.Intrinsics.X86.Sse41]::IsSupported
    $features.SSE42 = [System.Runtime.Intrinsics.X86.Sse42]::IsSupported
    $features.AVX = [System.Runtime.Intrinsics.X86.Avx]::IsSupported
    $features.AVX2 = [System.Runtime.Intrinsics.X86.Avx2]::IsSupported
    $features.BMI1 = [System.Runtime.Intrinsics.X86.Bmi1]::IsSupported
    $features.BMI2 = [System.Runtime.Intrinsics.X86.Bmi2]::IsSupported
    $features.POPCNT = [System.Runtime.Intrinsics.X86.Popcnt]::IsSupported

    return $features
}

function Get-CpuLevel {
    $features = Get-CpuFeatures

    # Check if it's x64 architecture
    if (-not [Environment]::Is64BitProcess) {
        return ""
    }

    # Define requirements for each level
    $v1 = $features.SSE2
    $v2 = $v1 -and $features.SSE3 -and $features.SSSE3 -and $features.SSE41 -and $features.SSE42 -and $features.POPCNT
    $v3 = $v2 -and $features.AVX -and $features.AVX2 -and $features.BMI1 -and $features.BMI2

    if ($v3) { return "-v3" }
    if ($v2) { return "-v2" }
    if ($v1) { return "-v1" }

    return ""
}

function Download-Mihomo {
    param([string]$outputPath)

    $baseArch = Detect-Architecture
    if ($baseArch -eq "unsupported") {
        Log-Error "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE"
        return $false
    }

    $arch = if ($baseArch -eq "amd64") {
        $cpuLevel = Get-CpuLevel
        "${baseArch}${cpuLevel}"
    } else {
        $baseArch
    }
    $os = Detect-OS

    # Get latest version
    try {
        Log-Info "Fetching latest Mihomo version information..."
        $latestRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/MetaCubeX/mihomo/releases/latest"
        $latestVersion = $latestRelease.tag_name.Trim('v')

        # Build download URL
        $downloadUrl = "https://github.com/MetaCubeX/mihomo/releases/download/v${latestVersion}/mihomo-windows-${arch}-v${latestVersion}.zip"

        Log-Info "Downloading Mihomo version: $latestVersion"
        Log-Info "Download URL: $downloadUrl"

        # Create temporary directory
        $tempDir = Join-Path $env:TEMP "mihomo_temp"
        if (Test-Path $tempDir) {
            Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
        }
        New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

        # Download file
        $zipPath = Join-Path $tempDir "mihomo.zip"
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UserAgent "PowerShell"

        # Extract file
        Log-Info "Extracting Mihomo..."
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

        # Find executable file
        $exePath = Get-ChildItem -Path $tempDir -Recurse -Include "*.exe" | Where-Object {
              $_.Name -match "^mihomo" -and $_.Extension -eq ".exe"
        } | Select-Object -First 1
        if (-not $exePath) {
            Log-Error "Could not find Mihomo executable in downloaded files"
            return $false
        }

        # Copy to target location
        Copy-Item -Path $exePath.FullName -Destination $outputPath -Force
        Log-Info "Mihomo downloaded to: $outputPath"

        # Clean up temporary files
        Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue

        return $true
    }
    catch {
        Log-Error "Failed to download Mihomo: $($_.Exception.Message)"
        return $false
    }
}

function Configure-MihomoFirewall {
    param(
        [string]$mihomoPath
    )

    Log-Warn "If you cannot access external networks, you may need to allow ""$mihomoPath"" through the firewall"
    return $true
}

function Install-Mihomo {
    $mihomoPath = Join-Path $CLASHTUI_INSTALL_DIR "mihomo.exe"

    # Check if already in PATH
    $existingMihomo = Get-Command "mihomo" -ErrorAction SilentlyContinue
    $existingClashMeta = Get-Command "clash-meta" -ErrorAction SilentlyContinue

    if ($existingMihomo -or $existingClashMeta) {
        Log-Info "Detected existing Mihomo/Clash-Meta installation in system"

        # Record location of existing installation
        if ($existingMihomo) {
            $sourcePath = $existingMihomo.Source
        } else {
            $sourcePath = $existingClashMeta.Source
        }

        if (Test-Path $sourcePath) {
            Log-Info "Using existing Mihomo installation: $sourcePath"
            return $sourcePath
        }
    }

    # If not found, download and install
    Log-Info "No existing Mihomo installation found, starting download and installation..."

    # Ensure installation directory exists
    if (-not (Test-Path $CLASHTUI_INSTALL_DIR)) {
        New-Item -ItemType Directory -Path $CLASHTUI_INSTALL_DIR -Force | Out-Null
    }

    $downloadResult = Download-Mihomo -outputPath $mihomoPath
    if (-not $downloadResult) {
        return $false
    }

    $firewallResult = Configure-MihomoFirewall -mihomoPath $mihomoPath
    if (-not $firewallResult) {
        Log-Warn "Firewall configuration failed, may require manual configuration"
    }

    return $mihomoPath
}

function Download-ClashTUI {
    param([string]$outputPath)

    $arch = Detect-Architecture
    $os = Detect-OS

    if ($arch -eq "unsupported") {
        Log-Error "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE"
        return $false
    }

    # Get latest version
    try {
        Log-Info "Fetching latest ClashTUI version information..."
        $latestRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/JohanChane/clashtui/releases/latest"
        $latestVersion = $latestRelease.tag_name

        # Build download URL - Windows platform uses zip format
        $downloadUrl = "https://github.com/JohanChane/clashtui/releases/latest/download/clashtui-${os}-${arch}-${latestVersion}.zip"

        Log-Info "Downloading ClashTUI version: $latestVersion"
        Log-Info "Download URL: $downloadUrl"

        # Create temporary directory
        $tempDir = Join-Path $env:TEMP "clashtui_temp"
        if (Test-Path $tempDir) {
            Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
        }
        New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

        # Download file
        $zipPath = Join-Path $tempDir "clashtui.zip"
        Invoke-WebRequest -Uri $downloadUrl -OutFile $zipPath -UserAgent "PowerShell"

        # Extract zip file
        Log-Info "Extracting ClashTUI..."
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

        # Find executable file
        $exePath = Get-ChildItem -Path $tempDir -Recurse -Include "clashtui*.exe", "*.exe" | Where-Object {
            $_.Name -match "^clashtui" -and $_.Extension -eq ".exe"
        } | Select-Object -First 1

        if (-not $exePath) {
            # If standard naming not found, look for any exe file
            $allExeFiles = Get-ChildItem -Path $tempDir -Recurse -Include "*.exe"
            if ($allExeFiles.Count -eq 0) {
                Log-Error "Could not find ClashTUI executable in downloaded files"
                Log-Error "Files in extracted directory:"
                Get-ChildItem -Path $tempDir -Recurse -File | ForEach-Object {
                    Log-Error "  $($_.FullName)"
                }
                return $false
            }
            $exePath = $allExeFiles | Select-Object -First 1
            Log-Warn "Standard named executable not found, using: $($exePath.Name)"
        }

        Log-Info "Found executable: $($exePath.Name)"

        # Copy to target location
        Copy-Item -Path $exePath.FullName -Destination $outputPath -Force
        Log-Info "ClashTUI downloaded to: $outputPath"

        # Clean up temporary files
        Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue

        return $true
    }
    catch {
        Log-Error "Failed to download ClashTUI: $($_.Exception.Message)"
        return $false
    }
}

function Install-ClashTUI {
    $clashtuiPath = Join-Path $CLASHTUI_INSTALL_DIR "clashtui.exe"

    # Check if already in PATH
    $existingClashTUI = Get-Command "clashtui" -ErrorAction SilentlyContinue
    if ($existingClashTUI) {
        Log-Info "Detected existing ClashTUI installation: $($existingClashTUI.Source)"
        return $true
    }

    # If not found, download and install
    Log-Info "No existing ClashTUI installation found, starting download and installation..."

    # Ensure installation directory exists
    if (-not (Test-Path $CLASHTUI_INSTALL_DIR)) {
        New-Item -ItemType Directory -Path $CLASHTUI_INSTALL_DIR -Force | Out-Null
    }

    $downloadResult = Download-ClashTUI -outputPath $clashtuiPath
    if (-not $downloadResult) {
        return $false
    }

    # Add to PATH
    $pathAdded = Add-ToPath -directory $CLASHTUI_INSTALL_DIR
    if ($pathAdded) {
        Log-Info "ClashTUI installed and added to PATH"
    } else {
        Log-Info "ClashTUI installed"
    }

    return $true
}

function Create-ConfigDirectory {
    # Create configuration directory
    if (-not (Test-Path $CLASHTUI_CONFIG_DIR)) {
        New-Item -ItemType Directory -Path $CLASHTUI_CONFIG_DIR -Force | Out-Null
        Log-Info "Created configuration directory: $CLASHTUI_CONFIG_DIR"
    } else {
        Log-Info "Configuration directory already exists: $CLASHTUI_CONFIG_DIR"
    }

    # Create Mihomo configuration directory
    if (-not (Test-Path $MIHOMO_CONFIG_DIR)) {
        New-Item -ItemType Directory -Path $MIHOMO_CONFIG_DIR -Force | Out-Null
        Log-Info "Created Mihomo configuration directory: $MIHOMO_CONFIG_DIR"
    } else {
        Log-Info "Mihomo configuration directory already exists: $MIHOMO_CONFIG_DIR"
    }

    # Download GeoIP and Geosite database files
    $geoIPUrl = "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geoip.metadb"
    $geositeUrl = "https://github.com/MetaCubeX/meta-rules-dat/releases/download/latest/geosite.dat"

    $geoIPPath = Join-Path $MIHOMO_CONFIG_DIR "geoip.metadb"
    $geositePath = Join-Path $MIHOMO_CONFIG_DIR "geosite.dat"

    try {
        # Download geoip.metadb
        if (-not (Test-Path $geoIPPath)) {
            Log-Info "Downloading geoip.metadb..."
            Invoke-WebRequest -Uri $geoIPUrl -OutFile $geoIPPath -UseBasicParsing
            Log-Info "Downloaded geoip.metadb to: $geoIPPath"
        } else {
            Log-Info "geoip.metadb already exists: $geoIPPath"
        }

        # Download geosite.dat
        if (-not (Test-Path $geositePath)) {
            Log-Info "Downloading geosite.dat..."
            Invoke-WebRequest -Uri $geositeUrl -OutFile $geositePath -UseBasicParsing
            Log-Info "Downloaded geosite.dat to: $geositePath"
        } else {
            Log-Info "geosite.dat already exists: $geositePath"
        }
    } catch {
        Log-Error "Error downloading files: $($_.Exception.Message)"
        Write-Host "Please check your network connection or manually download files to $MIHOMO_CONFIG_DIR directory" -ForegroundColor Yellow
    }

    # Create profiles directory
    $profilesDir = Join-Path $CLASHTUI_CONFIG_DIR "profiles"
    if (-not (Test-Path $profilesDir)) {
        New-Item -ItemType Directory -Path $profilesDir -Force | Out-Null
        Log-Info "Created profiles directory: $profilesDir"
    } else {
        Log-Info "Profiles directory already exists: $profilesDir"
    }

    # Create templates directory
    $templatesDir = Join-Path $CLASHTUI_CONFIG_DIR "templates"
    if (-not (Test-Path $templatesDir)) {
        New-Item -ItemType Directory -Path $templatesDir -Force | Out-Null
        Log-Info "Created templates directory: $templatesDir"
    } else {
        Log-Info "Templates directory already exists: $templatesDir"
    }

    # Create template_proxy_providers file
    $templateProxyProviders = Join-Path $templatesDir "template_proxy_providers"
    if (-not (Test-Path $templateProxyProviders)) {
        @"
# This is a comment
# Place each subscription on a separate line
"@ | Set-Content -Path "$templateProxyProviders" -Encoding UTF8 | Out-Null
        Log-Info "Created template_proxy_providers file: $templateProxyProviders"
    } else {
        Log-Info "template_proxy_providers file already exists: $templateProxyProviders"
    }

    # Generate basic_clash_config.yaml
    $basicConfigPath = Join-Path $CLASHTUI_CONFIG_DIR "basic_clash_config.yaml"
    $response = Read-Host "Do you want to download basic_clash_config.yaml? (y/N)"
    if ($response -eq "y" -or $response -eq "Y") {
        if (Test-Path $basicConfigPath) {
            Copy-Item "$basicConfigPath" "${basicConfigPath}.old" -Force
        }
        $basicConfigUrl = "$INSTALL_RES_URL_PREFIX/basic_clash_config.yaml"
        Log-Info "Downloading basic_clash_config.yaml..."
        Invoke-WebRequest -Uri $basicConfigUrl -OutFile $basicConfigPath
        Log-Info "Downloaded basic_clash_config.yaml to ""$basicConfigPath"""
    } elseif (-not (Test-Path $basicConfigPath)) {
        $basicConfigContent = @"
mixed-port: 7890
mode: rule
log-level: info
external-controller: 127.0.0.1:9090
"@
        Set-Content -Path $basicConfigPath -Value $basicConfigContent -Encoding UTF8
        Log-Info "Generated basic configuration file: $basicConfigPath"
        Log-Info "Basic configuration file content:"
        Write-Host "=== Basic Configuration File Content ===" -ForegroundColor Cyan
        Write-Host $basicConfigContent -ForegroundColor Cyan
        Write-Host "=======================================" -ForegroundColor Cyan
    } else {
        Log-Info "Basic configuration file already exists: $basicConfigPath"
    }

    # Copy basic_clash_config.yaml to $MIHOMO_CONFIG_DIR/config.yaml
    $mihomoConfigPath = Join-Path $MIHOMO_CONFIG_DIR "config.yaml"
    if (-not (Test-Path $mihomoConfigPath)) {
        try {
            Copy-Item -Path $basicConfigPath -Destination $mihomoConfigPath -Force
            Log-Info "Copied basic configuration file to Mihomo configuration directory: $mihomoConfigPath"
        } catch {
            Log-Error "Error copying configuration file: $($_.Exception.Message)"
            Write-Host "Unable to copy configuration file, please do it manually" -ForegroundColor Red
        }
    }

    # Ask if user wants to download templates
    Download-Templates
}

function Download-Templates {
    $response = Read-Host "Do you want to download template files? (y/N)"

    if ($response -eq "y" -or $response -eq "Y") {
        Log-Info "Downloading template files..."

        $templates = @(
            @{
                Name = "common_tpl.yaml"
                Url = "$INSTALL_RES_URL_PREFIX/templates/common_tpl.yaml"
            },
            @{
                Name = "generic_tpl.yaml"
                Url = "$INSTALL_RES_URL_PREFIX/templates/generic_tpl.yaml"
            },
            @{
                Name = "generic_tpl_with_all.yaml"
                Url = "$INSTALL_RES_URL_PREFIX/templates/generic_tpl_with_all.yaml"
            },
            @{
                Name = "generic_tpl_with_filter.yaml"
                Url = "$INSTALL_RES_URL_PREFIX/templates/generic_tpl_with_filter.yaml"
            },
            @{
                Name = "generic_tpl_with_ruleset.yaml"
                Url = "$INSTALL_RES_URL_PREFIX/templates/generic_tpl_with_ruleset.yaml"
            }
        )

        $templatesDir = Join-Path $CLASHTUI_CONFIG_DIR "templates"

        foreach ($template in $templates) {
            $outputPath = Join-Path $templatesDir $template.Name
            try {
                Invoke-WebRequest -Uri $template.Url -OutFile $outputPath -UserAgent "PowerShell"
                Log-Info "Downloaded: $($template.Name)"
            }
            catch {
                Log-Error "Failed to download $($template.Name): $($_.Exception.Message)"
            }
        }

        Log-Info "Template download completed"
    } else {
        Log-Info "Skipping template download"
    }
}

function Generate-ConfigFile {
    param(
        [string]$mihomoPath,
        [string]$configDir,
        [string]$configPath
    )

    # Replace Windows path separator \ with /
    $mihomoPath = $mihomoPath -replace '\\', '/'
    $configDir = $configDir -replace '\\', '/'
    $configPath = $configPath -replace '\\', '/'

    $configContent = @"
clash_cfg_dir: '$configDir'
clash_cfg_path: '$configPath'
clash_core_path: '$mihomoPath'
clash_srv_name: 'clashtui_mihomo'
#edit_cmd: ''
#open_dir_cmd: ''
"@

    $configFile = Join-Path $CLASHTUI_CONFIG_DIR "config.yaml"

    if (Test-Path "$configFile") {
        Copy-Item "$configFile" "${configFile}.old" -Force
    }

    try {
        Set-Content -Path $configFile -Value $configContent -Encoding UTF8
        Log-Info "Generated configuration file: $configFile"
        Log-Info "Configuration file content:"
        Write-Host "=== Configuration File Content ===" -ForegroundColor Cyan
        Write-Host $configContent -ForegroundColor Cyan
        Write-Host "================================" -ForegroundColor Cyan
        return $true
    }
    catch {
        Log-Error "Failed to generate configuration file: $($_.Exception.Message)"
        return $false
    }
}

function Download-Nssm {
    param([string]$outputDir)

    $arch = Detect-Architecture
    if ($arch -eq "unsupported") {
        Log-Error "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE"
        return $false
    }

    # Map architecture to NSSM architecture name
    $nssmArch = if ($arch -eq "amd64") { "win64" } else { "win32" }

    try {
        Log-Info "Fetching NSSM latest version information..."
        $nssmUrl = "https://nssm.cc/release/nssm-2.24.zip"

        Log-Info "Downloading NSSM..."
        Log-Info "Download URL: $nssmUrl"

        # Create temporary directory
        $tempDir = Join-Path $env:TEMP "nssm_temp"
        if (Test-Path $tempDir) {
            Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue
        }
        New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

        # Download file
        $zipPath = Join-Path $tempDir "nssm.zip"
        Invoke-WebRequest -Uri $nssmUrl -OutFile $zipPath -UserAgent "PowerShell"

        # Extract file
        Log-Info "Extracting NSSM..."
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

        # Find executable file
        $nssmExePath = Get-ChildItem -Path $tempDir -Recurse -Include "nssm.exe" | Select-Object -First 1
        if (-not $nssmExePath) {
            Log-Error "Could not find nssm.exe in downloaded files"
            return $false
        }

        # Copy to target location
        $targetPath = Join-Path $outputDir "nssm.exe"
        Copy-Item -Path $nssmExePath.FullName -Destination $targetPath -Force
        Log-Info "NSSM downloaded to: $targetPath"

        # Clean up temporary files
        Remove-Item -Recurse -Force $tempDir -ErrorAction SilentlyContinue

        # Add to PATH
        $pathAdded = Add-ToPath -directory $CLASHTUI_INSTALL_DIR
        if ($pathAdded) {
            Log-Info "NSSM installed and added to PATH"
        } else {
            Log-Info "NSSM installed"
        }

        return $true
    }
    catch {
        Log-Error "Failed to download NSSM: $($_.Exception.Message)"
        return $false
    }
}

function Install-Nssm {
    # Check if already in PATH
    $existingNssm = Get-Command "nssm" -ErrorAction SilentlyContinue
    if ($existingNssm) {
        Log-Info "Detected existing NSSM installation: $($existingNssm.Source)"
        return $true
    }

    Log-Info "No existing NSSM installation found, starting download and installation..."

    # Ensure installation directory exists
    if (-not (Test-Path $CLASHTUI_INSTALL_DIR)) {
        New-Item -ItemType Directory -Path $CLASHTUI_INSTALL_DIR -Force | Out-Null
    }

    # Download NSSM
    $downloadResult = Download-Nssm -outputDir $CLASHTUI_INSTALL_DIR
    if (-not $downloadResult) {
        Log-Error "NSSM download failed"
        return $false
    }

    return $true
}

function Install-Loopback {
    $loopbackPath = Join-Path $CLASHTUI_INSTALL_DIR "EnableLoopback.exe"

    # Check if already exists
    if (Test-Path $loopbackPath) {
        Log-Info "EnableLoopback already exists: $loopbackPath"
        return $true
    }

    Log-Info "Downloading EnableLoopback tool..."

    try {
        $downloadUrl = "https://telerik-fiddler.s3.amazonaws.com/fiddler/addons/enableloopbackutility.exe"

        # Ensure installation directory exists
        if (-not (Test-Path $CLASHTUI_INSTALL_DIR)) {
            New-Item -ItemType Directory -Path $CLASHTUI_INSTALL_DIR -Force | Out-Null
        }

        # Download file
        Invoke-WebRequest -Uri $downloadUrl -OutFile $loopbackPath -UserAgent "PowerShell"

        Log-Info "EnableLoopback downloaded to: $loopbackPath"

        # Add to PATH
        $pathAdded = Add-ToPath -directory $CLASHTUI_INSTALL_DIR
        if ($pathAdded) {
            Log-Info "EnableLoopback installed and added to PATH"
        }

        return $true
    }
    catch {
        Log-Error "Failed to download EnableLoopback: $($_.Exception.Message)"
        return $false
    }
}

function Uninstall-ClashTUI {
    Log-Info "Please perform the following steps manually:"
    Log-Info "1. Uninstall clashtui_mihomo (default) service"
    Log-Info "2. Delete installation directory ""$CLASHTUI_INSTALL_DIR"" (default)"
    Log-Info "3. Remove clashtui PATH entry ""$CLASHTUI_INSTALL_DIR"" (default)"
    Log-Info "4. [Optional] Delete ClashTUI configuration directory ""$CLASHTUI_CONFIG_DIR"" (default)"
}

function Main {
    Log-Info "Starting installation of ClashTUI and Mihomo"
    Log-Info "Installation directory: $CLASHTUI_INSTALL_DIR"
    Log-Info "Configuration directory: $CLASHTUI_CONFIG_DIR"
    Log-Info "Mihomo configuration directory: $MIHOMO_CONFIG_DIR"

    # Install Mihomo
    Log-Info "=== Installing Mihomo ==="
    $mihomoResult = Install-Mihomo
    if (-not $mihomoResult) {
        Log-Error "Mihomo installation failed"
        exit 1
    }

    # Install ClashTUI
    Log-Info "=== Installing ClashTUI ==="
    $clashtuiResult = Install-ClashTUI
    if (-not $clashtuiResult) {
        Log-Error "ClashTUI installation failed"
        exit 1
    }

    # Create configuration directory
    Log-Info "=== Creating configuration directories ==="
    Create-ConfigDirectory

    # Generate configuration file
    Log-Info "=== Generating configuration file ==="
    $mihomoConfigPath = Join-Path $MIHOMO_CONFIG_DIR "config.yaml"
    $configResult = Generate-ConfigFile -mihomoPath $mihomoResult -configDir $MIHOMO_CONFIG_DIR -configPath $mihomoConfigPath
    if (-not $configResult) {
        Log-Warn "Configuration file generation failed, but installation continues"
    }

    # Install NSSM (for service management)
    Log-Info "=== Installing NSSM ==="
    $nssmResult = Install-Nssm
    if (-not $nssmResult) {
        Log-Warn "NSSM installation failed, service management features may not be available"
    }

    # Install EnableLoopback tool
    Log-Info "=== Installing EnableLoopback tool ==="
    $loopbackResult = Install-Loopback
    if (-not $loopbackResult) {
        Log-Warn "EnableLoopback installation failed, UWP app loopback proxy configuration may not work"
    }

    Log-Info "=== Installation completed ==="
    Log-Info "ClashTUI and Mihomo successfully installed"
    Log-Info "Installation directory: $CLASHTUI_INSTALL_DIR"
    Log-Info "Configuration directory: $CLASHTUI_CONFIG_DIR"
    Log-Info "Mihomo configuration directory: $MIHOMO_CONFIG_DIR"
    Log-Info "Configuration file: $CLASHTUI_CONFIG_DIR\config.yaml"
    Log-Info "Mihomo location: $mihomoResult"
    Log-Warn "Please restart your terminal"

    Log-Info "=== Next steps ==="
    Log-Info "1. Check clashtui configuration"
    Log-Info "2. Use clashtui to install mihomo service"
    Log-Info "3. Use clashtui to start mihomo service, and may need to wait for mihomo to download required files"
    Log-Info "4. [Optional] Configure tun mode"
    Log-Info "5. Import a profile"

    return $true
}

# Main program entry
if ($Help) {
    Show-Help
    exit 0
}

if ($Uninstall) {
    $result = Uninstall-ClashTUI
    exit $(if ($result) { 0 } else { 1 })
}

# Execute installation
$result = Main
exit $(if ($result) { 0 } else { 1 })

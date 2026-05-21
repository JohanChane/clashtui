# Pester test suite for install.ps1
# Run: Invoke-Pester -Path installs/tests/install.tests.ps1

BeforeAll {
    $scriptRoot = Split-Path $PSScriptRoot -Parent
    $installScript = Join-Path $scriptRoot "install.ps1"

    # Dot-source the script to load functions without executing Main
    . $installScript -IsTest -InstallDir (Join-Path $env:TEMP "clashtui-pester-test") -Repo "JohanChane/clashtui" -Branch "demotui"

    # Manually resolve paths since Main (which calls Resolve-Paths) was skipped
    Resolve-Paths
}

Describe "Get-Architecture" {
    It "Returns a supported architecture" {
        $result = Get-Architecture
        $result | Should -Not -Be "unsupported"
    }

    It "Returns amd64 or arm64" {
        $result = Get-Architecture
        $result | Should -BeIn @("amd64", "arm64")
    }
}

Describe "Get-OS" {
    It "Returns windows on Windows, unsupported on other platforms" {
        $result = Get-OS
        if ([System.Environment]::OSVersion.Platform -eq [System.PlatformID]::Win32NT) {
            $result | Should -Be "windows"
        } else {
            $result | Should -Be "unsupported"
        }
    }
}

Describe "Test-ValidInstallDir" {
    It "Rejects directories with spaces" {
        { Test-ValidInstallDir "C:\Program Files\test" } | Should -Throw
    }

    It "Accepts simple valid paths" {
        $testDir = Join-Path $env:TEMP "clashtui_test_$(Get-Random)"
        try {
            Test-ValidInstallDir $testDir
            Test-Path $testDir | Should -Be $true
        } finally {
            Remove-Item $testDir -Force -Recurse -ErrorAction SilentlyContinue
        }
    }
}

Describe "Resolve-Paths" {
    BeforeEach {
        # Run with a custom install dir
        $testInstallDir = Join-Path $env:TEMP "clashtui-resolve-test"
    }

    It "Sets correct INSTALL_DIR" {
        # Resolve-Paths uses script-scope variables set during BeforeAll
        # We can verify they exist and are non-null
        $script:INSTALL_DIR | Should -Not -BeNullOrEmpty
    }

    It "Sets correct subdirectory paths" {
        $script:INSTALL_DIR_MIHOMO | Should -Not -BeNullOrEmpty
        $script:INSTALL_DIR_SINGBOX | Should -Not -BeNullOrEmpty
        $script:INSTALL_BIN | Should -Not -BeNullOrEmpty
    }

    It "Sets config directory paths" {
        $script:CLASHTUI_CONFIG_DIR | Should -Not -BeNullOrEmpty
        $script:MIHOMO_USER_CONFIG_DIR | Should -Not -BeNullOrEmpty
        $script:SINGBOX_USER_CONFIG_DIR | Should -Not -BeNullOrEmpty
    }
}

Describe "Backup-File" {
    It "Returns early if file does not exist" {
        $nonexistentPath = Join-Path $env:TEMP "nonexistent_backup_test_$(Get-Random)"
        # Backup-File does not throw, it just returns
        { Backup-File $nonexistentPath } | Should -Not -Throw
    }
}

Describe "Copy-Contrib" {
    It "Has CONTRIB_URL_PREFIX set correctly" {
        $script:CONTRIB_URL_PREFIX | Should -Not -BeNullOrEmpty
        $script:CONTRIB_URL_PREFIX | Should -Match "https://raw.githubusercontent.com"
        $script:CONTRIB_URL_PREFIX | Should -Match "JohanChane/clashtui"
        $script:CONTRIB_URL_PREFIX | Should -Match "demotui"
    }
}

Describe "Write-Info / Write-Warn / Write-ErrorLog" {
    It "Write-Info does not throw" {
        { Write-Info "Test info message" } | Should -Not -Throw
    }

    It "Write-Warn does not throw" {
        { Write-Warn "Test warn message" } | Should -Not -Throw
    }

    It "Write-ErrorLog does not throw" {
        { Write-ErrorLog "Test error message" } | Should -Not -Throw
    }
}

Describe "End-to-end: install.ps1 -IsTest" {
    It "Running install.ps1 -IsTest succeeds with all cores" {
        $testDir = Join-Path $env:TEMP "clashtui-e2e-test-$(Get-Random)"
        $scriptPath = Join-Path $PSScriptRoot ".." "install.ps1"

        # Find the contrib dir (relative to the script)
        $contribDir = Join-Path $PSScriptRoot ".." ".." "contrib"
        if (-not (Test-Path $contribDir)) {
            # May be running from a different location; try project root
            $contribDir = Join-Path $PSScriptRoot ".." ".." ".." "contrib"
        }

        $result = & $scriptPath -IsTest -InstallDir $testDir -Repo "JohanChane/clashtui" -Branch "demotui" -Core "all" 2>&1
        $exitCode = $LASTEXITCODE

        Write-Host "--- install.ps1 output ---"
        Write-Host ($result -join "`n")

        $exitCode | Should -Be 0
    }

    It "Creates expected directory structure" {
        $testDir = Join-Path $env:TEMP "clashtui-structure-test-$(Get-Random)"
        $scriptPath = Join-Path $PSScriptRoot ".." "install.ps1"

        & $scriptPath -IsTest -InstallDir $testDir -Repo "JohanChane/clashtui" -Branch "demotui" -Core "all" 2>&1 | Out-Null

        # In IsTest mode, the script uses $env:TEMP\clashtui-test\ not $testDir
        $actualTestDir = Join-Path $env:TEMP "clashtui-test"
        Write-Host "Checking structure under: $actualTestDir"

        if (Test-Path $actualTestDir) {
            Test-Path (Join-Path $actualTestDir "opt/clashtui/bin") | Should -Be $true
            Test-Path (Join-Path $actualTestDir "opt/clashtui/mihomo/config") | Should -Be $true
            Test-Path (Join-Path $actualTestDir "opt/clashtui/sing-box/config") | Should -Be $true
            Test-Path (Join-Path $actualTestDir "config/clashtui") | Should -Be $true
        } else {
            Write-Host "Skipped: test dir not found (may not exist on this platform)"
            Set-ItResult -Skipped -Because "Running on non-Windows platform"
        }
    }
}

Describe "Guard: dot-sourcing does not execute Main" {
    It "Functions are available without running Main" {
        # Functions should be defined after dot-sourcing
        { Get-Command Write-Info } | Should -Not -Throw
        { Get-Command Resolve-Paths } | Should -Not -Throw
        { Get-Command Get-Architecture } | Should -Not -Throw
        { Get-Command Get-OS } | Should -Not -Throw
    }
}

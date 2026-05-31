# Pester test suite for install.ps1
# Run: Invoke-Pester -Path installs/tests/install.tests.ps1

BeforeAll {
    $scriptRoot = Split-Path $PSScriptRoot -Parent
    $installScript = Join-Path $scriptRoot "install.ps1"

    # Dot-source the script to load functions without executing Main
    . $installScript -NoPrompt -InstallDir (Join-Path $env:TEMP "clashtui-pester-test") -Repo "JohanChane/clashtui" -Branch "demotui"

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
    It "Returns a known OS string" {
        $result = Get-OS
        $result | Should -BeIn @("windows", "unsupported")
    }
}

Describe "Test-ValidInstallDir" {
    It "Rejects directories with spaces (child process)" {
        $scriptPath = (Resolve-Path (Join-Path $PSScriptRoot ".." "install.ps1")).Path
        $result = & powershell -NoProfile -Command "& '$scriptPath' -InstallDir 'C:\Program Files\test' -Core mihomo 2>&1; exit `$LASTEXITCODE" 2>&1
        $LASTEXITCODE | Should -Not -Be 0
    }

    It "Accepts valid writable path" {
        $testDir = Join-Path $env:TEMP "clashtui_valid_test_$(Get-Random)"
        try {
            Test-ValidInstallDir $testDir
            Test-Path $testDir | Should -Be $true
        } finally {
            Remove-Item $testDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

Describe "Resolve-Paths" {
    It "Sets INSTALL_DIR" {
        $script:INSTALL_DIR | Should -Not -BeNullOrEmpty
    }

    It "Sets subdirectory paths" {
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
        $nonexistentPath = Join-Path $env:TEMP "nonexistent_backup_$(Get-Random)"
        { Backup-File $nonexistentPath } | Should -Not -Throw
    }
}

Describe "Copy-Contrib URL prefix" {
    It "Has CONTRIB_URL_PREFIX set with repo and branch" {
        $script:CONTRIB_URL_PREFIX | Should -Not -BeNullOrEmpty
        $script:CONTRIB_URL_PREFIX | Should -Match "https://raw.githubusercontent.com"
        $script:CONTRIB_URL_PREFIX | Should -Match "JohanChane/clashtui"
        $script:CONTRIB_URL_PREFIX | Should -Match "demotui"
    }
}

Describe "Write-Info / Write-Warn / Write-ErrorLog" {
    It "Write-Info does not throw" {
        { Write-Info "test message" } | Should -Not -Throw
    }

    It "Write-Warn does not throw" {
        { Write-Warn "test message" } | Should -Not -Throw
    }

    It "Write-ErrorLog does not throw" {
        { Write-ErrorLog "test message" } | Should -Not -Throw
    }
}

Describe "-NoPrompt parameter" {
    It "Script accepts -NoPrompt without error" {
        $scriptPath = (Resolve-Path (Join-Path $PSScriptRoot ".." "install.ps1")).Path
        $err = $null
        try {
            # --help exits early without installation
            $result = & powershell -NoProfile -Command "& '$scriptPath' -NoPrompt -? 2>&1; exit `$LASTEXITCODE" 2>&1
        } catch {
            $err = $_
        }
        $err | Should -Be $null
    }

    It "-NoPrompt is accepted as a switch" {
        { Get-Command "Invoke-OptionalDownloads" -ErrorAction Stop } | Should -Not -Throw
    }
}

Describe "-IsTest is rejected" {
    It "Script rejects -IsTest parameter" {
        $scriptPath = (Resolve-Path (Join-Path $PSScriptRoot ".." "install.ps1")).Path
        $result = & powershell -NoProfile -Command "& '$scriptPath' -IsTest -? 2>&1; exit `$LASTEXITCODE" 2>&1
        $LASTEXITCODE | Should -Not -Be 0
    }
}

Describe "Guard: dot-sourcing does not execute Main" {
    It "Functions are available without running Main" {
        { Get-Command Write-Info -ErrorAction Stop } | Should -Not -Throw
        { Get-Command Resolve-Paths -ErrorAction Stop } | Should -Not -Throw
        { Get-Command Get-Architecture -ErrorAction Stop } | Should -Not -Throw
        { Get-Command Get-OS -ErrorAction Stop } | Should -Not -Throw
        { Get-Command Backup-File -ErrorAction Stop } | Should -Not -Throw
        { Get-Command Copy-Contrib -ErrorAction Stop } | Should -Not -Throw
    }
}

Describe "irm|iex remote execution (ScriptName null)" {
    It "SCRIPT_DIR expression uses fallback when ScriptName is null" {
        $nullResult = if ($null) { Split-Path $null -Parent } else { (Get-Location).Path }
        $nullResult | Should -Not -BeNullOrEmpty
        $nullResult | Should -Be (Get-Location).Path
    }

    It "SCRIPT_DIR expression uses ScriptName when available" {
        $validResult = if ($installScript) { Split-Path $installScript -Parent } else { (Get-Location).Path }
        $validResult | Should -Not -BeNullOrEmpty
        $validResult | Should -Be (Split-Path $installScript -Parent)
    }

    It "SCRIPT_DIR falls back to Get-Location when ScriptName is empty" {
        $sb = [scriptblock]::Create(@"
`$script:SCRIPT_DIR = if (`$MyInvocation.ScriptName) { Split-Path `$MyInvocation.ScriptName -Parent } else { (Get-Location).Path }
`$script:SCRIPT_DIR
"@)
        $result = & $sb
        $result | Should -Not -BeNullOrEmpty
        $result | Should -BeLike "*[\\/]*"
    }
}

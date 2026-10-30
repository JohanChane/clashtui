# Run with Windows PowerShell 5.1. All service, UAC and registry writes are faked.
$ErrorActionPreference = 'Stop'
$root = Join-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) 'contrib/scripts/coresrvctrl/windows'
function Assert-True($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
function Assert-Throws([scriptblock]$Action) {
    $threw = $false
    try { & $Action | Out-Null } catch { $threw = $true }
    Assert-True $threw 'Expected failure'
}
foreach ($name in 'service.ps1','sysproxy.ps1') {
    $tokens = $null; $errors = $null
    $null = [System.Management.Automation.Language.Parser]::ParseFile((Join-Path $root $name), [ref]$tokens, [ref]$errors)
    Assert-True ($errors.Count -eq 0) ($errors | Out-String)
}
. (Join-Path $root 'service.ps1')
$nativeInvoker = (Get-Item Function:Invoke-Nssm).ScriptBlock
$temporary = Join-Path ([IO.Path]::GetTempPath()) ('clashtui script tests ' + [guid]::NewGuid())
$null = New-Item -ItemType Directory -Path $temporary
try {
    # Verify the actual native invocation, including CRT quoting, with a tiny exe.
    $script:fakeNssm = Join-Path $temporary 'fake nssm.exe'
    Add-Type -OutputAssembly $fakeNssm -OutputType ConsoleApplication -TypeDefinition @'
using System;
using System.IO;
using System.Text;
public static class FakeNssm {
    public static int Main(string[] args) {
        using (var writer = new StreamWriter(Environment.GetEnvironmentVariable("CLASHTUI_TEST_ARGUMENTS"))) {
            foreach (var arg in args) writer.WriteLine(Convert.ToBase64String(Encoding.UTF8.GetBytes(arg)));
        }
        return args.Length > 0 && args[0] == "fail" ? 37 : 0;
    }
}
'@
    $env:CLASHTUI_TEST_ARGUMENTS = Join-Path $temporary 'argv.txt'
    function Get-Command {
        param($Name, $CommandType, $ErrorAction)
        if ($Name -eq 'nssm.exe') { return [pscustomobject]@{ Source = $script:fakeNssm } }
        Microsoft.PowerShell.Core\Get-Command @PSBoundParameters
    }
    $expected = @('install','service name','C:\Program Files\core.exe','quote"value','trailing\','',"中文",'$(no-command)')
    & $nativeInvoker -Arguments $expected
    $actual = @(Get-Content $env:CLASHTUI_TEST_ARGUMENTS | ForEach-Object { [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($_)) })
    Assert-True ($actual.Count -eq $expected.Count) 'Native argument count changed'
    for ($i = 0; $i -lt $expected.Count; $i++) { Assert-True ($actual[$i] -ceq $expected[$i]) "Native argument $i changed" }
    Assert-Throws { & $nativeInvoker -Arguments @('fail') }

    # Fake only the kernel boundary; exercise the actual join/close/failure flow.
    Add-Type @'
using System;
public static class ClashTuiManagedJob {
    public static bool CanOpen = true, CanAssign = true, Member = false;
    public static int Opened, Assigned, Closed;
    public static uint Access;
    public static bool Inherit;
    public static IntPtr OpenJobObject(uint access, bool inherit, string name) {
        Opened++; Access = access; Inherit = inherit;
        return CanOpen ? new IntPtr(42) : IntPtr.Zero;
    }
    public static IntPtr GetCurrentProcess() { return new IntPtr(-1); }
    public static bool AssignProcessToJobObject(IntPtr job, IntPtr process) { Assigned++; return CanAssign; }
    public static bool IsProcessInJob(IntPtr process, IntPtr job, out bool member) { member = Member; return true; }
    public static bool CloseHandle(IntPtr handle) { Closed++; return true; }
}
'@
    Join-ManagedJob ''
    Assert-True ([ClashTuiManagedJob]::Opened -eq 0) 'Manual execution unexpectedly opens a Job'
    [ClashTuiManagedJob]::CanOpen = $false
    Assert-Throws { Join-ManagedJob 'gone-job' }
    Assert-True ([ClashTuiManagedJob]::Assigned -eq 0) 'Missing Job was recreated or assigned'
    [ClashTuiManagedJob]::CanOpen = $true; [ClashTuiManagedJob]::CanAssign = $false
    Assert-Throws { Join-ManagedJob 'denied-job' }
    Assert-True ([ClashTuiManagedJob]::Closed -eq 1) 'Failed assignment leaked Job handle'
    [ClashTuiManagedJob]::CanAssign = $true
    Join-ManagedJob 'caller-job'
    Assert-True ([ClashTuiManagedJob]::Closed -eq 2 -and [ClashTuiManagedJob]::Access -eq 5 -and -not [ClashTuiManagedJob]::Inherit) 'Successful join did not immediately close its non-inherited handle'
    [ClashTuiManagedJob]::Member = $true
    $assignments = [ClashTuiManagedJob]::Assigned
    Join-ManagedJob 'already-inherited-job'
    Assert-True ([ClashTuiManagedJob]::Assigned -eq $assignments -and [ClashTuiManagedJob]::Closed -eq 3) 'Already inherited Job was assigned again'
    [ClashTuiManagedJob]::Member = $false

    $script:Calls = New-Object 'Collections.Generic.List[object]'
    $script:NativeState = 'Stopped'
    $script:IsAdmin = $true
    $script:QueryFailure = $false
    $script:UacCancelled = $false
    $script:UacExit = 0
    $script:UacCount = 0
    function Get-CoreServiceStatus([string]$Name) {
        if ($script:QueryFailure) { throw 'Access denied' }
        return $script:NativeState
    }
    function Test-Administrator { return $script:IsAdmin }
    function Invoke-Nssm([string[]]$Arguments) {
        $script:Calls.Add($Arguments)
        switch ($Arguments[0]) {
            { $_ -in 'start','restart' } { $script:NativeState = 'Running' }
            'stop' { $script:NativeState = 'Stopped' }
            'remove' { $script:NativeState = 'missing' }
            'install' { $script:NativeState = 'Stopped' }
        }
    }
    function Start-Process {
        param($FilePath, $ArgumentList, $Verb, [switch]$Wait, [switch]$PassThru, $ErrorAction)
        $script:UacCount++
        $script:UacArguments = $ArgumentList
        Assert-True ($Verb -eq 'RunAs' -and $Wait -and $PassThru) 'Invalid elevation contract'
        if ($script:UacCancelled) { throw 'UAC cancelled' }
        $process = [pscustomobject]@{ ExitCode = $script:UacExit }
        $process | Add-Member -MemberType ScriptMethod -Name Dispose -Value {}
        return $process
    }
    $ServiceName = 'service name'
    foreach ($status in 'Running','Stopped','Paused','StartPending','StopPending','ContinuePending','PausePending','missing') {
        $NativeState = $status
        $expectedState = if ($status -eq 'Running') { 'active' } elseif ($status -eq 'missing') { 'missing' } else { 'inactive' }
        Assert-True ((Get-CoreServiceState $ServiceName) -eq $expectedState) "Wrong state for $status"
    }
    $Operation = 'status'; $NativeState = 'Stopped'; $IsAdmin = $false
    $capture = New-Object IO.StringWriter
    $original = [Console]::Out
    try { [Console]::SetOut($capture); $code = Invoke-ServiceOperation } finally { [Console]::SetOut($original) }
    Assert-True ($code -eq 0 -and $capture.ToString() -eq "inactive`r`n") 'Status protocol includes extra output'
    Assert-True ($UacCount -eq 0) 'Status requested UAC'
    $QueryFailure = $true
    Assert-Throws { Invoke-ServiceOperation }
    $QueryFailure = $false; $IsAdmin = $true

    $Operation = 'start'; $NativeState = 'Running'; $Calls.Clear()
    Assert-True ((Invoke-ServiceOperation) -eq 0 -and $Calls.Count -eq 0) 'Start restarted a running service'
    $Operation = 'stop'; $NativeState = 'missing'
    Assert-True ((Invoke-ServiceOperation) -eq 0) 'Stop missing is not idempotent'
    $Operation = 'start'
    Assert-Throws { Invoke-ServiceOperation }

    $NativeState = 'Running'; $Calls.Clear()
    foreach ($operationName in 'enable-autostart','enable-autostart','disable-autostart','disable-autostart') {
        $Operation = $operationName
        Assert-True ((Invoke-ServiceOperation) -eq 0) 'Autostart operation failed'
    }
    Assert-True ($Calls.Count -eq 4) 'Unexpected autostart calls'
    Assert-True ($Calls[0][3] -eq 'SERVICE_AUTO_START' -and $Calls[2][3] -eq 'SERVICE_DEMAND_START') 'Wrong startup types'
    Assert-True ($NativeState -eq 'Running') 'Autostart changed runtime state'

    $Operation = 'install'; $NativeState = 'missing'; $CoreType = 'singbox'
    $CoreBin = 'C:\path with space\sing-box.exe'; $ConfigDir = 'C:\中文 configs'; $ConfigPath = 'C:\中文 configs\config.json'
    $Calls.Clear()
    Assert-True ((Invoke-ServiceOperation) -eq 0) 'Install failed'
    Assert-True ($Calls[0][2] -eq $CoreBin) 'Core binary changed'
    $parameters = '"-D" "C:\中文 configs" "-c" "C:\中文 configs\config.json" "run"'
    Assert-True ($Calls[1][2] -eq 'AppParameters' -and $Calls[1][3] -ceq $parameters) 'Stored core arguments lost quoting'
    Assert-True ($Calls[2][2] -eq 'AppDirectory' -and $Calls[2][3] -eq $ConfigDir) 'Working directory missing'

    $Operation = 'restart'; $IsAdmin = $false; $NativeState = 'Running'; $Calls.Clear()
    $ManagedJob = 'Local\ClashtuiProcess-test'
    $UacExit = 37
    Assert-True ((Invoke-ServiceOperation) -eq 37 -and $Calls.Count -eq 0) 'Elevated failure did not propagate'
    Assert-True ($UacArguments.Contains('"-ManagedJob" "Local\ClashtuiProcess-test"')) 'RunAs did not forward its managed Job'
    $UacCancelled = $true
    Assert-Throws { Invoke-ServiceOperation }
    Assert-True ($Calls.Count -eq 0) 'Cancelled UAC ran a service operation'
    $Elevated = $true; $IsAdmin = $true
    [ClashTuiManagedJob]::CanOpen = $false
    Assert-Throws { Invoke-ServiceOperation }
    Assert-True ($Calls.Count -eq 0) 'Elevated process ran after its caller Job disappeared'
    [ClashTuiManagedJob]::CanOpen = $true; [ClashTuiManagedJob]::CanAssign = $false
    Assert-Throws { Invoke-ServiceOperation }
    Assert-True ($Calls.Count -eq 0) 'Elevated process ran without Job assignment'
    [ClashTuiManagedJob]::CanAssign = $true
    Assert-True ((Invoke-ServiceOperation) -eq 0 -and $Calls.Count -eq 1) 'Attached elevated operation did not run'
    $Elevated = $false; $ManagedJob = ''

    # Registry/broadcast mocks keep these tests independent of the user's proxy.
    . (Join-Path $root 'sysproxy.ps1')
    $script:Registry = @{}; $script:Broadcasts = 0
    function New-Item { param($Path, [switch]$Force) Assert-True ($Path.StartsWith('HKCU:')) 'Proxy writes outside HKCU' }
    function Set-ItemProperty { param($Path,$Name,$Value,$Type) $script:Registry[$Name] = $Value }
    function Send-ProxySettingsChanged { $script:Broadcasts++ }
    $Operation = 'enable'; $Port = 7890
    Invoke-ProxyOperation
    Assert-True ($Registry.ProxyEnable -eq 1 -and $Registry.ProxyServer -eq '127.0.0.1:7890' -and $Registry.ProxyOverride -eq '<-loopback>') 'Proxy enable changed'
    $Operation = 'disable'; $Port = 0
    Invoke-ProxyOperation
    Assert-True ($Registry.ProxyEnable -eq 0 -and $Broadcasts -eq 2) 'Offline proxy disable failed'
    $Operation = 'enable'
    Assert-Throws { Invoke-ProxyOperation }
    Write-Output 'PowerShell script tests passed (fake service/UAC/registry operations).'
} finally {
    Remove-Item Env:CLASHTUI_TEST_ARGUMENTS -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $temporary -Recurse -Force
}

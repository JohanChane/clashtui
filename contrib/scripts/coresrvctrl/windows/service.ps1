param(
    [ValidateSet('start','stop','restart','status','enable-autostart','disable-autostart','install','uninstall')]
    [string]$Operation,
    [string]$ServiceName,
    [ValidateSet('mihomo','singbox')][string]$CoreType,
    [string]$CoreBin,
    [string]$ConfigDir,
    [string]$ConfigPath,
    [string]$ManagedJob = $env:CLASHTUI_PROCESS_JOB,
    [switch]$Elevated
)

# Windows PowerShell 5.1: keep the script BOM, but never emit one on stdout.
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
$OutputEncoding = [Console]::OutputEncoding

function ConvertTo-NativeArgument([AllowEmptyString()][string]$Value) {
    # Quote for CommandLineToArgvW/CRT, including quotes and trailing backslashes.
    '"' + [regex]::Replace([regex]::Replace($Value, '(\\*)"', '$1$1\"'), '(\\+)$', '$1$1') + '"'
}

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    try {
        $principal = New-Object Security.Principal.WindowsPrincipal($identity)
        $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    } finally { $identity.Dispose() }
}

function Join-ManagedJob([string]$Name) {
    if ([string]::IsNullOrEmpty($Name)) { return }
    if (-not ('ClashTuiManagedJob' -as [type])) {
        Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ClashTuiManagedJob {
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr OpenJobObject(uint access, bool inherit, string name);
    [DllImport("kernel32.dll")]
    public static extern IntPtr GetCurrentProcess();
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool AssignProcessToJobObject(IntPtr job, IntPtr process);
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool IsProcessInJob(IntPtr process, IntPtr job, out bool member);
    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool CloseHandle(IntPtr handle);
}
'@
    }
    # Open the existing caller-owned Job; never recreate it after cancellation.
    $handle = [ClashTuiManagedJob]::OpenJobObject(5, $false, $Name) # ASSIGN_PROCESS | QUERY
    if ($handle -eq [IntPtr]::Zero) {
        throw [ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error(), 'Cannot join command job; caller ended or access was denied')
    }
    try {
        $member = $false
        $current = [ClashTuiManagedJob]::GetCurrentProcess()
        if (-not [ClashTuiManagedJob]::IsProcessInJob($current, $handle, [ref]$member)) {
            throw [ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error(), 'Cannot query elevated command Job membership')
        }
        if (-not $member -and -not [ClashTuiManagedJob]::AssignProcessToJobObject($handle, $current)) {
            throw [ComponentModel.Win32Exception]::new([Runtime.InteropServices.Marshal]::GetLastWin32Error(), 'Cannot attach elevated command to its caller job')
        }
    } finally {
        # Do not keep the Job alive after clashtui exits. If its last owner has
        # already gone, closing this handle kills this process before any writes.
        if (-not [ClashTuiManagedJob]::CloseHandle($handle)) { throw 'Cannot close command Job handle' }
    }
}

function Get-CoreServiceStatus([string]$Name) {
    # SCM distinguishes a missing service (1060) from access/query failures.
    Add-Type -AssemblyName System.ServiceProcess
    $service = New-Object System.ServiceProcess.ServiceController($Name)
    try {
        $service.Refresh()
        return [string]$service.Status
    } catch {
        $cause = $_.Exception
        while ($null -ne $cause) {
            if ($cause -is [ComponentModel.Win32Exception] -and $cause.NativeErrorCode -eq 1060) { return 'missing' }
            $cause = $cause.InnerException
        }
        throw
    } finally { $service.Dispose() }
}

function Get-CoreServiceState([string]$Name) {
    switch (Get-CoreServiceStatus $Name) {
        'Running' { return 'active' }
        'missing' { return 'missing' }
        { $_ -in 'Stopped','Paused','StartPending','StopPending','ContinuePending','PausePending' } { return 'inactive' }
        default { throw "Unrecognized service state: $_" }
    }
}

function Invoke-Nssm([string[]]$Arguments) {
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = (Get-Command nssm.exe -CommandType Application -ErrorAction Stop).Source
    $info.Arguments = ($Arguments | ForEach-Object { ConvertTo-NativeArgument $_ }) -join ' '
    $info.UseShellExecute = $false
    # Inherit the terminal; diagnostics stay visible and no pipe can deadlock.
    $process = [Diagnostics.Process]::Start($info)
    try {
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "nssm $($Arguments[0]) failed ($($process.ExitCode))" }
    } finally { $process.Dispose() }
}

function Wait-CoreServiceState([string]$Name, [string]$Expected) {
    $nativeExpected = if ($Expected -eq 'active') { 'Running' } else { 'Stopped' }
    for ($attempt = 0; $attempt -lt 50; $attempt++) {
        if ((Get-CoreServiceStatus $Name) -eq $nativeExpected) { return }
        Start-Sleep -Milliseconds 100
    }
    throw "Service did not become ${Expected}: $Name"
}

function Test-ServiceArguments {
    if ([string]::IsNullOrWhiteSpace($Operation)) { throw 'Operation is required' }
    if ([string]::IsNullOrWhiteSpace($ServiceName) -or $ServiceName -match '[/\\\x00-\x1f]' -or $ServiceName.StartsWith('-')) {
        throw 'Invalid service name'
    }
}

function Initialize-ServiceContext {
    if ($Elevated) {
        if (-not (Test-Administrator)) { throw 'Administrator privileges were not granted' }
        Join-ManagedJob $ManagedJob
    }
    $null = Get-Command nssm.exe -CommandType Application -ErrorAction Stop
}

function Test-OperationComplete([string]$State) {
    switch ($Operation) {
        'start' { return $State -eq 'active' }
        'stop' { return ($State -eq 'missing') -or ((Get-CoreServiceStatus $ServiceName) -eq 'Stopped') }
        'uninstall' { return $State -eq 'missing' }
        'install' { return $State -ne 'missing' }
        default { return $false }
    }
}

function Invoke-ElevatedOperation {
    if ($Elevated) { throw 'Administrator privileges were not granted' }
    $arguments = @('-NoProfile','-ExecutionPolicy','Bypass','-File',$PSCommandPath,'-Operation',$Operation,'-ServiceName',$ServiceName,'-Elevated')
    if ($Operation -eq 'install') {
        $arguments += @('-CoreType',$CoreType,'-CoreBin',$CoreBin,'-ConfigDir',$ConfigDir,'-ConfigPath',$ConfigPath)
    }
    if (-not [string]::IsNullOrEmpty($ManagedJob)) { $arguments += @('-ManagedJob',$ManagedJob) }
    $line = ($arguments | ForEach-Object { ConvertTo-NativeArgument $_ }) -join ' '
    # Cancellation throws; only the elevated script's actual exit code succeeds.
    $child = Start-Process -FilePath (Join-Path $PSHOME 'powershell.exe') -ArgumentList $line -Verb RunAs -Wait -PassThru -ErrorAction Stop
    try { return $child.ExitCode } finally { $child.Dispose() }
}

function Start-CoreService {
    Invoke-Nssm @('start',$ServiceName)
    Wait-CoreServiceState $ServiceName 'active'
}

function Stop-CoreService {
    Invoke-Nssm @('stop',$ServiceName)
    Wait-CoreServiceState $ServiceName 'inactive'
}

function Restart-CoreService {
    Invoke-Nssm @('restart',$ServiceName)
    Wait-CoreServiceState $ServiceName 'active'
}

function Show-CoreServiceStatus {
    [Console]::Out.WriteLine((Get-CoreServiceState $ServiceName))
}

function Enable-CoreAutostart {
    Invoke-Nssm @('set',$ServiceName,'Start','SERVICE_AUTO_START')
}

function Disable-CoreAutostart {
    Invoke-Nssm @('set',$ServiceName,'Start','SERVICE_DEMAND_START')
}

function Install-CoreService {
    if ([string]::IsNullOrEmpty($CoreType) -or -not [IO.Path]::IsPathRooted($CoreBin) -or -not [IO.Path]::IsPathRooted($ConfigDir)) {
        throw 'Install requires core type and absolute core/config paths'
    }
    if ($CoreType -eq 'mihomo') {
        $launch = @('-d',$ConfigDir)
    } else {
        if (-not [IO.Path]::IsPathRooted($ConfigPath)) { throw 'singbox requires an absolute config path' }
        $launch = @('-D',$ConfigDir,'-c',$ConfigPath,'run')
    }
    # Store one fully quoted command line: NSSM joins separate parameter
    # values with spaces, which otherwise loses grouping in core paths.
    $parameters = ($launch | ForEach-Object { ConvertTo-NativeArgument $_ }) -join ' '
    Invoke-Nssm @('install',$ServiceName,$CoreBin)
    Invoke-Nssm @('set',$ServiceName,'AppParameters',$parameters)
    Invoke-Nssm @('set',$ServiceName,'AppDirectory',$ConfigDir)
}

function Uninstall-CoreService {
    if ((Get-CoreServiceStatus $ServiceName) -ne 'Stopped') {
        Stop-CoreService
    }
    Invoke-Nssm @('remove',$ServiceName,'confirm')
}

function Main {
    Test-ServiceArguments
    Initialize-ServiceContext
    if ($Operation -eq 'status') {
        Show-CoreServiceStatus
        return 0
    }

    $state = Get-CoreServiceState $ServiceName
    if (Test-OperationComplete $state) { return 0 }
    if ($Operation -ne 'install' -and $state -eq 'missing') { throw "Service is not installed: $ServiceName" }
    if (-not (Test-Administrator)) { return Invoke-ElevatedOperation }

    switch ($Operation) {
        'start' { Start-CoreService }
        'stop' { Stop-CoreService }
        'restart' { Restart-CoreService }
        'enable-autostart' { Enable-CoreAutostart }
        'disable-autostart' { Disable-CoreAutostart }
        'install' { Install-CoreService }
        'uninstall' { Uninstall-CoreService }
    }
    return 0
}

if ($MyInvocation.InvocationName -ne '.') {
    try { exit (Main) }
    catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
}

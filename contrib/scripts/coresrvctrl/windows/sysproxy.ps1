param(
    [ValidateSet('enable','disable')][string]$Operation,
    [int]$Port
)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)

function Send-ProxySettingsChanged {
    if (-not ('ClashTuiProxySettings' -as [type])) {
        Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class ClashTuiProxySettings {
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint msg, UIntPtr wParam,
        IntPtr lParam, uint flags, uint timeout, out UIntPtr result);
    [DllImport("wininet.dll", SetLastError = true)]
    public static extern bool InternetSetOption(IntPtr handle, uint option, IntPtr buffer, uint length);
}
'@
    }
    $null = [ClashTuiProxySettings]::InternetSetOption([IntPtr]::Zero, 39, [IntPtr]::Zero, 0)
    $null = [ClashTuiProxySettings]::InternetSetOption([IntPtr]::Zero, 37, [IntPtr]::Zero, 0)
    $result = [UIntPtr]::Zero
    $null = [ClashTuiProxySettings]::SendMessageTimeout([IntPtr]0xffff, 0x001a, [UIntPtr]::Zero, [IntPtr]::Zero, 2, 5000, [ref]$result)
}

function Invoke-ProxyOperation {
    if ($Operation -notin 'enable','disable') { throw 'Operation is required' }
    if ($Operation -eq 'enable' -and ($Port -lt 1 -or $Port -gt 65535)) { throw 'Proxy port must be between 1 and 65535' }
    $path = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings'
    $null = New-Item -Path $path -Force
    if ($Operation -eq 'enable') {
        Set-ItemProperty -Path $path -Name ProxyServer -Value "127.0.0.1:$Port" -Type String
        Set-ItemProperty -Path $path -Name ProxyOverride -Value '<-loopback>' -Type String
        Set-ItemProperty -Path $path -Name ProxyEnable -Value 1 -Type DWord
    } else {
        Set-ItemProperty -Path $path -Name ProxyEnable -Value 0 -Type DWord
    }
    Send-ProxySettingsChanged
}

if ($MyInvocation.InvocationName -ne '.') {
    try { Invoke-ProxyOperation; exit 0 }
    catch { [Console]::Error.WriteLine($_.Exception.Message); exit 1 }
}

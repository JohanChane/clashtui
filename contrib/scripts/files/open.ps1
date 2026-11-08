# Open a literal file or directory path with its associated application.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][ValidateNotNullOrEmpty()][string]$Path)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false)
try {
    if (-not [System.IO.Path]::IsPathRooted($Path)) {
        throw 'open.ps1: Path must be absolute'
    }
    Invoke-Item -LiteralPath $Path -ErrorAction Stop
} catch {
    [Console]::Error.WriteLine($_.Exception.Message)
    exit 1
}

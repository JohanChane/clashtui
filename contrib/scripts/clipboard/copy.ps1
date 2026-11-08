# Read the entire UTF-8 input without adding a trailing newline.
param([ValidateSet('auto', 'native', 'osc52')][string]$Backend = 'auto')
$ErrorActionPreference = 'Stop'
[Console]::InputEncoding = New-Object System.Text.UTF8Encoding
try {
    if ($Backend -eq 'auto') {
        if ($env:SSH_TTY -or $env:SSH_CONNECTION -or $env:SSH_CLIENT) {
            $Backend = 'osc52'
        } else {
            $Backend = 'native'
        }
    }
    $text = [Console]::In.ReadToEnd()
    if ($Backend -eq 'osc52') {
        $encoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($text))
        $sequence = [string][char]27 + ']52;c;' + $encoded + [char]7
        $bytes = [Text.Encoding]::ASCII.GetBytes($sequence)
        $stdout = [Console]::OpenStandardOutput()
        $stdout.Write($bytes, 0, $bytes.Length)
        $stdout.Flush()
    } else {
        Set-Clipboard -Value $text
    }
} catch {
    Write-Error $_
    exit 1
}

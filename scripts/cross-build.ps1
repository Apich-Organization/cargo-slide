<#
.SYNOPSIS
    Convenience wrapper script for cargo-slide's built-in cross command (PowerShell / Windows).

.DESCRIPTION
    NOTE: Cross-compilation is now natively built directly into cargo-slide!
    You can directly run:
        cargo slide cross --target all
        cargo slide cross --target windows-x86_64
    without downloading or needing any external script files.
#>
[CmdletBinding()]
param (
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$ArgsList
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$RepoRoot = (Get-Item "$ScriptDir\..").FullName
Set-Location $RepoRoot

if (Get-Command "cargo-slide" -ErrorAction SilentlyContinue) {
    & cargo slide cross @ArgsList
} else {
    & cargo run -p cargo-slide -- cross @ArgsList
}
exit $LASTEXITCODE

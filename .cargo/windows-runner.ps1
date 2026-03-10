param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Command
)

$gstBin = $env:GSTREAMER_BIN_DIR
if ($gstBin -and (Test-Path $gstBin)) {
    $env:Path = "$gstBin;$env:Path"
}

if ($Command.Length -eq 0) {
    Write-Error "windows-runner.ps1 expected a command to execute"
    exit 1
}

$exe = $Command[0]
$args = @()
if ($Command.Length -gt 1) {
    $args = $Command[1..($Command.Length - 1)]
}

& $exe @args
exit $LASTEXITCODE

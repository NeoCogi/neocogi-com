# Use the same explicit DSO, import-library and test-discovery policy as Unix/CI.
$ErrorActionPreference = "Stop"
& python (Join-Path $PSScriptRoot "compatibility.py") --suite linked @args
exit $LASTEXITCODE

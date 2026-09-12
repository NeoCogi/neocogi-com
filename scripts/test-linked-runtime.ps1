$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "../../..")

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    throw "rustup and the stable and nightly Rust toolchains are required"
}
& rustup run nightly cargo --version | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "the nightly Rust toolchain is required"
}

$targetDir = if ($env:CARGO_TARGET_DIR) {
    $env:CARGO_TARGET_DIR
} else {
    "target/linked-runtime"
}

# The runtime package is normally an rlib consumed by cargo-neocogi's generated
# root. This direct-link fixture supplies that one DLL root itself.
& cargo rustc -p neocogi-com-runtime --lib --crate-type cdylib --locked --target-dir $targetDir
if ($LASTEXITCODE -ne 0) {
    throw "failed to build neocogi-com-runtime"
}

$runtimeDir = [IO.Path]::GetFullPath((Join-Path $targetDir "debug"))
$runtimeDepsDir = Join-Path $runtimeDir "deps"

# rustc's MSVC cdylib output may use the `.dll.lib` suffix while a native
# `#[link(name = "nc_com_shared_runtime")]` lookup requests `nc_com_shared_runtime.lib`.
$importLibraries = Get-ChildItem -Path @($runtimeDir, $runtimeDepsDir) `
    -Filter "nc_com_shared_runtime.dll.lib" -ErrorAction SilentlyContinue
foreach ($importLibrary in $importLibraries) {
    Copy-Item $importLibrary.FullName `
        (Join-Path $importLibrary.DirectoryName "nc_com_shared_runtime.lib") -Force
}
if (-not (Test-Path (Join-Path $runtimeDir "nc_com_shared_runtime.lib")) -and
    -not (Test-Path (Join-Path $runtimeDepsDir "nc_com_shared_runtime.lib"))) {
    throw "neocogi-com-runtime import library was not produced"
}

$nativeFlags = "-Lnative=$runtimeDir -Lnative=$runtimeDepsDir"
$env:RUSTFLAGS = if ($env:RUSTFLAGS) {
    "$($env:RUSTFLAGS) $nativeFlags"
} else {
    $nativeFlags
}
$env:NC_COM_RUNTIME_LINK_DIR = $runtimeDir
$env:PATH = "$runtimeDir;$runtimeDepsDir;$($env:PATH)"

$listed = (& cargo test -p neocogi-com --features runtime --locked `
    --target-dir $targetDir -- --list 2>&1 | Out-String)
if ($LASTEXITCODE -ne 0) {
    throw "failed to enumerate linked-runtime tests`n$listed"
}
Write-Host $listed

$requiredTests = @(
    "generated_contract_factory_activates_without_a_class_table",
    "dynamic_factory_plugin_links_the_exact_contract_dso"
)
foreach ($requiredTest in $requiredTests) {
    if (-not $listed.Contains("${requiredTest}: test")) {
        throw "linked-runtime test was not discovered: $requiredTest"
    }
}

& cargo test -p neocogi-com --features runtime --locked --target-dir $targetDir
if ($LASTEXITCODE -ne 0) {
    throw "linked-runtime test suite failed"
}

Write-Host "linked-runtime test suite passed"

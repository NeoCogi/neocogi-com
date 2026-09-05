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

& cargo build -p nc-com-runtime --locked --target-dir $targetDir
if ($LASTEXITCODE -ne 0) {
    throw "failed to build nc-com-runtime"
}

$runtimeDir = [IO.Path]::GetFullPath((Join-Path $targetDir "debug"))
$runtimeDepsDir = Join-Path $runtimeDir "deps"

# rustc's MSVC cdylib output may use the `.dll.lib` suffix while a native
# `#[link(name = "nc_com_runtime")]` lookup requests `nc_com_runtime.lib`.
$importLibraries = Get-ChildItem -Path @($runtimeDir, $runtimeDepsDir) `
    -Filter "nc_com_runtime.dll.lib" -ErrorAction SilentlyContinue
foreach ($importLibrary in $importLibraries) {
    Copy-Item $importLibrary.FullName `
        (Join-Path $importLibrary.DirectoryName "nc_com_runtime.lib") -Force
}
if (-not (Test-Path (Join-Path $runtimeDir "nc_com_runtime.lib")) -and
    -not (Test-Path (Join-Path $runtimeDepsDir "nc_com_runtime.lib"))) {
    throw "nc-com-runtime import library was not produced"
}

$nativeFlags = "-Lnative=$runtimeDir -Lnative=$runtimeDepsDir"
$env:RUSTFLAGS = if ($env:RUSTFLAGS) {
    "$($env:RUSTFLAGS) $nativeFlags"
} else {
    $nativeFlags
}
$env:NC_COM_RUNTIME_LINK_DIR = $runtimeDir
$env:PATH = "$runtimeDir;$runtimeDepsDir;$($env:PATH)"

$listed = (& cargo test -p nc-com --features runtime --locked `
    --target-dir $targetDir -- --list 2>&1 | Out-String)
if ($LASTEXITCODE -ne 0) {
    throw "failed to enumerate linked-runtime tests`n$listed"
}
Write-Host $listed

$requiredTests = @(
    "runtime_is_shareable_as_process_wide_state",
    "runtime_exposes_validated_descriptor_without_registration",
    "runtime_loads_shared_library_and_constructs_class",
    "contract_crate_workflow_example_passes",
    "stable_debug_producer_component_is_abi_compatible"
)
foreach ($requiredTest in $requiredTests) {
    if (-not $listed.Contains("${requiredTest}: test")) {
        throw "linked-runtime test was not discovered: $requiredTest"
    }
}

& cargo test -p nc-com --features runtime --locked --target-dir $targetDir
if ($LASTEXITCODE -ne 0) {
    throw "linked-runtime test suite failed"
}

Write-Host "linked-runtime test suite passed"

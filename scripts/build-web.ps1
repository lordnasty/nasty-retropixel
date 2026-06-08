$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
  throw "rustup non trovato. Installa Rust (https://rustup.rs) e riprova."
}

rustup target add wasm32-unknown-unknown | Out-Host

cargo build --lib --target wasm32-unknown-unknown --release | Out-Host

$toolsDir = Join-Path $root ".tools"
$wasmBindgen = Join-Path $toolsDir "bin\wasm-bindgen.exe"

if (-not (Test-Path $wasmBindgen)) {
  cargo install wasm-bindgen-cli --version 0.2.105 --root $toolsDir --locked | Out-Host
}

$pkgDir = Join-Path $root "web\pkg"
New-Item -ItemType Directory -Force -Path $pkgDir | Out-Null

$wasmPath = Join-Path $root "target\wasm32-unknown-unknown\release\nasty_retropixel.wasm"
if (-not (Test-Path $wasmPath)) {
  throw "WASM non trovato: $wasmPath"
}

& $wasmBindgen $wasmPath --target web --out-dir $pkgDir --out-name nasty_retropixel --typescript | Out-Host

Write-Host "OK: web/pkg aggiornato."

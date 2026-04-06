$ErrorActionPreference = "Stop"

# Always resolve paths from the project root (where package.json lives),
# regardless of which directory npm was invoked from.
$ProjectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $ProjectRoot

function Import-EnvFile {
  param([string]$FilePath)
  $resolved = Join-Path $ProjectRoot $FilePath
  if (-not (Test-Path -LiteralPath $resolved)) { return }

  foreach ($line in Get-Content -LiteralPath $resolved) {
    $trimmed = $line.Trim()
    if (-not $trimmed -or $trimmed.StartsWith("#")) { continue }
    $parts = $trimmed -split "=", 2
    if ($parts.Count -ne 2) { continue }
    $key = $parts[0].Trim()
    $value = $parts[1].Trim().Trim('"').Trim("'")
    if (-not $key) { continue }
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($key))) {
      [Environment]::SetEnvironmentVariable($key, $value, "Process")
    }
  }
}

function Require-Env {
  param([string]$Name)
  $val = [Environment]::GetEnvironmentVariable($Name)
  if ([string]::IsNullOrWhiteSpace($val)) {
    Write-Error "[start:rust] ERROR: Required env var '$Name' is not set. Aborting."
    exit 1
  }
}

# Load env files — production priority order
Import-EnvFile ".env.production.local"
Import-EnvFile ".env.production"
Import-EnvFile ".env.local"
Import-EnvFile ".env"
Import-EnvFile "env"

# Require critical production secrets — never fall back to defaults
Require-Env "JWT_SECRET"
Require-Env "REFRESH_TOKEN_SECRET"

$binary = Join-Path $ProjectRoot "rust\src\target\release\reverz-session-api.exe"

if (-not (Test-Path -LiteralPath $binary)) {
  Write-Error "[start:rust] ERROR: Binary not found at '$binary'. Run 'npm run build:rust' first."
  exit 1
}

Write-Host "[start:rust] Starting production Rust API..."
Write-Host "[start:rust] Binary: $binary"
& $binary

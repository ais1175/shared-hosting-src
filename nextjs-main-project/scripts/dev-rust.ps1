$ErrorActionPreference = "Stop"

function Import-EnvFile {
  param([string]$FilePath)
  if (-not (Test-Path -LiteralPath $FilePath)) { return }

  foreach ($line in Get-Content -LiteralPath $FilePath) {
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

function Set-DefaultEnv {
  param([string]$Name, [string]$Value)
  if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($Name))) {
    [Environment]::SetEnvironmentVariable($Name, $Value, "Process")
  }
}

Import-EnvFile ".env.local"
Import-EnvFile ".env"
Import-EnvFile "env"
Import-EnvFile "..\\env"

Set-DefaultEnv "TRUEMONEY_RECEIVER_PHONE" "0931959423"
Set-DefaultEnv "TRUEMONEY_TIMEOUT_MS" "10000"
Set-DefaultEnv "ACCESS_TOKEN_TTL_SECONDS" "86400"
Set-DefaultEnv "REFRESH_TOKEN_TTL_SECONDS" "2592000"
Set-DefaultEnv "TOPUP_RATE_LIMIT_PER_MINUTE" "5"
Set-DefaultEnv "COOKIE_SECURE" "false"
Set-DefaultEnv "ADMIN_USERNAME" "root"
Set-DefaultEnv "ADMIN_PASSWORD" "root"
Set-DefaultEnv "DA_URL" "https://dcadmin.reverz.in.th"
Set-DefaultEnv "DA_USERNAME" "reverzinth"
Set-DefaultEnv "DA_PASSWORD" "GV886700"

# Force fixed token secrets in dev to avoid INVALID_ACCESS_TOKEN from secret drift.
[Environment]::SetEnvironmentVariable("JWT_SECRET", "reverz-dev-jwt-secret-fixed-v1", "Process")
[Environment]::SetEnvironmentVariable("REFRESH_TOKEN_SECRET", "reverz-dev-refresh-secret-fixed-v1", "Process")

Write-Host "[dev:rust] TRUEMONEY_RECEIVER_PHONE=$env:TRUEMONEY_RECEIVER_PHONE"
Write-Host "[dev:rust] JWT/Refresh secrets fixed for this process"
cargo run --manifest-path rust/src/Cargo.toml

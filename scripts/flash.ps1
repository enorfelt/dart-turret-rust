#Requires -Version 7
param(
    [string]$Port,
    [ValidateSet('debug', 'release')]
    [string]$BuildProfile = 'debug',
    [string]$Firmware = 'turret',
    [string]$Target = 'xtensa-esp32s3-espidf'
)

$ErrorActionPreference = 'Stop'

$elf = Join-Path $PSScriptRoot "..\firmware\$Firmware\target\$Target\$BuildProfile\$Firmware"
if (-not (Test-Path $elf)) {
    throw "No firmware image at $elf. Build it in WSL first: cd firmware/$Firmware && cargo build$(if ($BuildProfile -eq 'release') { ' --release' })"
}

$espflashArgs = @('flash', '--monitor')
if ($Port) { $espflashArgs += @('--port', $Port) }
$espflashArgs += (Resolve-Path $elf).ProviderPath

& espflash @espflashArgs
exit $LASTEXITCODE

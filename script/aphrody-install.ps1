$ErrorActionPreference = "Stop"

$Channel = if ($env:APHRODY_CHANNEL) { $env:APHRODY_CHANNEL } elseif ($env:ZED_CHANNEL) { $env:ZED_CHANNEL } else { "stable" }
$Version = if ($env:APHRODY_VERSION) { $env:APHRODY_VERSION } elseif ($env:ZED_VERSION) { $env:ZED_VERSION } else { "latest" }
$BaseUrl = if ($env:APHRODY_RELEASES_URL) { $env:APHRODY_RELEASES_URL } else { "https://releases.aphrody.com" }

$Arch = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq [System.Runtime.InteropServices.Architecture]::Arm64) { "arm64" } else { "x64" }
$Asset = "Aphrody-Setup-$Arch.exe"
$Url = "$BaseUrl/assets/$Channel/$Version/$Asset"
$OutFile = Join-Path $env:TEMP $Asset

Write-Host "Downloading Aphrody ($Channel/$Version) from $Url..."
Invoke-WebRequest -Uri $Url -OutFile $OutFile -UseBasicParsing
Write-Host "Downloaded installer to $OutFile — launching silent setup..."
Start-Process -FilePath $OutFile -ArgumentList "/S" -Wait

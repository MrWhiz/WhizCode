# Rejoin split installer parts back into a single executable
# Usage: .\join-installer.ps1

$outputFile = "WhizCode Setup 0.1.0.exe"
$parts = Get-ChildItem -Filter "WhizCode_Part_*.bin" | Sort-Object { [int]($_.BaseName -split '_')[2] }

if ($parts.Count -eq 0) {
    Write-Error "No split files found. Looking for WhizCode_Part_*.bin files"
    exit 1
}

Write-Host "Found $($parts.Count) parts. Joining..."
Write-Host ""

$outputStream = [System.IO.File]::Create($outputFile)

foreach ($part in $parts) {
    Write-Host "Adding: $($part.Name) ($([Math]::Round($part.Length / 1MB, 2)) MB)"
    $fileStream = [System.IO.File]::OpenRead($part.FullName)
    $fileStream.CopyTo($outputStream)
    $fileStream.Close()
}

$outputStream.Close()

$finalSize = (Get-Item $outputFile).Length
Write-Host ""
Write-Host "Joined complete!"
Write-Host "Output: $outputFile ($([Math]::Round($finalSize / 1MB, 2)) MB)"
Write-Host ""
Write-Host "You can now run the installer:"
Write-Host "  .\$outputFile"

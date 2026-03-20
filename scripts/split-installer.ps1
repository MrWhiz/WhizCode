# Split the installer into 20MB chunks for email distribution
# Usage: .\split-installer.ps1

$installerPath = "release/0.1.0/WhizCode Setup 0.1.0.exe"
$chunkSize = 20MB  # 20 megabytes
$outputDir = "release/0.1.0/split"

if (-not (Test-Path $installerPath)) {
    Write-Error "Installer not found: $installerPath"
    exit 1
}

# Create output directory
New-Item -ItemType Directory -Path $outputDir -Force | Out-Null

# Get file info
$file = Get-Item $installerPath
$totalSize = $file.Length
$totalChunks = [Math]::Ceiling($totalSize / $chunkSize)

Write-Host "Splitting $($file.Name) ($([Math]::Round($totalSize / 1MB, 2)) MB) into $totalChunks chunks of 20MB each..."
Write-Host ""

# Read file and split
$fileStream = [System.IO.File]::OpenRead($installerPath)
$buffer = New-Object byte[] $chunkSize

for ($i = 0; $i -lt $totalChunks; $i++) {
    $bytesRead = $fileStream.Read($buffer, 0, $chunkSize)
    $chunkPath = Join-Path $outputDir "WhizCode_Part_$($i + 1)_of_$totalChunks.bin"
    
    # Write only the bytes that were read
    [System.IO.File]::WriteAllBytes($chunkPath, $buffer[0..($bytesRead - 1)])
    
    $chunkSize_MB = [Math]::Round($bytesRead / 1MB, 2)
    Write-Host "Created: WhizCode_Part_$($i + 1)_of_$totalChunks.bin ($chunkSize_MB MB)"
}

$fileStream.Close()

Write-Host ""
Write-Host "Split complete! Files are in: $outputDir"
Write-Host ""
Write-Host "To reassemble on the receiving end, use:"
Write-Host "  .\join-installer.ps1"
Write-Host ""
Write-Host "Or manually with:"
Write-Host "  copy /b WhizCode_Part_1_of_5.bin + WhizCode_Part_2_of_5.bin + ... WhizCode_Setup.exe"

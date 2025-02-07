$exe_url = "https://github.com/TheBearodactyl/globalcaps-rs/releases/download/1/globalcaps.exe"
$exe_name = "globalcaps.exe"

$destination_dir = $env:Path -split ';' |
  Where-Object { $_ -ne '' } |
  Select-Object -Unique |
  Where-Object {
    try
    {
      $test_path = Join-Path $_ "testfile.tmp"
      [System.IO.File]::WriteAllText($test_path, "test")
      Remove-Item $test_path -ErrorAction Stop
      $true
    } catch
    { $false 
    }
  } | Select-Object -First 1

if (-not $destination_dir)
{
  Write-Host 'Error: No writable directory found in $PATH' -ForegroundColor Red
  Write-Host "Consider creating a directory like '$env:USERPROFILE\bin' and adding it to your PATH" -ForegroundColor Yellow
  exit 1
}

try
{
  $temp_file = Join-Path $env:TEMP $exe_name
  Invoke-WebRequest -Uri $exe_url -OutFile $temp_file -ErrorAction Stop
  Write-Host "File downloaded successfully" -ForegroundColor Green
} catch
{
  Write-Host "Error downloading file: $_" -ForegroundColor Red
  exit 1
}

try
{
  $destination_path = Join-Path $destination_dir $exe_name
  Copy-Item -Path $temp_file -Destination $destination_path -ErrorAction Stop
  Write-Host "File copied to: $destination_path" -ForegroundColor Green
} catch
{
  Write-Host "Error copying file: $_" -ForegroundColor Red
  exit 1
} finally
{
  if (Test-Path $temp_file)
  { Remove-Item $temp_file 
  }
}

Write-Host "Verification:"
Write-Host "File placed in directory: $destination_dir"
Write-Host "Which is in your PATH environment variable"

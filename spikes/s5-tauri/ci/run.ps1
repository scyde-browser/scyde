# Runs the spike exe in one mode, screenshots the desktop while it is up, waits for it to exit.
param(
  [Parameter(Mandatory)] [string] $Exe,
  [Parameter(Mandatory)] [string] $Name,
  [string[]] $AppArgs = @(),
  [int] $ShotAfter = 8,
  [int] $Timeout = 600
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
$p = Start-Process -FilePath $Exe -ArgumentList $AppArgs -PassThru
Start-Sleep -Seconds $ShotAfter
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($b.Location, [System.Drawing.Point]::Empty, $b.Size)
$bmp.Save("$Name.png", [System.Drawing.Imaging.ImageFormat]::Png)
if (-not $p.WaitForExit($Timeout * 1000)) { $p.Kill(); throw "$Name timed out after $Timeout s" }
if (Test-Path "$Name.json") { Get-Content "$Name.json" } else { throw "$Name.json was not written" }

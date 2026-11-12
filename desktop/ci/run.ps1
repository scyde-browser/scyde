# Starts SCYDE, screenshots the desktop while it runs, waits for it to exit (from spike S5).
param(
  [Parameter(Mandatory)] [string] $Exe,
  [Parameter(Mandatory)] [string] $Name,
  [string[]] $AppArgs = @(),
  [int] $ShotAfter = 10,
  [int] $Timeout = 120
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
$p = Start-Process -FilePath $Exe -ArgumentList $AppArgs -PassThru
Start-Sleep -Seconds $ShotAfter
$b = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap $b.Width, $b.Height
$g = [System.Drawing.Graphics]::FromImage($bmp)
# BitBlt with SRCCOPY | CAPTUREBLT includes layered (transparent) windows such as the floating
# island; Graphics.CopyFromScreen rejects that flag combination on .NET.
Add-Type -Namespace Scyde -Name Gdi -MemberDefinition @"
[DllImport("gdi32.dll")] public static extern bool BitBlt(IntPtr hdcDest, int x, int y, int w, int h, IntPtr hdcSrc, int sx, int sy, uint rop);
[DllImport("user32.dll")] public static extern IntPtr GetDC(IntPtr hwnd);
[DllImport("user32.dll")] public static extern int ReleaseDC(IntPtr hwnd, IntPtr hdc);
"@
$screen = [Scyde.Gdi]::GetDC([IntPtr]::Zero)
$dest = $g.GetHdc()
[void][Scyde.Gdi]::BitBlt($dest, 0, 0, $b.Width, $b.Height, $screen, $b.X, $b.Y, 0x40CC0020)
$g.ReleaseHdc($dest)
[void][Scyde.Gdi]::ReleaseDC([IntPtr]::Zero, $screen)
$bmp.Save("$Name.png", [System.Drawing.Imaging.ImageFormat]::Png)
if (-not $p.WaitForExit($Timeout * 1000)) { $p.Kill(); throw "$Name timed out after $Timeout s" }
if (Test-Path "$Name.json") { Get-Content "$Name.json" } else { throw "$Name.json was not written" }

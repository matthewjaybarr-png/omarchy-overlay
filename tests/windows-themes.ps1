# Starts Omarchy Overlay under several themes and saves a screenshot of the menu for each.
# A theme folder pushed to config\themes\<id>\ beside the exe must load without a config error.
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
$local=Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'OmarchyOverlay'
$configPath=Join-Path $local 'config.json'
$logPath=Join-Path $local 'omarchy-overlay.log'
$originalConfig=if(Test-Path $configPath){[IO.File]::ReadAllText($configPath)}else{$null}
$results=[Collections.Generic.List[object]]::new()
Add-Type -AssemblyName System.Windows.Forms,System.Drawing
Add-Type -TypeDefinition @'
using System; using System.Runtime.InteropServices;
public static class ThemeTest {
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string cls,string title);
 public static IntPtr ByClass(string cls){return FindWindow(cls,null);}  // PowerShell turns $null into "" for string parameters
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr v);
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
}
'@
function Check($name,$passed,$detail=''){$results.Add([pscustomobject]@{name=$name;passed=[bool]$passed;detail="$detail"})}
function Shot($file){$b=[Windows.Forms.Screen]::PrimaryScreen.Bounds;$bmp=[Drawing.Bitmap]::new($b.Width,$b.Height);$g=[Drawing.Graphics]::FromImage($bmp);try{$g.CopyFromScreen($b.X,$b.Y,0,0,$bmp.Size);$bmp.Save((Join-Path $root $file),[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$bmp.Dispose()}}
try{
 [void][ThemeTest]::SetProcessDpiAwarenessContext([IntPtr](-4))  # full-resolution screenshots on scaled displays
 if(Get-Process OmarchyOverlay -ErrorAction SilentlyContinue){throw 'A controller is already running.'}
 Check 'Neon Ridge theme folder is present beside the exe' (Test-Path (Join-Path $root 'config\themes\neon-ridge\colors.toml'))
 foreach($theme in 'tokyo-night','catppuccin-latte','neon-ridge'){
  New-Item -ItemType Directory -Force $local|Out-Null
  [IO.File]::WriteAllText($configPath,(@{theme=$theme;start_tiling=$false}|ConvertTo-Json),[Text.UTF8Encoding]::new($false))
  $logStart=if(Test-Path $logPath){(Get-Item $logPath).Length}else{0}
  $app=Start-Process (Join-Path $root 'OmarchyOverlay.exe') -PassThru
  $until=[DateTime]::UtcNow.AddSeconds(15)
  do{Start-Sleep -Milliseconds 250;$menu=[ThemeTest]::ByClass('OmarchyOverlay.Overlay')}while(-not ($menu -ne [IntPtr]::Zero -and [ThemeTest]::IsWindowVisible($menu)) -and [DateTime]::UtcNow -lt $until)
  Start-Sleep -Milliseconds 800
  Check "Menu opens under $theme" ($menu -ne [IntPtr]::Zero -and [ThemeTest]::IsWindowVisible($menu))
  Shot "theme-$theme.png"
  $saved=(Get-Content $configPath -Raw|ConvertFrom-Json).theme
  Check "$theme is still the saved theme (not a fallback)" ($saved -eq $theme) $saved
  $log=if(Test-Path $logPath){$s=[IO.File]::ReadAllText($logPath);$s.Substring([Math]::Min($logStart,$s.Length))}else{''}
  Check "No theme config error under $theme" (-not ($log -match 'themes\\')) $log
  $main=[ThemeTest]::ByClass('OmarchyOverlay.Main')
  [void][ThemeTest]::PostMessage($main,0x312,[IntPtr]105,[IntPtr]::Zero)  # the exit hotkey
  Check "Controller exits under $theme" ($app.WaitForExit(15000))
 }
}catch{Check 'Fixture completed' $false ($_|Out-String)}
finally{
 Get-Process OmarchyOverlay -ErrorAction SilentlyContinue|Stop-Process -Force
 Remove-Item -Recurse -Force (Join-Path $root 'config\themes\neon-ridge') -ErrorAction SilentlyContinue
 if($null -ne $originalConfig){[IO.File]::WriteAllText($configPath,$originalConfig,[Text.UTF8Encoding]::new($false))}elseif(Test-Path $configPath){Remove-Item $configPath}
 $results|ConvertTo-Json -Depth 4|Set-Content -Encoding UTF8 (Join-Path $root 'themes-result.json')
}

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$desktop = [Environment]::GetFolderPath('Desktop')
$destination = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'OmarchyOverlay\app'
New-Item -ItemType Directory -Force $destination | Out-Null
Copy-Item (Join-Path $root 'OmarchyOverlay.exe') (Join-Path $destination 'OmarchyOverlay.exe') -Force
Copy-Item (Join-Path $root 'README.md') (Join-Path $destination 'README.md') -Force
$link = (New-Object -ComObject WScript.Shell).CreateShortcut((Join-Path $desktop 'Omarchy Overlay.lnk'))
$link.TargetPath = Join-Path $destination 'OmarchyOverlay.exe'
$link.WorkingDirectory = $destination
$link.IconLocation = (Join-Path $destination 'OmarchyOverlay.exe') + ',0'
$link.Description = 'Omarchy Overlay: keyboard-first tiling and app launcher. Starts paused.'
$link.Save()
Start-Process (Join-Path $destination 'OmarchyOverlay.exe')
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Preview {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int left,top,right,bottom; }
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string name, string title);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr hwnd,int id);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hwnd,uint message,IntPtr w,IntPtr l);
  public static IntPtr Controls() { return FindWindow("OmarchyOverlay.Main", null); }
  public static IntPtr Bar() { return FindWindow("OmarchyOverlay.TopBar", null); }
  public static IntPtr Appearance() { return FindWindow("OmarchyOverlay.Appearance", null); }
  public static IntPtr Settings() { return FindWindow("OmarchyOverlay.Settings", null); }
}
'@
[void][Preview]::SetProcessDPIAware()
$deadline = [DateTime]::UtcNow.AddSeconds(20)
do {
    Start-Sleep -Milliseconds 250
    $window = [Preview]::Controls()
} while ($window -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
if ($window -eq [IntPtr]::Zero) { throw 'Keyspace controls did not open.' }
[void][Preview]::SetForegroundWindow($window)
Start-Sleep -Milliseconds 1200
$r = [Preview+RECT]::new()
[void][Preview]::GetWindowRect($window, [ref]$r)
$bitmap = [System.Drawing.Bitmap]::new(($r.right-$r.left), ($r.bottom-$r.top))
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
try {
    $graphics.CopyFromScreen($r.left,$r.top,0,0,$bitmap.Size)
    $bitmap.Save((Join-Path $root 'keyspace-controls.png'), [System.Drawing.Imaging.ImageFormat]::Png)
} finally {
    $graphics.Dispose()
    $bitmap.Dispose()
}
$bar = [Preview]::Bar()
if ($bar -ne [IntPtr]::Zero) {
    $r = [Preview+RECT]::new()
    [void][Preview]::GetWindowRect($bar, [ref]$r)
    $bitmap = [System.Drawing.Bitmap]::new(($r.right-$r.left), ($r.bottom-$r.top))
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($r.left,$r.top,0,0,$bitmap.Size)
        $bitmap.Save((Join-Path $root 'keyspace-topbar.png'), [System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
}
[void][Preview]::PostMessage($window,0x0111,[IntPtr]8,[IntPtr]::Zero)
Start-Sleep -Milliseconds 700
$appearance = [Preview]::Appearance()
if ($appearance -ne [IntPtr]::Zero) {
    $r = [Preview+RECT]::new()
    [void][Preview]::GetWindowRect($appearance, [ref]$r)
    $bitmap = [System.Drawing.Bitmap]::new(($r.right-$r.left), ($r.bottom-$r.top))
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($r.left,$r.top,0,0,$bitmap.Size)
        $bitmap.Save((Join-Path $root 'keyspace-appearance.png'), [System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
}
[void][Preview]::PostMessage($window,0x0111,[IntPtr]3,[IntPtr]::Zero)
Start-Sleep -Milliseconds 700
$settings=[Preview]::Settings()
if($settings -ne [IntPtr]::Zero) {
    [void][Preview]::PostMessage($settings,0x0111,[IntPtr]54,[IntPtr]::Zero)
    Start-Sleep -Milliseconds 650
    $list=[Preview]::GetDlgItem($settings,42)
    [void][Preview]::SendMessage($list,0x0186,[IntPtr]::Zero,[IntPtr]::Zero)
    [void][Preview]::PostMessage($settings,0x0111,[IntPtr](42+(1 -shl 16)),$list)
    Start-Sleep -Milliseconds 250
    $r=[Preview+RECT]::new()
    [void][Preview]::GetWindowRect($settings,[ref]$r)
    $bitmap=[System.Drawing.Bitmap]::new(($r.right-$r.left),($r.bottom-$r.top))
    $graphics=[System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($r.left,$r.top,0,0,$bitmap.Size)
        $bitmap.Save((Join-Path $root 'keyspace-settings.png'),[System.Drawing.Imaging.ImageFormat]::Png)
    } finally {$graphics.Dispose();$bitmap.Dispose()}
}
"Ready: desktop shortcut created, Omarchy Overlay paused, Settings open."

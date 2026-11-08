$ErrorActionPreference='Stop'
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
$app=Start-Process (Join-Path $root 'portable-a\OmarchyOverlay.exe') -PassThru
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
  public static IntPtr Profiles() { return FindWindow("OmarchyOverlay.Profiles", null); }
  public static IntPtr Settings() { return FindWindow("OmarchyOverlay.Settings", null); }
}
'@
[void][Preview]::SetProcessDPIAware()
try {
 Start-Sleep -Seconds 2
 [void][Preview]::PostMessage([Preview]::Controls(),0x111,[IntPtr]3,[IntPtr]::Zero)
 Start-Sleep -Milliseconds 500
 [void][Preview]::PostMessage([Preview]::Settings(),0x111,[IntPtr]68,[IntPtr]::Zero)
 Start-Sleep -Milliseconds 900
 $window=[Preview]::Profiles()
 [void][Preview]::SetForegroundWindow($window)
 Start-Sleep -Milliseconds 500
 $r=[Preview+RECT]::new()
 [void][Preview]::GetWindowRect($window,[ref]$r)
 $bitmap=[Drawing.Bitmap]::new(($r.right-$r.left),($r.bottom-$r.top))
 $graphics=[Drawing.Graphics]::FromImage($bitmap)
 try {$graphics.CopyFromScreen($r.left,$r.top,0,0,$bitmap.Size);$bitmap.Save((Join-Path $root 'keyspace-profiles.png'),[Drawing.Imaging.ImageFormat]::Png)} finally {$graphics.Dispose();$bitmap.Dispose()}
} finally {
 [void][Preview]::PostMessage([Preview]::Controls(),0x312,[IntPtr]105,[IntPtr]::Zero)
 [void]$app.WaitForExit(8000)
}

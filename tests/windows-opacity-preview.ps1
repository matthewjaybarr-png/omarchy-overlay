$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class OpacityPreview {
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr context);
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string name,string title);
  [DllImport("user32.dll")] static extern bool PostMessage(IntPtr hwnd,uint message,IntPtr w,IntPtr l);
  public static void ShowAppearance() { PostMessage(FindWindow("OmarchyOverlay.Main",null),0x0111,(IntPtr)8,IntPtr.Zero); }
  [DllImport("user32.dll")] public static extern void keybd_event(byte key,byte scan,uint flags,UIntPtr extra);
  public static void Fade() {
    keybd_event(0x11,0,0,UIntPtr.Zero); keybd_event(0x12,0,0,UIntPtr.Zero);
    keybd_event(0xbc,0,0,UIntPtr.Zero); keybd_event(0xbc,0,2,UIntPtr.Zero);
    keybd_event(0x12,0,2,UIntPtr.Zero); keybd_event(0x11,0,2,UIntPtr.Zero);
  }
}
'@
[void][OpacityPreview]::SetProcessDpiAwarenessContext([IntPtr](-4))
function Pump([int]$ms) {
    $until=[DateTime]::UtcNow.AddMilliseconds($ms)
    while([DateTime]::UtcNow -lt $until) { [System.Windows.Forms.Application]::DoEvents(); Start-Sleep -Milliseconds 20 }
}
$area=[System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
$x=$area.Left+[int](($area.Width-1000)/2)
$y=$area.Top+[int](($area.Height-650)/2)
$back=[System.Windows.Forms.Form]::new()
$front=[System.Windows.Forms.Form]::new()
try {
    $back.Text='Omarchy Overlay opacity preview - backdrop'
    $back.StartPosition='Manual'; $back.SetBounds($x,$y,1000,650)
    $back.BackColor=[System.Drawing.Color]::FromArgb(38,83,104)
    $grid=[System.Windows.Forms.Label]::new()
    $grid.Text="BACKGROUND`r`n`r`nThis stays visible behind the window.`r`n`r`n+       +       +       +       +`r`n`r`n+       +       +       +       +"
    $grid.Font=[System.Drawing.Font]::new('Consolas',22)
    $grid.ForeColor=[System.Drawing.Color]::FromArgb(170,220,230)
    $grid.SetBounds(36,32,920,540); $back.Controls.Add($grid)
    $back.Show(); Pump 300
    $front.Text='Omarchy Overlay demo - 80% opacity'
    $front.StartPosition='Manual'; $front.SetBounds(($x+120),($y+125),760,390)
    $front.BackColor=[System.Drawing.Color]::FromArgb(24,27,34)
    $label=[System.Windows.Forms.Label]::new()
    $label.Text="80% OPACITY`r`n`r`nA regular Windows window.`r`nOmarchy Overlay fades the whole window, including text."
    $label.Font=[System.Drawing.Font]::new('Segoe UI',20)
    $label.ForeColor=[System.Drawing.Color]::White
    $label.SetBounds(30,28,690,290); $front.Controls.Add($label)
    $front.Show(); Pump 400
    for($i=0;$i -lt 4;$i++) {
        if([OpacityPreview]::GetForegroundWindow() -ne $front.Handle) { throw 'Preview lost focus; no other window will be changed.' }
        [OpacityPreview]::Fade(); Pump 150
    }
    Pump 400
    $bitmap=[System.Drawing.Bitmap]::new(1000,650)
    $graphics=[System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($x,$y,0,0,$bitmap.Size)
        $bitmap.Save((Join-Path $root 'overlay-opacity.png'),[System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
} finally {
    $front.Close(); $front.Dispose(); $back.Close(); $back.Dispose()
    [OpacityPreview]::ShowAppearance()
}

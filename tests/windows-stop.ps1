$ErrorActionPreference = 'Stop'
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class StopApp {
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string name, string title);
  [DllImport("user32.dll")] static extern bool PostMessage(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
  public static void Run() {
    var hwnd = FindWindow("OmarchyOverlay.Main", null);
    if(hwnd != IntPtr.Zero) PostMessage(hwnd, 0x0312, (IntPtr)105, IntPtr.Zero);
  }
}
'@
[StopApp]::Run()
Start-Sleep -Seconds 2
if (Get-Process OmarchyOverlay -ErrorAction SilentlyContinue) { throw 'Omarchy Overlay did not exit normally.' }
'Previous Omarchy Overlay exited normally.'

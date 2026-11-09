# Historical 0.6.1 helper test; the removal control is retired in Omarchy Overlay 0.10.
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
$results=[Collections.Generic.List[object]]::new()
$app=$null;$forms=@();$madeKey=$false
$key='HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Rainmeter'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms -TypeDefinition @'
using System; using System.Runtime.InteropServices;
public static class RecoverTest {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int l,t,r,b; }
  [StructLayout(LayoutKind.Sequential)] struct APPBAR { public uint size; public IntPtr hwnd; public uint callback,edge; public RECT rect; public IntPtr data; }
  [StructLayout(LayoutKind.Sequential)] struct MONITOR { public uint size;public RECT screen,work;public uint flags; }
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string cls,string title);
  public static IntPtr MainWindow() { return FindWindow("OmarchyOverlay.Main",null); }
  public static IntPtr Overlay() { return FindWindow("OmarchyOverlay.Overlay", null); }
  // 0.16 opens the Omarchy Overlay menu at startup; the 0.15 dashboard stays hidden until asked for.
  public static bool Ready(IntPtr main) { return main != IntPtr.Zero && (IsWindowVisible(main) || IsWindowVisible(Overlay())); }
  [DllImport("user32.dll", EntryPoint="PostMessageW")] static extern bool PostToApp(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
  public static void CloseMenu() { IntPtr menu = Overlay(); if (IsWindowVisible(menu)) PostToApp(menu, 0x100, (IntPtr)0x1b, IntPtr.Zero); }
  [DllImport("user32.dll", EntryPoint="ShowWindow")] static extern bool ShowAppWindow(IntPtr hwnd, int command);
  public static bool ShowDashboard(IntPtr main) { ShowAppWindow(main, 1); return IsWindowVisible(main); }
  public static IntPtr Settings() { return FindWindow("OmarchyOverlay.Settings",null); }
  public static IntPtr Profiles() { return FindWindow("OmarchyOverlay.Profiles",null); }
  public static IntPtr Bar() { return FindWindow("OmarchyOverlay.TopBar",null); }
  public static IntPtr Dialog() { return FindWindow("#32770","Omarchy Overlay"); }
  public static IntPtr FileDialog(bool save) { return FindWindow("#32770",save?"Export Omarchy Overlay profile":"Import Omarchy Overlay profile"); }
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr value);
  [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr hwnd,int id);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hwnd,uint msg,IntPtr w,IntPtr l);
  [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] public static extern IntPtr Text(IntPtr hwnd,uint msg,IntPtr w,string text);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hwnd,uint msg,IntPtr w,IntPtr l);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd,out RECT rect);
  [DllImport("user32.dll",EntryPoint="SetForegroundWindow")] static extern bool NativeSetForegroundWindow(IntPtr hwnd);
  [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
  [DllImport("user32.dll")] static extern bool AttachThreadInput(uint from,uint to,bool attach);
  [DllImport("user32.dll")] static extern bool BringWindowToTop(IntPtr hwnd);
  [StructLayout(LayoutKind.Sequential)] struct FocusRect { public int left,top,right,bottom; }
  [DllImport("user32.dll",EntryPoint="GetWindowRect")] static extern bool ReadFocusRect(IntPtr hwnd,out FocusRect rect);
  [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
  [DllImport("user32.dll")] static extern bool SetCursorPos(int x,int y);
  [DllImport("user32.dll")] static extern void mouse_event(uint flags,uint x,uint y,uint data,UIntPtr extra);
  static int focusClick;
  public static bool SetForegroundWindow(IntPtr hwnd) {
    // Alternate caption positions so two activation clicks cannot maximize a window.
    // A real click on our own fixture avoids Windows' background-focus restrictions.
    FocusRect rect; if(!ReadFocusRect(hwnd,out rect)) return false;
    SetWindowPos(hwnd,(IntPtr)(-1),0,0,0,0,0x13);
    SetCursorPos(rect.left+80+32*((focusClick++)&1),rect.top+20);
    mouse_event(2,0,0,0,UIntPtr.Zero);mouse_event(4,0,0,0,UIntPtr.Zero);
    for(int i=0;i<30;i++) { System.Windows.Forms.Application.DoEvents(); if(GetForegroundWindow()==hwnd) break; System.Threading.Thread.Sleep(10); }
    SetWindowPos(hwnd,(IntPtr)(-2),0,0,0,0,0x13);
    return GetForegroundWindow()==hwnd;
  }
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint pid);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
  [DllImport("user32.dll")] static extern IntPtr GetWindowLongPtr(IntPtr hwnd,int index);
  [DllImport("user32.dll")] static extern bool GetLayeredWindowAttributes(IntPtr hwnd,out uint color,out byte alpha,out uint flags);
  [DllImport("shell32.dll")] static extern UIntPtr SHAppBarMessage(uint msg,ref APPBAR data);
  [DllImport("user32.dll")] static extern IntPtr MonitorFromWindow(IntPtr hwnd,uint flags);
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern bool GetMonitorInfo(IntPtr hwnd,ref MONITOR data);
  public static int WorkTop() {var d=new MONITOR();d.size=(uint)Marshal.SizeOf(d);GetMonitorInfo(MonitorFromWindow(IntPtr.Zero,1),ref d);return d.work.t;}
  [DllImport("user32.dll")] public static extern void keybd_event(byte key,byte scan,uint flags,UIntPtr extra);
  public static uint Taskbar() { var d=new APPBAR();d.size=(uint)Marshal.SizeOf(d);return (uint)SHAppBarMessage(4,ref d).ToUInt64() & 1; }
  public static void RestoreTaskbar(uint state) { var d=new APPBAR();d.size=(uint)Marshal.SizeOf(d);d.hwnd=FindWindow("Shell_TrayWnd",null);d.data=(IntPtr)state;SHAppBarMessage(10,ref d); }
  public static bool Layered(IntPtr hwnd) { return (GetWindowLongPtr(hwnd,-20).ToInt64() & 0x80000)!=0; }
  public static int Alpha(IntPtr hwnd) { uint c,f;byte a;return GetLayeredWindowAttributes(hwnd,out c,out a,out f)?a:-1; }
  public static void Shortcut(byte key) { keybd_event(0x11,0,0,UIntPtr.Zero);keybd_event(0x12,0,0,UIntPtr.Zero);keybd_event(key,0,0,UIntPtr.Zero);keybd_event(key,0,2,UIntPtr.Zero);keybd_event(0x12,0,2,UIntPtr.Zero);keybd_event(0x11,0,2,UIntPtr.Zero); }
}
'@
function Pump([int]$ms=700){$until=[DateTime]::UtcNow.AddMilliseconds($ms);while([DateTime]::UtcNow -lt $until){[Windows.Forms.Application]::DoEvents();Start-Sleep -Milliseconds 20}}
function Check([string]$name,[bool]$ok,$detail=''){$results.Add(@{name=$name;passed=$ok;detail=$detail});if(-not $ok){throw "$name ($detail)"}}
function Bounds($hwnd){$r=[RecoverTest+RECT]::new();[void][RecoverTest]::GetWindowRect($hwnd,[ref]$r);return "$($r.l),$($r.t),$($r.r),$($r.b)"}
function Click($hwnd,[int]$id){[void][RecoverTest]::PostMessage($hwnd,0x111,[IntPtr]$id,[IntPtr]::Zero);Pump}
# 0.17 turns tiling on at start by default (K7); these tests expect a paused start unless they set start_tiling themselves.
function Seed-Paused([string]$folder){$path=Join-Path $folder 'overlay-profile.json';if(-not (Test-Path $path)){return};$p=[IO.File]::ReadAllText($path)|ConvertFrom-Json;if($null -eq $p.settings){$p|Add-Member settings ([pscustomobject]@{})};if($null -ne $p.settings.PSObject.Properties['start_tiling']){return};$p.settings|Add-Member start_tiling $false;[IO.File]::WriteAllText($path,($p|ConvertTo-Json -Depth 20),[Text.UTF8Encoding]::new($false))}
function Start-App([string]$folder){
    Seed-Paused $folder
    $process=Start-Process (Join-Path $folder 'OmarchyOverlay.exe') -PassThru
    $deadline=[DateTime]::UtcNow.AddSeconds(20)
    do{Pump 100;$hwnd=[RecoverTest]::MainWindow();$owner=0;[void][RecoverTest]::GetWindowThreadProcessId($hwnd,[ref]$owner)}while(($hwnd -eq [IntPtr]::Zero -or $owner -ne $process.Id)-and [DateTime]::UtcNow -lt $deadline -and -not $process.HasExited)
    if($hwnd -eq [IntPtr]::Zero -or $owner -ne $process.Id){throw 'Controller did not start.'}
    Pump 700;return $process
}
function Stop-App($process){[void][RecoverTest]::PostMessage([RecoverTest]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero);$until=[DateTime]::UtcNow.AddSeconds(8);while(-not $process.HasExited -and [DateTime]::UtcNow -lt $until){Pump 100};Pump 700;if(-not $process.HasExited){throw 'Controller did not exit.'}}
function Read-Profile($folder){return Get-Content -Raw (Join-Path $folder 'overlay-profile.json')|ConvertFrom-Json}

Add-Type @'
using System;using System.Runtime.InteropServices;
public static class RemovalDialog {
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls,string title);
}
'@
try {
 [void][RecoverTest]::SetProcessDpiAwarenessContext([IntPtr](-4))
 $principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
 Check 'Removal helper runs as a limited user' (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))
 Check 'No existing Omarchy Overlay controller to disturb' ([RecoverTest]::MainWindow() -eq [IntPtr]::Zero)
 Check 'No existing per-user Rainmeter record to overwrite' (-not (Test-Path $key))
 $folder=Join-Path $root ('removal-helper-'+[Guid]::NewGuid().ToString('N'));New-Item -ItemType Directory $folder|Out-Null
 Copy-Item (Join-Path $root 'OmarchyOverlay.exe') (Join-Path $folder 'OmarchyOverlay.exe')
 [IO.File]::WriteAllText((Join-Path $folder 'overlay-profile.json'),'{"format":"overlay-profile","version":1,"id":"removal-helper-test","settings":{}}')
 $fixture=Join-Path $folder 'uninst.exe'
 Add-Type -ReferencedAssemblies System.Windows.Forms -OutputAssembly $fixture -OutputType WindowsApplication -TypeDefinition @'
using System;using System.IO;using System.Windows.Forms;
public static class RemovalFixture {
 [STAThread]public static void Main(string[] args){
  File.WriteAllText(Path.Combine(AppDomain.CurrentDomain.BaseDirectory,"fixture-started.txt"),"args="+String.Join("|",args));
 }
}
'@
 New-Item $key -Force|Out-Null;$madeKey=$true
 New-ItemProperty $key -Name DisplayName -Value 'Rainmeter'|Out-Null
 New-ItemProperty $key -Name UninstallString -Value ('"'+$fixture+'"')|Out-Null
 $app=Start-App $folder
 Click ([RecoverTest]::MainWindow()) 3
 $settings=[RecoverTest]::Settings()
 $button=[RecoverTest]::GetDlgItem($settings,70)
 Check 'Settings exposes the Remove Rainmeter button' ($button -ne [IntPtr]::Zero)
 $r=[RecoverTest+RECT]::new();$br=[RecoverTest+RECT]::new()
 [void][RecoverTest]::GetWindowRect($settings,[ref]$r);[void][RecoverTest]::GetWindowRect($button,[ref]$br)
 Check 'Removal button is visible inside the Settings window' ([RecoverTest]::IsWindowVisible($button) -and $br.b -lt $r.b -and $br.t -gt $r.t)
 [void][RecoverTest]::SetForegroundWindow($settings);Pump
 $bitmap=[Drawing.Bitmap]::new(($r.r-$r.l),($r.b-$r.t));$graphics=[Drawing.Graphics]::FromImage($bitmap)
 try{$graphics.CopyFromScreen($r.l,$r.t,0,0,$bitmap.Size);$bitmap.Save((Join-Path $root 'overlay-rainmeter-helper.png'),[Drawing.Imaging.ImageFormat]::Png)}finally{$graphics.Dispose();$bitmap.Dispose()}

 [void][RecoverTest]::PostMessage($settings,0x111,[IntPtr]70,[IntPtr]::Zero);Pump
 $dialog=[RemovalDialog]::FindWindow('#32770','Remove Rainmeter')
 Check 'Detected uninstaller opens a review dialog' ($dialog -ne [IntPtr]::Zero)
 [void][RecoverTest]::PostMessage($dialog,0x111,[IntPtr]2,[IntPtr]::Zero);Pump
 Check 'Cancel does not launch the uninstaller' (-not (Test-Path (Join-Path $folder 'fixture-started.txt')))
 [void][RecoverTest]::PostMessage($settings,0x111,[IntPtr]70,[IntPtr]::Zero);Pump
 $dialog=[RemovalDialog]::FindWindow('#32770','Remove Rainmeter')
 [void][RecoverTest]::PostMessage($dialog,0x111,[IntPtr]1,[IntPtr]::Zero);Pump 300
 $deadline=[DateTime]::UtcNow.AddSeconds(12)
 while(-not (Test-Path (Join-Path $folder 'fixture-started.txt')) -and [DateTime]::UtcNow -lt $deadline){Pump 100}
 Check 'Confirm launches the registered harmless uninstaller' (Test-Path (Join-Path $folder 'fixture-started.txt'))
 Check 'Uninstaller receives no silent flags' ((Get-Content -Raw (Join-Path $folder 'fixture-started.txt')) -eq 'args=')
 Set-ItemProperty $key UninstallString ('"'+(Join-Path $folder 'missing.exe')+'" /S')
 [void][RecoverTest]::PostMessage($settings,0x111,[IntPtr]70,[IntPtr]::Zero);Pump
 $dialog=[RemovalDialog]::FindWindow('#32770','Remove Rainmeter')
 Check 'Missing uninstaller produces the Installed apps fallback prompt' ($dialog -ne [IntPtr]::Zero)
 [void][RecoverTest]::PostMessage($dialog,0x111,[IntPtr]2,[IntPtr]::Zero);Pump
 Stop-App $app;$app=$null
 Check 'Omarchy Overlay exits normally after helper use' ([RecoverTest]::MainWindow() -eq [IntPtr]::Zero)
} catch {$results.Add(@{name='Removal helper harness';passed=$false;detail=$_.Exception.ToString()});throw}
finally {
 if($null -ne $app -and -not $app.HasExited){
  $dialog=[RemovalDialog]::FindWindow('#32770','Remove Rainmeter');if($dialog -ne [IntPtr]::Zero){[void][RecoverTest]::PostMessage($dialog,0x111,[IntPtr]2,[IntPtr]::Zero);Pump}
  Stop-App $app
 }
 if($madeKey){Remove-Item $key}
 $results|ConvertTo-Json -Depth 8|Set-Content -Encoding UTF8 (Join-Path $root 'rainmeter-helper-result.json')
}

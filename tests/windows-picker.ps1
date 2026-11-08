$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$resultFile = Join-Path $root 'picker-result.json'
$results = [System.Collections.Generic.List[object]]::new()
$forms = @()
$app = $null
$shortcut = $null
$marker = Join-Path $root 'launcher-marker.txt'
$originalTaskbar = $null
$configTouched = $false
$configPath = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'OmarchyOverlay\config.json'
$originalConfig = if (Test-Path $configPath) { [IO.File]::ReadAllText($configPath) } else { $null }
$upgradeBackupPath=[IO.Path]::ChangeExtension($configPath,'pre-mb-keyspace.json')
$originalUpgradeBackup=if(Test-Path $upgradeBackupPath){[IO.File]::ReadAllBytes($upgradeBackupPath)}else{$null}
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class Desktop {
  public delegate bool EnumProc(IntPtr hwnd, IntPtr data);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int left, top, right, bottom; }
  [DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc callback, IntPtr data);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string name, string title);
  public static IntPtr MainWindow() { return FindWindow("OmarchyOverlay.Main", null); }
  public static IntPtr Overlay() { return FindWindow("OmarchyOverlay.Overlay", null); }
  // 0.16 opens the Keyspace menu at startup; the 0.15 dashboard stays hidden until asked for.
  public static bool Ready(IntPtr main) { return main != IntPtr.Zero && (IsWindowVisible(main) || IsWindowVisible(Overlay())); }
  [DllImport("user32.dll", EntryPoint="PostMessageW")] static extern bool PostToKeyspace(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
  public static void CloseMenu() { IntPtr menu = Overlay(); if (IsWindowVisible(menu)) PostToKeyspace(menu, 0x100, (IntPtr)0x1b, IntPtr.Zero); }
  [DllImport("user32.dll", EntryPoint="ShowWindow")] static extern bool ShowKeyspaceWindow(IntPtr hwnd, int command);
  public static bool ShowDashboard(IntPtr main) { ShowKeyspaceWindow(main, 1); return IsWindowVisible(main); }
  public static IntPtr Launcher() { return FindWindow("OmarchyOverlay.Launcher", null); }
  public static IntPtr Bar() { return FindWindow("OmarchyOverlay.TopBar", null); }
  public static IntPtr Appearance() { return FindWindow("OmarchyOverlay.Appearance", null); }
  public static IntPtr Settings() { return FindWindow("OmarchyOverlay.Settings", null); }
  [DllImport("user32.dll")] static extern IntPtr GetWindowLongPtr(IntPtr hwnd, int index);
  [DllImport("user32.dll")] static extern IntPtr SetWindowLongPtr(IntPtr hwnd, int index, IntPtr value);
  [DllImport("user32.dll")] static extern bool GetLayeredWindowAttributes(IntPtr hwnd, out uint color, out byte alpha, out uint flags);
  [DllImport("user32.dll")] static extern bool SetLayeredWindowAttributes(IntPtr hwnd, uint color, byte alpha, uint flags);
  [DllImport("user32.dll")] static extern IntPtr GetDC(IntPtr hwnd);
  [DllImport("user32.dll")] static extern int ReleaseDC(IntPtr hwnd, IntPtr dc);
  [DllImport("gdi32.dll")] static extern uint GetPixel(IntPtr dc, int x, int y);
  public static uint Pixel(IntPtr hwnd) { var dc=GetDC(hwnd); var color=GetPixel(dc,4,4); ReleaseDC(hwnd,dc); return color; }
  public static bool Layered(IntPtr hwnd) { return (GetWindowLongPtr(hwnd,-20).ToInt64() & 0x80000)!=0; }
  public static int Alpha(IntPtr hwnd) { uint color,flags; byte alpha; return GetLayeredWindowAttributes(hwnd,out color,out alpha,out flags) ? alpha : -1; }
  public static void NativeOpacity(IntPtr hwnd, byte alpha) { SetWindowLongPtr(hwnd,-20,(IntPtr)(GetWindowLongPtr(hwnd,-20).ToInt64() | 0x80000)); SetLayeredWindowAttributes(hwnd,0,alpha,2); }
  [StructLayout(LayoutKind.Sequential)] public struct APPBAR { public uint size; public IntPtr hwnd; public uint callback,edge; public RECT rect; public IntPtr data; }
  [StructLayout(LayoutKind.Sequential)] public struct MONITOR { public uint size; public RECT screen,work; public uint flags; }
  [DllImport("shell32.dll")] static extern UIntPtr SHAppBarMessage(uint message, ref APPBAR data);
  [DllImport("user32.dll")] static extern IntPtr MonitorFromWindow(IntPtr hwnd, uint flags);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern bool GetMonitorInfo(IntPtr monitor, ref MONITOR data);
  [DllImport("user32.dll")] public static extern bool SetProcessDpiAwarenessContext(IntPtr context);
  public static uint TaskbarState() { var data=new APPBAR(); data.size=(uint)Marshal.SizeOf(data); return (uint)SHAppBarMessage(4,ref data).ToUInt64() & 1; }
  public static void SetTaskbarState(uint state) { var data=new APPBAR(); data.size=(uint)Marshal.SizeOf(data); data.hwnd=FindWindow("Shell_TrayWnd",null); data.data=(IntPtr)state; SHAppBarMessage(10,ref data); }
  public static RECT WorkArea() { var data=new MONITOR(); data.size=(uint)Marshal.SizeOf(data); GetMonitorInfo(MonitorFromWindow(IntPtr.Zero,1),ref data); return data.work; }
  [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr parent, int id);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hwnd, out RECT rect);
  [DllImport("user32.dll",EntryPoint="SetForegroundWindow")] static extern bool NativeSetForegroundWindow(IntPtr hwnd);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint pid);
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
    keybd_event(0x1b,0,0,UIntPtr.Zero);keybd_event(0x1b,0,2,UIntPtr.Zero);
    FocusRect rect; if(!ReadFocusRect(hwnd,out rect)) return false;
    SetWindowPos(hwnd,(IntPtr)(-1),0,0,0,0,0x13);
    SetCursorPos(rect.left+80+32*((focusClick++)&1),rect.top+20);
    mouse_event(2,0,0,0,UIntPtr.Zero);mouse_event(4,0,0,0,UIntPtr.Zero);
    for(int i=0;i<30;i++) { System.Windows.Forms.Application.DoEvents(); if(GetForegroundWindow()==hwnd) break; System.Threading.Thread.Sleep(10); }
    SetWindowPos(hwnd,(IntPtr)(-2),0,0,0,0,0x13);
    return GetForegroundWindow()==hwnd;
  }
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr hwnd);
  [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool SetWindowText(IntPtr hwnd, string text);
  [DllImport("user32.dll", EntryPoint="SendMessageW", CharSet=CharSet.Unicode)] public static extern IntPtr SendText(IntPtr hwnd, uint message, IntPtr w, string text);
  [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
  public static void Shortcut(byte key, bool shift=false) {
    keybd_event(0x11,0,0,UIntPtr.Zero); keybd_event(0x12,0,0,UIntPtr.Zero);
    if(shift) keybd_event(0x10,0,0,UIntPtr.Zero);
    keybd_event(key,0,0,UIntPtr.Zero); keybd_event(key,0,2,UIntPtr.Zero);
    if(shift) keybd_event(0x10,0,2,UIntPtr.Zero);
    keybd_event(0x12,0,2,UIntPtr.Zero); keybd_event(0x11,0,2,UIntPtr.Zero);
  }
}
'@

function Wait-Desktop([int]$milliseconds = 900) {
    $until = [DateTime]::UtcNow.AddMilliseconds($milliseconds)
    while ([DateTime]::UtcNow -lt $until) {
        [System.Windows.Forms.Application]::DoEvents()
        Start-Sleep -Milliseconds 20
    }
}
function Bounds($window) {
    $r = [Desktop+RECT]::new()
    [void][Desktop]::GetWindowRect($window, [ref]$r)
    return @($r.left, $r.top, $r.right, $r.bottom)
}
function Check([string]$name, [bool]$success, $detail = '') {
    $results.Add(@{ name=$name; passed=$success; detail=$detail })
    if (-not $success) { throw "Check failed: $name ($detail)" }
}
function Set-Slider($window, [int]$id, [int]$value) {
    $slider = [Desktop]::GetDlgItem($window,$id)
    [void][Desktop]::SendMessage($slider,1029,[IntPtr]1,[IntPtr]$value)
    [void][Desktop]::PostMessage($window,0x0114,[IntPtr]8,$slider)
    Wait-Desktop 850
}
function Click($window,[int]$id) { [void][Desktop]::PostMessage($window,0x0111,[IntPtr]$id,[IntPtr]::Zero); Wait-Desktop 650 }
function Choose($window,[int]$id,[int]$index) { [void][Desktop]::SendMessage([Desktop]::GetDlgItem($window,$id),0x014e,[IntPtr]$index,[IntPtr]::Zero) }
function Input-Text($window,[int]$id,[string]$value) {
 $h=[Desktop]::GetDlgItem($window,$id)
 if([Desktop]::SendMessage($h,0x146,[IntPtr]::Zero,[IntPtr]::Zero).ToInt64() -gt 0){
  $index=[Desktop]::SendText($h,0x158,[IntPtr](-1),$value)
  if($index.ToInt64() -lt 0){throw "Dropdown option not found: $id $value"}
  [void][Desktop]::SendMessage($h,0x14e,$index,[IntPtr]::Zero)
 }else{[void][Desktop]::SendText($h,12,[IntPtr]::Zero,$value)}
}
function Select-Rule($window,[int]$index=0) {
    $list=[Desktop]::GetDlgItem($window,42)
    [void][Desktop]::SendMessage($list,0x0186,[IntPtr]$index,[IntPtr]::Zero)
    [void][Desktop]::PostMessage($window,0x0111,[IntPtr](42+(1 -shl 16)),$list)
    Wait-Desktop 150
}
Add-Type @'
using System;using System.Text;using System.Runtime.InteropServices;
[ComImport,Guid("5CDF2C82-841E-4546-9722-0CF74078229A"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IVolume {
 [PreserveSig]int Register(IntPtr p);[PreserveSig]int Unregister(IntPtr p);[PreserveSig]int Channels(out uint n);
 [PreserveSig]int SetDb(float v,ref Guid g);[PreserveSig]int SetScalar(float v,ref Guid g);
 [PreserveSig]int GetDb(out float v);[PreserveSig]int GetScalar(out float v);
 [PreserveSig]int SetChannelDb(uint n,float v,ref Guid g);[PreserveSig]int SetChannelScalar(uint n,float v,ref Guid g);
 [PreserveSig]int GetChannelDb(uint n,out float v);[PreserveSig]int GetChannelScalar(uint n,out float v);
 [PreserveSig]int SetMute([MarshalAs(UnmanagedType.Bool)]bool mute,ref Guid g);[PreserveSig]int GetMute([MarshalAs(UnmanagedType.Bool)]out bool mute);
}
[ComImport,Guid("D666063F-1587-4E43-81F1-B948E807363F"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IDevice { [PreserveSig]int Activate(ref Guid id,uint context,IntPtr parameters,out IVolume volume); }
[ComImport,Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IDevices {
 [PreserveSig]int Enum(int flow,uint state,out IntPtr collection);
 [PreserveSig]int Default(int flow,int role,out IDevice device);
}
public static class PickerTest {
 [DllImport("user32.dll")] static extern bool PostMessage(IntPtr hwnd,uint message,IntPtr w,IntPtr l);
 public static IntPtr CalculatorWindow(){return FindWindow(null,"Calculator");}
 public static void CloseCalculator(){var hwnd=CalculatorWindow();if(hwnd!=IntPtr.Zero)PostMessage(hwnd,0x10,IntPtr.Zero,IntPtr.Zero);}
 static IVolume Endpoint(){var en=(IDevices)Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")));IDevice dev;IVolume volume;try{Marshal.ThrowExceptionForHR(en.Default(0,1,out dev));}finally{Marshal.ReleaseComObject(en);}try{var id=new Guid("5CDF2C82-841E-4546-9722-0CF74078229A");Marshal.ThrowExceptionForHR(dev.Activate(ref id,1,IntPtr.Zero,out volume));return volume;}finally{Marshal.ReleaseComObject(dev);}}
 public static float Volume(){var v=Endpoint();try{float result;Marshal.ThrowExceptionForHR(v.GetScalar(out result));return result;}finally{Marshal.ReleaseComObject(v);}}
 public static bool Muted(){var v=Endpoint();try{bool result;Marshal.ThrowExceptionForHR(v.GetMute(out result));return result;}finally{Marshal.ReleaseComObject(v);}}
 public static void Set(float level,bool mute){var v=Endpoint();try{var id=Guid.Empty;Marshal.ThrowExceptionForHR(v.SetScalar(level,ref id));Marshal.ThrowExceptionForHR(v.SetMute(mute,ref id));}finally{Marshal.ReleaseComObject(v);}}
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string cls,string title);
 public static IntPtr BarSettings(){return FindWindow("OmarchyOverlay.BarSettings",null);}
 [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] static extern IntPtr ReadText(IntPtr hwnd,uint msg,IntPtr w,StringBuilder text);
 public static string Item(IntPtr hwnd,int index){var text=new StringBuilder(4096);ReadText(hwnd,0x189,(IntPtr)index,text);return text.ToString();}
 [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
 public static void Top(IntPtr hwnd){SetWindowPos(hwnd,(IntPtr)(-1),0,0,0,0,0x53);}
}
'@
$historyPath=Join-Path (Split-Path $configPath) 'app-picker.json'
$history=if(Test-Path $historyPath){[IO.File]::ReadAllText($historyPath)}else{$null}
$volume=$null;$mute=$null;$calculator=$null;$calculatorCreated=$false
function Capture($hwnd,$name){$r=Bounds $hwnd;$b=[Drawing.Bitmap]::new(($r[2]-$r[0]),($r[3]-$r[1]));$g=[Drawing.Graphics]::FromImage($b);try{$g.CopyFromScreen($r[0],$r[1],0,0,$b.Size);$b.Save((Join-Path $root $name),[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$b.Dispose()}}
try {
 [void][Desktop]::SetProcessDpiAwarenessContext([IntPtr](-4))
 Check 'No controller to disturb' ([Desktop]::MainWindow() -eq [IntPtr]::Zero)
 [IO.Directory]::CreateDirectory((Split-Path $configPath))|Out-Null
 $fixtureExcluded=@('Taskmgr.exe','mstsc.exe','msrdc.exe','ApplicationFrameHost.exe','vmconnect.exe')+@(Get-Process | Where-Object {$_.SessionId -eq [Diagnostics.Process]::GetCurrentProcess().SessionId -and $_.ProcessName -ne 'powershell'} | ForEach-Object {$_.ProcessName+'.exe'} | Select-Object -Unique)
$fixtureDefaults=(@{excluded_processes=$fixtureExcluded;start_tiling=$false}|ConvertTo-Json -Depth 5)
[IO.File]::WriteAllText($configPath,$fixtureDefaults,[Text.UTF8Encoding]::new($false));$configTouched=$true
 [IO.File]::WriteAllText($historyPath,'{}',[Text.UTF8Encoding]::new($false))
 $originalTaskbar=[Desktop]::TaskbarState()
 $app=Start-Process (Join-Path $root 'OmarchyOverlay.exe') -PassThru
 $deadline=[DateTime]::UtcNow.AddSeconds(30)
 do{Wait-Desktop 200;$main=[Desktop]::MainWindow()}while(-not [Desktop]::Ready($main)-and [DateTime]::UtcNow -lt $deadline)
 Check 'App opens with Shell application discovery enabled' ([Desktop]::Ready($main));[Desktop]::CloseMenu()
 # The 0.15 search window refreshes the app list before it shows, which can take over a second.
 Click $main 2;$picker=[Desktop]::Launcher();$searchDeadline=[DateTime]::UtcNow.AddSeconds(5)
 while(-not [Desktop]::IsWindowVisible($picker) -and [DateTime]::UtcNow -lt $searchDeadline){Wait-Desktop 100;$picker=[Desktop]::Launcher()}
 $list=[Desktop]::GetDlgItem($picker,11)
 Check 'Larger app picker opens' ([Desktop]::IsWindowVisible($picker))
 # Add a shortcut after startup and discover it through Refresh.
 $shortcut=Join-Path ([Environment]::GetFolderPath('Programs')) ('Keyspace Picker Zebra '+[Guid]::NewGuid().ToString('N')+'.lnk')
 $link=(New-Object -ComObject WScript.Shell).CreateShortcut($shortcut);$link.TargetPath=Join-Path $env:SystemRoot 'System32\cmd.exe';$link.Arguments='/d /c echo launched>"'+$marker+'"';$link.Save()
 Click $picker 14;Input-Text $picker 10 'kpz';Wait-Desktop 400
 Check 'Fuzzy abbreviation finds newly refreshed shortcut' ([PickerTest]::Item($list,0).Contains('Keyspace Picker Zebra'))
 Click $picker 13
 Check 'Pin is visible beside an app' ([PickerTest]::Item($list,0).StartsWith([string][char]0x2605))
 Check 'Favorite preference saves locally' (Test-Path $historyPath)
 Input-Text $picker 10 '';Wait-Desktop 250
 Check 'Favorite rises to first entry with empty search' ([PickerTest]::Item($list,0).Contains('Keyspace Picker Zebra'))
 [PickerTest]::Top($picker);Wait-Desktop 250;Capture $picker 'keyspace-app-picker.png'
 Click $picker 12
 $launchDeadline=[DateTime]::UtcNow.AddSeconds(10)
 do{Wait-Desktop 100}while(-not (Test-Path $marker) -and [DateTime]::UtcNow -lt $launchDeadline)
 Check 'Visible Open app button launches the selected shortcut' (Test-Path $marker)
 Click $main 2;Input-Text $picker 10 '';Wait-Desktop 300
 Check 'Recently launched app stays prominent' ([PickerTest]::Item($list,0).Contains('Keyspace Picker Zebra'))
 # Remove the disposable app before capturing a representative app-picker view.
 Remove-Item $shortcut;$shortcut=$null;Click $main 2;Click $picker 14
 Input-Text $picker 10 'Calculator';Wait-Desktop 200;Click $picker 13
 Input-Text $picker 10 '';Wait-Desktop 200;[PickerTest]::Top($picker);Wait-Desktop 200
 Capture $picker 'keyspace-app-picker.png'
 # Verify a packaged application absent from shortcut-only discovery.
 Check 'No existing Calculator window to disturb' ([PickerTest]::CalculatorWindow() -eq [IntPtr]::Zero)
 Input-Text $picker 10 'Calculator';Wait-Desktop 300
 Check 'Windows Shell discovery includes Calculator' ([PickerTest]::Item($list,0).Contains('Calculator'))
 $calculatorCreated=$true;Click $picker 12
 $deadline=[DateTime]::UtcNow.AddSeconds(15)
 do {Wait-Desktop 300;$calculator=[PickerTest]::CalculatorWindow()} while($calculator -eq [IntPtr]::Zero -and [DateTime]::UtcNow -lt $deadline)
 Check 'Shell application launches through its registered target' ($calculator -ne [IntPtr]::Zero -and [Desktop]::IsWindowVisible($calculator))
 [PickerTest]::CloseCalculator();Wait-Desktop 650;$calculator=$null;$calculatorCreated=$false
 [void][Desktop]::SendMessage($picker,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 $volume=[PickerTest]::Volume();$mute=[PickerTest]::Muted();[PickerTest]::Set(0.42,$false)
 Click $main 14;$settings=[PickerTest]::BarSettings()
 Input-Text $settings 201 'sound';Input-Text $settings 202 '';Input-Text $settings 203 '';Click $settings 211
 [void][Desktop]::SendMessage($settings,0x10,[IntPtr]::Zero,[IntPtr]::Zero);Wait-Desktop 1800
 $bar=[Desktop]::Bar();$r=Bounds $bar;Capture $bar 'keyspace-volume.png'
 # Wheel uses screen coordinates; middle-click uses client coordinates.
 [void][Desktop]::PostMessage($bar,0x20a,[IntPtr](120 -shl 16),[IntPtr](($r[0]+35)+(($r[1]+15) -shl 16)));Wait-Desktop 800
 Check 'Wheel over sound changes actual Windows master volume' ([Math]::Abs([PickerTest]::Volume()-0.44) -lt 0.015)
 [void][Desktop]::PostMessage($bar,0x208,[IntPtr]::Zero,[IntPtr](35+(15 -shl 16)));Wait-Desktop 1400
 Check 'Middle-click over sound mutes Windows audio' ([PickerTest]::Muted())
 Capture $bar 'keyspace-mute.png'
 [void][Desktop]::PostMessage($main,0x312,[IntPtr]105,[IntPtr]::Zero)
 $exitDeadline=[DateTime]::UtcNow.AddSeconds(8)
 do{Wait-Desktop 100}while(-not $app.HasExited -and [DateTime]::UtcNow -lt $exitDeadline)
 Check 'Exit closes app after Shell and audio use' $app.HasExited
 $app=Start-Process (Join-Path $root 'OmarchyOverlay.exe') -PassThru
 $deadline=[DateTime]::UtcNow.AddSeconds(30)
 do{Wait-Desktop 200;$main=[Desktop]::MainWindow()}while(-not [Desktop]::Ready($main)-and [DateTime]::UtcNow -lt $deadline);[Desktop]::CloseMenu()
 Click $main 2;$picker=[Desktop]::Launcher();$list=[Desktop]::GetDlgItem($picker,11)
 Check 'Favorite app persists after restarting Keyspace' ([PickerTest]::Item($list,0).Contains('Calculator') -and [PickerTest]::Item($list,0).StartsWith([string][char]0x2605))
} catch {$results.Add(@{name='Picker/audio harness';passed=$false;detail=$_.Exception.ToString()})}
finally {
 if($null -ne $app -and -not $app.HasExited){[void][Desktop]::PostMessage([Desktop]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero);Wait-Desktop 1400}
 if($null -ne $volume){[PickerTest]::Set($volume,$mute)}
 if($calculatorCreated){[PickerTest]::CloseCalculator()}
 if($null -ne $originalTaskbar){[Desktop]::SetTaskbarState($originalTaskbar)}
 if($null -ne $shortcut -and (Test-Path $shortcut)){Remove-Item $shortcut}
 if(Test-Path $marker){Remove-Item $marker}
 if($configTouched){if($null -ne $originalConfig){[IO.File]::WriteAllText($configPath,$originalConfig,[Text.UTF8Encoding]::new($false))}elseif(Test-Path $configPath){Remove-Item $configPath}}
 if($null -ne $history){[IO.File]::WriteAllText($historyPath,$history,[Text.UTF8Encoding]::new($false))}elseif(Test-Path $historyPath){Remove-Item $historyPath}
 if($configTouched){
  if($null -ne $originalUpgradeBackup){[IO.File]::WriteAllBytes($upgradeBackupPath,$originalUpgradeBackup)}
  elseif(Test-Path $upgradeBackupPath){Remove-Item -LiteralPath $upgradeBackupPath}
 }
 $results|ConvertTo-Json -Depth 5|Set-Content -Encoding UTF8 $resultFile
}

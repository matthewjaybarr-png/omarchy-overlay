$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$resultFile = Join-Path $root 'roadmap-effects-result.json'
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

 [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr h);
 [DllImport("user32.dll")] public static extern bool IsWindow(IntPtr h);
 [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] static extern IntPtr ReadText(IntPtr h,uint m,IntPtr count,StringBuilder t);
 public static string ControlText(IntPtr h,int id){var t=new StringBuilder(4096);ReadText(GetDlgItem(h,id),13,(IntPtr)t.Capacity,t);return t.ToString();}
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern bool SystemParametersInfo(uint action,uint param,StringBuilder text,uint flags);
 public static string Wallpaper(){var text=new StringBuilder(32768);if(!SystemParametersInfo(0x73,(uint)text.Capacity,text,0))throw new Exception("Get wallpaper");return text.ToString();}
 [DllImport("user32.dll")] static extern int GetWindowRgn(IntPtr h,IntPtr r);
 [DllImport("gdi32.dll")] static extern IntPtr CreateRectRgn(int l,int t,int r,int b);
 [DllImport("gdi32.dll")] static extern bool PtInRegion(IntPtr h,int x,int y);
 [DllImport("gdi32.dll")] static extern bool DeleteObject(IntPtr h);
 public static bool Hollow(IntPtr h){var r=CreateRectRgn(0,0,0,0);try{GetWindowRgn(h,r);RECT box;GetWindowRect(h,out box);return PtInRegion(r,0,0)&&!PtInRegion(r,(box.right-box.left)/2,(box.bottom-box.top)/2);}finally{DeleteObject(r);}}

  public static void Top(IntPtr h){SetWindowPos(h,(IntPtr)(-1),0,0,0,0,0x13);}
 [DllImport("ntdll.dll")] public static extern int NtSuspendProcess(IntPtr process);
  public static IntPtr ByClass(string cls) { return FindWindow(cls, null); }
 [DllImport("dwmapi.dll")] static extern int DwmGetWindowAttribute(IntPtr h,uint attribute,out uint value,uint size);
 [DllImport("dwmapi.dll")] static extern int DwmSetWindowAttribute(IntPtr h,uint attribute,ref uint value,uint size);
 public static uint Frame(IntPtr h,uint attribute){uint value;if(DwmGetWindowAttribute(h,attribute,out value,4)<0)throw new Exception("Get DWM "+attribute);return value;}
 public static void SetFrame(IntPtr h,uint attribute,uint value){if(DwmSetWindowAttribute(h,attribute,ref value,4)<0)throw new Exception("Set DWM");}
 [DllImport("user32.dll",EntryPoint="SystemParametersInfoW")] static extern bool ReadMotion(uint a,uint p,out int value,uint flags);
 [DllImport("user32.dll",EntryPoint="SystemParametersInfoW")] static extern bool WriteMotion(uint a,uint p,IntPtr value,uint flags);
 public static bool Motion(){int value;if(!ReadMotion(0x1042,0,out value,0))throw new Exception("Read animation setting");return value!=0;}
 public static void Motion(bool value){if(!WriteMotion(0x1043,0,(IntPtr)(value?1:0),0))throw new Exception("Set animation setting");}
 public static string Item(IntPtr h,int i){var text=new StringBuilder(4096);ReadText(h,0x189,(IntPtr)i,text);return text.ToString();}
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
    Write-Output ("CHECK $name : $success $detail")
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
using System;using System.Runtime.InteropServices;using System.Threading;
public static class ResizeTest {
 delegate IntPtr Proc(IntPtr hwnd,uint msg,IntPtr w,IntPtr l);
 static Proc proc=DefWindowProc;
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] struct Class {public uint style;public Proc proc;public int clsExtra,wndExtra;public IntPtr instance,icon,cursor,background;public string menu,name;}
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern ushort RegisterClass(ref Class cls);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr DefWindowProc(IntPtr hwnd,uint msg,IntPtr w,IntPtr l);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] static extern IntPtr GetModuleHandle(string name);
 [DllImport("user32.dll")] static extern IntPtr LoadIcon(IntPtr instance,IntPtr id);
 [DllImport("user32.dll")] static extern IntPtr LoadCursor(IntPtr instance,IntPtr id);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr CreateWindowEx(uint ex,string cls,string title,uint style,int x,int y,int w,int h,IntPtr parent,IntPtr menu,IntPtr instance,IntPtr data);
 [DllImport("user32.dll")] public static extern bool DestroyWindow(IntPtr hwnd);
 [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
 [StructLayout(LayoutKind.Sequential)] struct Rect{public int left,top,right,bottom;}
 [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd,out Rect rect);
 [DllImport("user32.dll")] static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] static extern void mouse_event(uint flags,uint x,uint y,uint data,UIntPtr extra);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string cls,string title);
 public static IntPtr BarSettings(){return FindWindow("OmarchyOverlay.BarSettings",null);}
 public static IntPtr Fixture(int i){var cls=new Class();cls.proc=proc;cls.name="OmarchyOverlay.ResizeFixture";cls.instance=GetModuleHandle(null);cls.icon=LoadIcon(IntPtr.Zero,(IntPtr)32516);cls.cursor=LoadCursor(IntPtr.Zero,(IntPtr)32512);cls.background=(IntPtr)6;RegisterClass(ref cls);return CreateWindowEx(0,cls.name,"Resize fixture "+i,0x10cf0000,100+i*80,140+i*60,520,380,IntPtr.Zero,IntPtr.Zero,cls.instance,IntPtr.Zero);}
 public static bool LiveChanged;
 public static void Drag(IntPtr hwnd,IntPtr neighbor,bool horizontal,int delta){
  LiveChanged=false;Rect start,other;GetWindowRect(hwnd,out start);GetWindowRect(neighbor,out other);
  SetWindowPos(hwnd,(IntPtr)(-1),0,0,0,0,0x13);
  var worker=new Thread(()=>{int x=horizontal?start.right-3:(start.left+start.right)/2;int y=horizontal?(start.top+start.bottom)/2:start.bottom-3;
   try{SetCursorPos(x,y);Thread.Sleep(150);mouse_event(2,0,0,0,UIntPtr.Zero);Thread.Sleep(100);
    for(int i=1;i<=12;i++){SetCursorPos(x+(horizontal?delta*i/12:0),y+(horizontal?0:delta*i/12));Thread.Sleep(40);}
    Thread.Sleep(200);Rect now;GetWindowRect(neighbor,out now);LiveChanged=now.left!=other.left||now.top!=other.top||now.right!=other.right||now.bottom!=other.bottom;Thread.Sleep(100);
   }finally{mouse_event(4,0,0,0,UIntPtr.Zero);SetWindowPos(hwnd,(IntPtr)(-2),0,0,0,0,0x13);}
  });worker.IsBackground=true;worker.Start();
 }
}
'@
$windows=@()
function Capture($hwnd,$name){Check ('Visible capture '+$name) ([Desktop]::IsWindowVisible($hwnd));[Desktop]::Top($hwnd);Wait-Desktop 300;$r=Bounds $hwnd;$b=[Drawing.Bitmap]::new(($r[2]-$r[0]),($r[3]-$r[1]));$g=[Drawing.Graphics]::FromImage($b);try{$g.CopyFromScreen($r[0],$r[1],0,0,$b.Size);$b.Save((Join-Path $root $name),[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$b.Dispose()}}


function Wait-Panel([string]$class){
 $until=[DateTime]::UtcNow.AddSeconds(8)
 do{$window=[Desktop]::ByClass($class);if([Desktop]::IsWindowVisible($window)){return $window};Wait-Desktop 100}while([DateTime]::UtcNow -lt $until)
 return $window
}
function Key-Action([int]$id){[void][Desktop]::PostMessage($main,0x312,[IntPtr](100+$id),[IntPtr]::Zero);Wait-Desktop 900}

$local=Split-Path $configPath;$motionBefore=$null;$framesBefore=@();$colorsBefore=$null
function Colors(){ $key=[Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Microsoft\Windows\CurrentVersion\Themes\Personalize');try{return ([ordered]@{AppsUseLightTheme=$key.GetValue('AppsUseLightTheme',$null);SystemUsesLightTheme=$key.GetValue('SystemUsesLightTheme',$null)}|ConvertTo-Json -Compress)}finally{$key.Dispose()}}
function Check-Box($h,[int]$id,[bool]$checked){[void][Desktop]::SendMessage([Desktop]::GetDlgItem($h,$id),0xf1,[IntPtr]([int]$checked),[IntPtr]::Zero)}
function Select-Fixture($h){$list=[Desktop]::GetDlgItem($h,1001);$n=[Desktop]::SendMessage($list,0x18b,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32();for($i=0;$i -lt $n;$i++){if([Desktop]::Item($list,$i) -match 'Resize fixture 0'){[void][Desktop]::SendMessage($list,0x186,[IntPtr]$i,[IntPtr]::Zero);Click $h (1001+(1 -shl 16));return}};throw 'Fixture not in app chooser'}
try {
 [void][Desktop]::SetProcessDpiAwarenessContext([IntPtr](-4))
 Check 'No controller to disturb' ([Desktop]::MainWindow() -eq [IntPtr]::Zero)
 Check 'No pending frame or Windows-color recovery to disturb' (-not (Test-Path "$local\window-theme-restore.json") -and -not (Test-Path "$local\windows-colors-restore.json"))
 $originalTaskbar=[Desktop]::TaskbarState();$motionBefore=[Desktop]::Motion();$colorsBefore=Colors
 $excluded=@(Get-Process|Where-Object {$_.SessionId -eq [Diagnostics.Process]::GetCurrentProcess().SessionId -and $_.ProcessName -ne 'powershell'}|ForEach-Object {$_.ProcessName+'.exe'}|Select-Object -Unique)
 $config=@{theme='tokyo-night';excluded_processes=$excluded;start_tiling=$false;rules=@(@{executable='powershell.exe';mode='ignore'},@{executable='powershell.exe';window_class='OmarchyOverlay.ResizeFixture';mode='tile'})}
 [IO.File]::WriteAllText($configPath,($config|ConvertTo-Json -Depth 8),[Text.UTF8Encoding]::new($false));$configTouched=$true
 for($i=0;$i -lt 3;$i++){$windows+=,[ResizeTest]::Fixture($i)};Wait-Desktop 300
 $originals=@($windows|ForEach-Object{,(Bounds $_)})
 foreach($h in $windows){$f=@{};foreach($a in @(20,38)){$f[$a]=[Desktop]::Frame($h,$a)};$framesBefore+=,$f}
 $app=Start-Process "$root\OmarchyOverlay.exe" -PassThru;[void](Wait-Panel 'OmarchyOverlay.Overlay');$main=[Desktop]::MainWindow();[Desktop]::CloseMenu();Click $main 1;Wait-Desktop 1000
 Click $main 8;Click ([Desktop]::Appearance()) 38;$profiles=Wait-Panel 'OmarchyOverlay.AppearanceProfiles';Click $profiles 828;$options=Wait-Panel 'OmarchyOverlay.AppearanceOptions'
 Check 'Material and motion options open from Appearance' ([Desktop]::IsWindowVisible($options))
 Choose $options 1201 2;Check-Box $options 1202 $true;Check-Box $options 1203 $true;Input-Text $options 1204 '300';Input-Text $options 1210 '100';Input-Text $options 1211 '100';Click $options 1206;Wait-Desktop 1000
 $saved=Get-Content $configPath -Raw|ConvertFrom-Json
 Check 'Global material, frame and animation settings save' ($saved.window_backdrop -eq 'acrylic' -and $saved.coordinate_windows -and $saved.animations -and $saved.animation_duration_ms -eq 300)
 # 0.17 (3.2): the 0.15 windows draw GDI text with no alpha, so they stay solid (DWMSBT_NONE=1) whatever material is picked.
 Check 'The 0.15 options window stays solid while App windows = Acrylic (3.2)' ([Desktop]::Frame($options,38) -eq 1) ([Desktop]::Frame($options,38))
 Check 'Eligible app frame mode follows the dark Tokyo Night theme' ([Desktop]::Frame($windows[0],20) -eq 1)
 Check 'External app frame changes preserve a local recovery record' (Test-Path "$local\window-theme-restore.json")
 Check-Box $options 1202 $false;Click $options 1206;Wait-Desktop 900
 Check 'Turning global frame coordination off restores original app attributes' (@(20,38|Where-Object {[Desktop]::Frame($windows[0],$_) -ne $framesBefore[0][$_]}).Count -eq 0)
 Check 'Disabling app-frame coordination leaves the 0.15 options window solid' ([Desktop]::Frame($options,38) -eq 1) ([Desktop]::Frame($options,38))
 Click $options 1207;$apps=Wait-Panel 'OmarchyOverlay.AppRules';Select-Fixture $apps;Choose $apps 1009 1;Choose $apps 1012 3;Click $apps 1010;Wait-Desktop 900
 Check 'Per-app material can be selected independently of frame colors' ([Desktop]::Frame($windows[0],38) -eq 3 -and [Desktop]::Frame($windows[0],20) -eq $framesBefore[0][20])
 Choose $apps 1012 1;Click $apps 1010;Wait-Desktop 900
 Check 'Per-app native material restores the original backdrop' ([Desktop]::Frame($windows[0],38) -eq $framesBefore[0][38])
 [void][Desktop]::PostMessage($apps,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 Click $options 1208
 $colors=Colors|ConvertFrom-Json
 Check 'Explicit Windows coordination applies native dark-mode preferences' ($colors.AppsUseLightTheme -eq 0 -and $colors.SystemUsesLightTheme -eq 0)
 Check 'Windows color preference change has a local undo journal' (Test-Path "$local\windows-colors-restore.json")
 Click $options 1209
 Check 'Restore returns the exact original Windows color preferences' ((Colors) -eq $colorsBefore -and -not (Test-Path "$local\windows-colors-restore.json"))
 Capture $options 'appearance-options-acrylic.png'
 Click $options 1205
 Check 'Readability action restores solid surfaces and an opaque bar' ((Get-Content $configPath -Raw|ConvertFrom-Json).window_backdrop -eq 'solid' -and [Desktop]::Frame($options,38) -eq 1 -and -not [Desktop]::Layered([Desktop]::Bar()))
 Capture $options 'appearance-options-solid.png'
 $appearance=[Desktop]::Appearance();Choose $appearance 30 4;Click $appearance (30+(1 -shl 16));Wait-Desktop 300
 Check 'Solid Paper surface uses its readable light palette' ([Desktop]::Pixel($options) -eq 0xeaf2f5)
 Capture $options 'appearance-options-paper.png'
 Choose $appearance 30 1;Click $appearance (30+(1 -shl 16));Wait-Desktop 300

 # Observe real SetWindowPos transitions, then verify the Windows reduced-motion choice.
 [Desktop]::Motion($true)
 [void][Desktop]::PostMessage($main,0x111,[IntPtr]15,[IntPtr]::Zero)
 $seen=@{};for($i=0;$i -lt 24;$i++){Wait-Desktop 20;$seen[((Bounds $windows[0])-join ',')]=1}
 Check 'Enabled tile animation produces intermediate native window bounds' ($seen.Count -ge 3) ("distinct=$($seen.Count)")
 $settled=(Bounds $windows[0])-join ',';Wait-Desktop 800
 Check 'Animated layout reaches stable final tile bounds' (((Bounds $windows[0])-join ',') -eq $settled)
 [Desktop]::Motion($false)
 [void][Desktop]::PostMessage($main,0x111,[IntPtr]15,[IntPtr]::Zero);Wait-Desktop 70;$reduced=(Bounds $windows[0])-join ',';Wait-Desktop 500
 Check 'Windows reduced-motion preference suppresses the transition' (((Bounds $windows[0])-join ',') -eq $reduced)
 [Desktop]::Motion($motionBefore)
 # Named profiles carry advanced appearance choices without changing app rules.
 [void][Desktop]::PostMessage($options,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 Click $main 8;Click ([Desktop]::Appearance()) 38;$profiles=Wait-Panel 'OmarchyOverlay.AppearanceProfiles'
 Input-Text $profiles 802 'Work Material';Click $profiles 809
 Check 'Saved appearance profile includes advanced material and motion choices' ((Get-Content $configPath -Raw|ConvertFrom-Json).appearance_profiles[0].advanced.animation_duration_ms -eq 300)
 Click $profiles 828;$options=Wait-Panel 'OmarchyOverlay.AppearanceOptions';Click $options 1208
 # Opt the known fixture rule into frame colors for recovery verification.
 Click $options 1207;$apps=Wait-Panel 'OmarchyOverlay.AppRules';Select-Fixture $apps;Choose $apps 1009 2;Choose $apps 1012 2;Click $apps 1010;Wait-Desktop 900
 # Freeze Keyspace first: its frame poll would otherwise re-apply the rule before the kill lands.
 [void][Desktop]::NtSuspendProcess($app.Handle);[Desktop]::SetFrame($windows[0],38,4)
 $app.Kill();[void]$app.WaitForExit(5000);$app=$null
 $until=[DateTime]::UtcNow.AddSeconds(12);do{Wait-Desktop 200}while(((Test-Path "$local\window-theme-restore.json") -or (Test-Path "$local\windows-colors-restore.json")) -and [DateTime]::UtcNow -lt $until)
 Check 'Recovery preserves a frame attribute changed outside Keyspace' ([Desktop]::Frame($windows[0],38) -eq 4)
 Check 'Recovery restores the other original frame attributes' (@(@(20)|Where-Object {[Desktop]::Frame($windows[0],$_) -ne $framesBefore[0][$_]}).Count -eq 0)
 Check 'Recovery restores Windows light/dark preferences after termination' ((Colors) -eq $colorsBefore)
 Check 'Successful frame and color recovery clears the journals' (-not (Test-Path "$local\window-theme-restore.json") -and -not (Test-Path "$local\windows-colors-restore.json"))
 # Verify the same reversible choices through the normal exit path.
 $normalMaterial=[Desktop]::Frame($windows[0],38)
 $app=Start-Process "$root\OmarchyOverlay.exe" -PassThru;[void](Wait-Panel 'OmarchyOverlay.Overlay');$main=[Desktop]::MainWindow();[Desktop]::CloseMenu()
 Click $main 8;Click ([Desktop]::Appearance()) 38;$profiles=Wait-Panel 'OmarchyOverlay.AppearanceProfiles';Click $profiles 828;$options=Wait-Panel 'OmarchyOverlay.AppearanceOptions';Click $options 1208
 Check 'Restart adopts configured frame mode without tiling' ([Desktop]::Frame($windows[0],20) -eq 1)
 [void][Desktop]::PostMessage($main,0x312,[IntPtr]105,[IntPtr]::Zero)
 $until=[DateTime]::UtcNow.AddSeconds(12);do{Wait-Desktop 100}while(-not $app.HasExited -and [DateTime]::UtcNow -lt $until)
 Check 'Normal exit restores original frame mode and material' ([Desktop]::Frame($windows[0],20) -eq $framesBefore[0][20] -and [Desktop]::Frame($windows[0],38) -eq $normalMaterial)
 Check 'Normal exit restores Windows color preferences' ((Colors) -eq $colorsBefore)
 Check 'Normal exit removes successful appearance recovery records' (-not (Test-Path "$local\window-theme-restore.json") -and -not (Test-Path "$local\windows-colors-restore.json"))

} catch {$results.Add(@{name='Appearance integration harness';passed=$false;detail=$_.Exception.ToString()})}
finally {
 if($null -ne $app -and -not $app.HasExited){[void][Desktop]::PostMessage([Desktop]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero);$until=[DateTime]::UtcNow.AddSeconds(8);do{Wait-Desktop 100}while(-not $app.HasExited -and [DateTime]::UtcNow -lt $until)}
 if($null -ne $motionBefore){[Desktop]::Motion($motionBefore)}
 foreach($h in $windows){[void][ResizeTest]::DestroyWindow($h)}
 if($null -ne $originalTaskbar){[Desktop]::SetTaskbarState($originalTaskbar)}
 if($configTouched){if($null -ne $originalConfig){[IO.File]::WriteAllText($configPath,$originalConfig,[Text.UTF8Encoding]::new($false))}elseif(Test-Path $configPath){Remove-Item $configPath}}
 if($configTouched){if($null -ne $originalUpgradeBackup){[IO.File]::WriteAllBytes($upgradeBackupPath,$originalUpgradeBackup)}elseif(Test-Path $upgradeBackupPath){Remove-Item -LiteralPath $upgradeBackupPath}}
 $results|ConvertTo-Json -Depth 6|Set-Content -Encoding UTF8 (Join-Path $root 'appearance-integration-result.json')
}

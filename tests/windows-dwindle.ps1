$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$resultFile = Join-Path $root 'dwindle-result.json'
$results = [System.Collections.Generic.List[object]]::new()
$forms = @()
$app = $null
$shortcut = $null
$marker = Join-Path $root 'launcher-marker.txt'
$originalTaskbar = $null
$configTouched = $false
$configPath = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'OmarchyOverlay\config.json'
$originalConfig = if (Test-Path $configPath) { [IO.File]::ReadAllText($configPath) } else { $null }
$upgradeBackupPath=[IO.Path]::ChangeExtension($configPath,'pre-0.10.json')
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
  // 0.16 opens the Omarchy Overlay menu at startup; the 0.15 dashboard stays hidden until asked for.
  public static bool Ready(IntPtr main) { return main != IntPtr.Zero && (IsWindowVisible(main) || IsWindowVisible(Overlay())); }
  [DllImport("user32.dll", EntryPoint="PostMessageW")] static extern bool PostToApp(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
  public static void CloseMenu() { IntPtr menu = Overlay(); if (IsWindowVisible(menu)) PostToApp(menu, 0x100, (IntPtr)0x1b, IntPtr.Zero); }
  [DllImport("user32.dll", EntryPoint="ShowWindow")] static extern bool ShowAppWindow(IntPtr hwnd, int command);
  public static bool ShowDashboard(IntPtr main) { ShowAppWindow(main, 1); return IsWindowVisible(main); }
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
function Capture($hwnd,$name){$r=Bounds $hwnd;$b=[Drawing.Bitmap]::new(($r[2]-$r[0]),($r[3]-$r[1]));$g=[Drawing.Graphics]::FromImage($b);try{$g.CopyFromScreen($r[0],$r[1],0,0,$b.Size);$b.Save((Join-Path $root $name),[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$b.Dispose()}}

try {
 [void][Desktop]::SetProcessDpiAwarenessContext([IntPtr](-4))
 Check 'No controller to disturb' ([Desktop]::MainWindow() -eq [IntPtr]::Zero)
 $originalTaskbar=[Desktop]::TaskbarState()
 $excluded=@(Get-Process | Where-Object {$_.SessionId -eq [Diagnostics.Process]::GetCurrentProcess().SessionId -and $_.ProcessName -ne 'powershell'} | ForEach-Object {$_.ProcessName+'.exe'} | Select-Object -Unique)
 $config=@{layout='dwindle';excluded_processes=$excluded;start_tiling=$false;rules=@(@{executable='powershell.exe';mode='ignore'},@{executable='powershell.exe';window_class='OmarchyOverlay.ResizeFixture';mode='tile'})}
 [IO.File]::WriteAllText($configPath,($config|ConvertTo-Json -Depth 8),[Text.UTF8Encoding]::new($false));$configTouched=$true
 for($i=0;$i -lt 3;$i++){$windows+=,[ResizeTest]::Fixture($i)}
 Wait-Desktop 300;$originals=@{};foreach($h in $windows){$originals[$h.ToInt64()]=(Bounds $h)}
 $app=Start-Process "$root\OmarchyOverlay.exe" -PassThru
 Wait-Desktop 1600;$main=[Desktop]::MainWindow();[Desktop]::CloseMenu();Click $main 1;Wait-Desktop 1200
 $windows=@($windows|Sort-Object @{Expression={(Bounds $_)[0]}},@{Expression={(Bounds $_)[1]}})
 $before=@($windows|ForEach-Object{,(Bounds $_)})
 # Dwindle splits a pane along its longer side (src/dwindle.rs sync), so the right half stacks on a tall
 # screen and splits side by side on a wide one. Columns would give three equal thirds instead.
 $span=[Math]::Max($before[1][2],$before[2][2])-$before[0][0];$rightW=[Math]::Max($before[1][2],$before[2][2])-$before[1][0];$rightH=[Math]::Max($before[1][3],$before[2][3])-[Math]::Min($before[1][1],$before[2][1])
 $halfLeft=[Math]::Abs(($before[0][2]-$before[0][0])-$span/2) -le 20
 $rightSplit=if($rightW -ge $rightH){$before[1][0] -lt $before[2][0] -and [Math]::Abs($before[1][1]-$before[2][1]) -le 2}else{$before[1][1] -lt $before[2][1] -and [Math]::Abs($before[1][0]-$before[2][0]) -le 2}
 Check 'Dwindle initially splits into a left half and a right half split along its longer side' ($before[0][0] -lt $before[1][0] -and $halfLeft -and $rightSplit) ($before|ConvertTo-Json -Compress)
 [void][Desktop]::SetForegroundWindow($windows[1]);Wait-Desktop 800
 $fourth=[ResizeTest]::Fixture(3);Wait-Desktop 1400
 Check 'Adding a window preserves the unrelated first pane' (((Bounds $windows[0])-join ',') -eq ($before[0]-join ','))
 Check 'Adding a window preserves the unrelated lower pane' (((Bounds $windows[2])-join ',') -eq ($before[2]-join ','))
 Check 'New window splits the previously focused pane' (((Bounds $windows[1])-join ',') -ne ($before[1]-join ','))
 [void][ResizeTest]::DestroyWindow($fourth);Wait-Desktop 1400
 Check 'Closing a window collapses its split' (((Bounds $windows[1])-join ',') -eq ($before[1]-join ','))
 [void][Desktop]::SetForegroundWindow($windows[0]);[Desktop]::Shortcut(0x53);Wait-Desktop 900
 Check 'Flip split hotkey changes root orientation' ([Math]::Abs((Bounds $windows[0])[0]-(Bounds $windows[1])[0]) -le 2)
 [Desktop]::Shortcut(0x53);Wait-Desktop 900
 Check 'Flipping again restores vertical geometry' (((Bounds $windows[0])-join ',') -eq ($before[0]-join ','))
 [Desktop]::Shortcut(0xdd);Wait-Desktop 1000
 Check 'Existing grow shortcut adjusts the focused dwindle split' ((Bounds $windows[0])[2] -gt $before[0][2]+20)
 [ResizeTest]::Drag($windows[0],$windows[1],$true,65);Wait-Desktop 1500
 Check 'Dwindle shared edge resizes its neighbor live' ([ResizeTest]::LiveChanged)
 $resized=Bounds $windows[0];Wait-Desktop 1000
 Check 'Dwindle retains native mouse resizing across timer ticks' (((Bounds $windows[0])-join ',') -eq ($resized-join ','))
 [void][Desktop]::ShowDashboard($main);Wait-Desktop 400;Capture $main 'dwindle-dashboard.png'
 Click $main 1
 for($i=0;$i -lt 3;$i++){Check "Pause restores fixture $i" (((Bounds $windows[$i])-join ',') -eq ($originals[$windows[$i].ToInt64()]-join ','))}
} catch {$results.Add(@{name='Dwindle harness';passed=$false;detail=$_.Exception.ToString()})}
finally{
 if($null -ne $app -and -not $app.HasExited){[void][Desktop]::PostMessage([Desktop]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero);Wait-Desktop 1500}
 foreach($hwnd in $windows){[void][ResizeTest]::DestroyWindow($hwnd)}
 if($null -ne $originalTaskbar){[Desktop]::SetTaskbarState($originalTaskbar)}
 if($configTouched){if($null -ne $originalConfig){[IO.File]::WriteAllText($configPath,$originalConfig,[Text.UTF8Encoding]::new($false))}elseif(Test-Path $configPath){Remove-Item $configPath}}
 if($configTouched){
  if($null -ne $originalUpgradeBackup){[IO.File]::WriteAllBytes($upgradeBackupPath,$originalUpgradeBackup)}
  elseif(Test-Path $upgradeBackupPath){Remove-Item -LiteralPath $upgradeBackupPath}
 }
 $results|ConvertTo-Json -Depth 5|Set-Content -Encoding UTF8 $resultFile
}

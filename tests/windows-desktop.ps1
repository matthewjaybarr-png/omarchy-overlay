$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$resultFile = Join-Path $root 'desktop-result.json'
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
using System; using System.Text; using System.Runtime.InteropServices;
public static class TasksTest {
 delegate bool EnumProc(IntPtr hwnd,IntPtr data);
 [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback,IntPtr data);
 [DllImport("user32.dll")] static extern IntPtr GetWindow(IntPtr hwnd,uint command);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd,StringBuilder text,int max);
 [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] static extern IntPtr ReadText(IntPtr hwnd,uint message,IntPtr count,StringBuilder text);
 public static string Tooltip(IntPtr bar){IntPtr tooltip=IntPtr.Zero;EnumProc callback=(hwnd,data)=>{var name=new StringBuilder(256);GetClassName(hwnd,name,256);if(name.ToString()=="tooltips_class32"&&GetWindow(hwnd,4)==bar)tooltip=hwnd;return true;};EnumWindows(callback,IntPtr.Zero);var text=new StringBuilder(4096);if(tooltip!=IntPtr.Zero)ReadText(tooltip,0x0d,(IntPtr)4096,text);return text.ToString();}
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr CreateWindowEx(uint ex,string cls,string title,uint style,int x,int y,int w,int h,IntPtr parent,IntPtr menu,IntPtr instance,IntPtr data);
 [DllImport("user32.dll")] public static extern bool DestroyWindow(IntPtr hwnd);
 public static IntPtr Fixture(){return CreateWindowEx(0,"STATIC","KS07 native restore",0x10cf0000,360,260,520,360,IntPtr.Zero,IntPtr.Zero,IntPtr.Zero,IntPtr.Zero);}
 [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr hwnd,int command);
 [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
 public static void Top(IntPtr hwnd,bool top){SetWindowPos(hwnd,(IntPtr)(top?-1:-2),0,0,0,0,0x53);}
 [DllImport("user32.dll")] static extern IntPtr OpenInputDesktop(uint flags,bool inherit,uint access);
 [DllImport("user32.dll")] static extern bool CloseDesktop(IntPtr desktop);
 public static bool InputAvailable(){var desktop=OpenInputDesktop(0,false,1);if(desktop==IntPtr.Zero)return false;CloseDesktop(desktop);return true;}
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string cls,string title);
 public static IntPtr Find(string cls){return FindWindow(cls,null);}
 [DllImport("user32.dll")] public static extern bool IsIconic(IntPtr hwnd);
 [DllImport("user32.dll")] public static extern IntPtr GetFocus();
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd,StringBuilder text,int max);
 public static string Text(IntPtr hwnd){var text=new StringBuilder(2048);GetWindowText(hwnd,text,2048);return text.ToString();}
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] public static extern void mouse_event(uint flags,uint x,uint y,uint data,UIntPtr extra);
 public static void Click(int x,int y){SetCursorPos(x,y);mouse_event(2,0,0,0,UIntPtr.Zero);mouse_event(4,0,0,0,UIntPtr.Zero);}
}
'@
$nativeFixture=[IntPtr]::Zero
function Focus-Panel($hwnd){
 [TasksTest]::Top($hwnd,$true);$r=Bounds $hwnd;[TasksTest]::Click(($r[0]+80),($r[1]+20));Wait-Desktop 250;[TasksTest]::Top($hwnd,$false)
}
function Key([byte]$code){[Desktop]::keybd_event($code,0,0,[UIntPtr]::Zero);[Desktop]::keybd_event($code,0,2,[UIntPtr]::Zero);Wait-Desktop 500}
function Capture($hwnd,$name){
 $r=Bounds $hwnd
 $bitmap=[Drawing.Bitmap]::new(($r[2]-$r[0]),($r[3]-$r[1]));$g=[Drawing.Graphics]::FromImage($bitmap)
 try{$g.CopyFromScreen($r[0],$r[1],0,0,$bitmap.Size);$bitmap.Save((Join-Path $root $name),[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$bitmap.Dispose()}
}
try {
 [void][Desktop]::SetProcessDpiAwarenessContext([IntPtr](-4))
 $principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
 Check 'Runs with a limited token' (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))
 Check 'Interactive desktop is unlocked (unlock VM before running)' ([TasksTest]::InputAvailable())
 Check 'No existing controller to disturb' ([Desktop]::MainWindow() -eq [IntPtr]::Zero)
 [IO.Directory]::CreateDirectory((Split-Path $configPath))|Out-Null
 $fixtureExcluded=@('Taskmgr.exe','mstsc.exe','msrdc.exe','ApplicationFrameHost.exe','vmconnect.exe')+@(Get-Process | Where-Object {$_.SessionId -eq [Diagnostics.Process]::GetCurrentProcess().SessionId -and $_.ProcessName -ne 'powershell'} | ForEach-Object {$_.ProcessName+'.exe'} | Select-Object -Unique)
$fixtureDefaults=(@{excluded_processes=$fixtureExcluded;start_tiling=$false}|ConvertTo-Json -Depth 5)
[IO.File]::WriteAllText($configPath,$fixtureDefaults,[Text.UTF8Encoding]::new($false));$configTouched=$true
 Key 0x10
 $originalTaskbar=[Desktop]::TaskbarState()
 for($i=0;$i -lt 3;$i++){
  $form=[Windows.Forms.Form]::new();$form.Text="KS07 fixture $i";$form.StartPosition='Manual';$form.SetBounds((120+$i*100),(120+$i*70),560,380);$form.Show();$forms+=$form
 }
 Wait-Desktop 500
 $originals=@($forms|ForEach-Object { ,(Bounds $_.Handle) })
 $app=Start-Process (Join-Path $root 'OmarchyOverlay.exe') -PassThru
 $deadline=[DateTime]::UtcNow.AddSeconds(30)
 do {Wait-Desktop 150;$main=[Desktop]::MainWindow()} while (-not [Desktop]::Ready($main) -and -not $app.HasExited -and [DateTime]::UtcNow -lt $deadline)
 $bar=[Desktop]::Bar()
 Check 'Startup opens the Omarchy Overlay menu' ([Desktop]::IsWindowVisible([Desktop]::Overlay()))
 [Desktop]::CloseMenu();Wait-Desktop 400
 Check 'Escape closes the Omarchy Overlay menu' (-not [Desktop]::IsWindowVisible([Desktop]::Overlay()))
 Check 'The 0.15 dashboard still opens' ([Desktop]::ShowDashboard($main));Wait-Desktop 300
 Focus-Panel $main
 Capture $main 'overlay-dashboard.png'
 Click $main 5
 $help=[TasksTest]::Find('OmarchyOverlay.Help')
 Check 'Dedicated shortcut reference opens' ([Desktop]::IsWindowVisible($help)) "main=$main help=$help exited=$($app.HasExited)"
 [void][Desktop]::SendMessage($help,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 # Verify the MENU action with controls initially hidden.
 [void][Desktop]::SendMessage($main,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 [void][Desktop]::PostMessage($bar,0x202,[IntPtr]::Zero,[IntPtr](20+(10 -shl 16)));Wait-Desktop 500
 Check 'MENU opens the Omarchy Overlay menu' ([Desktop]::IsWindowVisible([Desktop]::Overlay()))
 [Desktop]::CloseMenu();Wait-Desktop 300;[void][Desktop]::ShowDashboard($main)
 [Desktop]::Shortcut(0x57);Wait-Desktop 800
 $tasks=[TasksTest]::Find('OmarchyOverlay.Tasks');$list=[Desktop]::GetDlgItem($tasks,302)
 Check 'Open windows shortcut opens switcher' ([Desktop]::IsWindowVisible($tasks))
 Input-Text $tasks 301 'KS07 fixture';Wait-Desktop 250
 Check 'Search finds all fixture windows' ([Desktop]::SendMessage($list,0x18b,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32() -eq 3)
 Input-Text $tasks 301 'KS07 fixture 1';Wait-Desktop 200
 Check 'Search uses all words' ([Desktop]::SendMessage($list,0x18b,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32() -eq 1)
 Click $tasks 305
 Check 'Minimize button minimizes selected app' ([TasksTest]::IsIconic($forms[1].Handle))
 Check 'Minimized app remains searchable' ([Desktop]::SendMessage($list,0x18b,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32() -eq 1)
 Focus-Panel $tasks
 Capture $tasks 'overlay-open-windows.png'
 Key 0x0d
 Check 'Enter restores and activates minimized app' (-not [TasksTest]::IsIconic($forms[1].Handle) -and [Desktop]::GetForegroundWindow() -eq $forms[1].Handle) "min=$([TasksTest]::IsIconic($forms[1].Handle)) foreground=$([Desktop]::GetForegroundWindow()) target=$($forms[1].Handle)"
 [Desktop]::Shortcut(0x57);Wait-Desktop 700
 Input-Text $tasks 301 'no-such-ks07-window';Wait-Desktop 200
 Check 'No-match status reports zero windows' ([TasksTest]::Text([Desktop]::GetDlgItem($tasks,303)).StartsWith('0 window'))
 Input-Text $tasks 301 'KS07 fixture 2';Wait-Desktop 200
 $forms[2].Text='KS07 changed title';Wait-Desktop 1000
 Check 'Title changes refresh the live search' ([TasksTest]::Text([Desktop]::GetDlgItem($tasks,303)).StartsWith('0 window'))
 Input-Text $tasks 301 'KS07 changed';Wait-Desktop 200
 $forms[2].Close();Wait-Desktop 1000
 Check 'Closed apps disappear from switcher' ([TasksTest]::Text([Desktop]::GetDlgItem($tasks,303)).StartsWith('0 window'))
 [void][Desktop]::SendMessage($tasks,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 Click $main 1;Wait-Desktop 900
 [void][Desktop]::SetForegroundWindow($forms[1].Handle);Wait-Desktop 900
 [Desktop]::Shortcut(0x4d);Wait-Desktop 900
 $a=Bounds $forms[0].Handle;$b=Bounds $forms[1].Handle
 Check 'Make-main shortcut promotes focused window' ($b[0] -lt $a[0] -and ($b[2]-$b[0]) -gt ($a[2]-$a[0]))
 [Desktop]::Shortcut(0x56);Wait-Desktop 900
 $config=Get-Content -Raw $configPath|ConvertFrom-Json
 Check 'Cycle-layout shortcut saves columns' ($config.layout -eq 'columns')
 [Desktop]::Shortcut(0x56);Wait-Desktop 900
 $a=Bounds $forms[0].Handle;$b=Bounds $forms[1].Handle
 Check 'Cycle-layout shortcut arranges rows' ($a[0] -eq $b[0] -and $a[1] -ne $b[1])
 [Desktop]::Shortcut(0x56);Wait-Desktop 250
 [Desktop]::Shortcut(0x56);Wait-Desktop 700
 for($iteration=0;$iteration -lt 8;$iteration++){[Desktop]::Shortcut(0x56);Wait-Desktop 200}
 $area=[Desktop]::WorkArea();$fits=$true
 foreach($form in @($forms[0],$forms[1])){$bounds=Bounds $form.Handle;$fits=$fits -and ($bounds[0] -ge $area.left-64 -and $bounds[1] -ge $area.top-64 -and $bounds[2] -le $area.right+64 -and $bounds[3] -le $area.bottom+64)}
 Check 'Rapid layout changes do not inflate windows beyond the work area' $fits
 [void][Desktop]::SetForegroundWindow($forms[1].Handle);Wait-Desktop 800
 Click $main 17
 Check 'Dashboard floats remembered app' (((Bounds $forms[1].Handle)-join ',') -eq ($originals[1]-join ','))
 Click $main 17
 Check 'Dashboard returns app to tiling' (((Bounds $forms[1].Handle)-join ',') -ne ($originals[1]-join ','))
 # A plain native fixture honors WINDOWPLACEMENT rather than caching its own restore size.
 $nativeFixture=[TasksTest]::Fixture();$nativeOriginal=Bounds $nativeFixture;Wait-Desktop 1000
 [void][TasksTest]::ShowWindowAsync($nativeFixture,6);Wait-Desktop 800
 Click $main 1
 Check 'Pausing leaves user-minimized app minimized' ([TasksTest]::IsIconic($nativeFixture))
 [void][TasksTest]::ShowWindowAsync($nativeFixture,9);Wait-Desktop 700
 Check 'Minimized native app retains original normal placement' (((Bounds $nativeFixture)-join ',') -eq ($nativeOriginal-join ',')) "actual=$((Bounds $nativeFixture)-join ',') expected=$($nativeOriginal-join ',')"
 [void][TasksTest]::DestroyWindow($nativeFixture);$nativeFixture=[IntPtr]::Zero
 # Isolate tasks; locate the fixture by its tooltip, not by the number of test consoles.
 Click $main 14;$settings=[TasksTest]::Find('OmarchyOverlay.BarSettings')
 Input-Text $settings 201 'app-drawer, tasks';Input-Text $settings 202 '';Input-Text $settings 203 '';Input-Text $settings 210 '8';Click $settings 211
 [void][Desktop]::SendMessage($settings,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 Wait-Desktop 900
 $bar=[Desktop]::Bar();$r=Bounds $bar;$scale=($r[3]-$r[1])/30
 [TasksTest]::Click(($r[0]+30*$scale),($r[1]+15*$scale));Wait-Desktop 700
 Check 'Open-window drawer opens monitor window list' ([Desktop]::IsWindowVisible($tasks))
 Check 'Drawer list reports monitor scope' ([TasksTest]::Text([Desktop]::GetDlgItem($tasks,303)).Contains('this monitor'))
 [void][Desktop]::SendMessage($tasks,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 Focus-Panel $main
 Capture $bar 'overlay-taskbar.png'
 $buttonX=$null
 for($x=20;$x -lt [int](($r[2]-$r[0])/$scale/2);$x+=32){
  [void][TasksTest]::SetCursorPos(($r[0]+[int]($x*$scale)),($r[1]+[int](15*$scale)));Wait-Desktop 550
  if([TasksTest]::Tooltip($bar).Contains('KS07 fixture 1')){$buttonX=$r[0]+[int]($x*$scale);break}
 }
 Check 'Tooltip identifies the disposable fixture button' ($null -ne $buttonX)
 [TasksTest]::Click($buttonX,($r[1]+15*$scale));Wait-Desktop 700
 $target=[Desktop]::GetForegroundWindow()
 Check 'Running-app bar button activates fixture' (@($forms[0].Handle,$forms[1].Handle).Contains($target)) "foreground=$target title=$([TasksTest]::Text($target))"
 [TasksTest]::Click($buttonX,($r[1]+15*$scale));Wait-Desktop 700
 Check 'Clicking active app button minimizes it' ([TasksTest]::IsIconic($target)) ("target=$target foreground=$([Desktop]::GetForegroundWindow()) cursor=$buttonX scale=$scale tooltip=$([TasksTest]::Tooltip($bar)) bounds=$((Bounds $bar)-join ',')")
 [TasksTest]::Click($buttonX,($r[1]+15*$scale));Wait-Desktop 700
 Check 'Clicking minimized app button restores it' (-not [TasksTest]::IsIconic($target) -and [Desktop]::GetForegroundWindow() -eq $target)
 Capture $bar 'overlay-taskbar.png'
 [Desktop]::Shortcut(0x1b);Wait-Desktop 1400
 Check 'Exit closes controller' $app.HasExited
 Check 'Exit restores Windows taskbar preference' ([Desktop]::TaskbarState() -eq $originalTaskbar)
} catch {$results.Add(@{name='Test harness';passed=$false;detail=$_.Exception.ToString()})}
finally {
 if($null -ne $app -and -not $app.HasExited){[void][Desktop]::PostMessage([Desktop]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero);Wait-Desktop 1500}
 if($nativeFixture -ne [IntPtr]::Zero){[void][TasksTest]::DestroyWindow($nativeFixture)}
 foreach($form in $forms){$form.Close();$form.Dispose()}
 if($null -ne $originalTaskbar){[Desktop]::SetTaskbarState($originalTaskbar)}
 if($configTouched){if($null -ne $originalConfig){[IO.File]::WriteAllText($configPath,$originalConfig,[Text.UTF8Encoding]::new($false))}elseif(Test-Path $configPath){Remove-Item $configPath}}
 if($configTouched){
  if($null -ne $originalUpgradeBackup){[IO.File]::WriteAllBytes($upgradeBackupPath,$originalUpgradeBackup)}
  elseif(Test-Path $upgradeBackupPath){Remove-Item -LiteralPath $upgradeBackupPath}
 }
 $results|ConvertTo-Json -Depth 5|Set-Content -Encoding UTF8 $resultFile
}

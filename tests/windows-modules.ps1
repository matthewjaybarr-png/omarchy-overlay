$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$resultFile = Join-Path $root 'modules-result.json'
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
using System;using System.Runtime.InteropServices;
public static class ModulesTest {
 [DllImport("user32.dll")] static extern void mouse_event(uint flags,uint x,uint y,uint data,UIntPtr extra);
 public static void Click(int x,int y){SetCursorPos(x,y);mouse_event(2,0,0,0,UIntPtr.Zero);mouse_event(4,0,0,0,UIntPtr.Zero);}
 [StructLayout(LayoutKind.Sequential)] struct Point{public int x,y;}
 [DllImport("user32.dll")] static extern bool GetCursorPos(out Point point);
 [DllImport("user32.dll")] static extern IntPtr WindowFromPoint(Point point);
 public static string CursorWindow(){Point p;GetCursorPos(out p);var hwnd=WindowFromPoint(p);var name=new System.Text.StringBuilder(256);GetClassName(hwnd,name,256);return p.x+","+p.y+" hwnd="+hwnd+" class="+name;}
 public delegate bool EnumProc(IntPtr hwnd,IntPtr data);
 [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback,IntPtr data);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string cls,string title);
 public static IntPtr Find(string cls){return FindWindow(cls,null);}
 [DllImport("user32.dll")] static extern IntPtr GetWindow(IntPtr hwnd,uint cmd);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr hwnd,System.Text.StringBuilder name,int max);
 public static IntPtr Tooltip(IntPtr bar){IntPtr found=IntPtr.Zero;EnumProc callback=(w,d)=>{var name=new System.Text.StringBuilder(256);GetClassName(w,name,256);if(name.ToString()=="tooltips_class32"&&GetWindow(w,4)==bar)found=w;return true;};EnumWindows(callback,IntPtr.Zero);return found;}
 [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] static extern IntPtr ReadText(IntPtr hwnd,uint msg,IntPtr count,System.Text.StringBuilder text);
 public static string Tip(IntPtr tooltip,IntPtr bar){var text=new System.Text.StringBuilder(4096);ReadText(tooltip,0x0d,(IntPtr)4096,text);return text.ToString();}
 [DllImport("user32.dll")] public static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr hwnd,out uint pid);
 [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtr(IntPtr hwnd,int index);
 [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr hwnd,IntPtr after,int x,int y,int w,int h,uint flags);
 public static void Top(IntPtr hwnd){SetWindowPos(hwnd,(IntPtr)(-1),0,0,0,0,0x53);}
}
'@
function Capture($hwnd,$name){$r=Bounds $hwnd;$b=[Drawing.Bitmap]::new(($r[2]-$r[0]),($r[3]-$r[1]));$g=[Drawing.Graphics]::FromImage($b);try{$g.CopyFromScreen($r[0],$r[1],0,0,$b.Size);$b.Save((Join-Path $root $name),[Drawing.Imaging.ImageFormat]::Png)}finally{$g.Dispose();$b.Dispose()}}
function Bar-Click([int]$x){$bar=[Desktop]::Bar();$r=Bounds $bar;$scale=($r[3]-$r[1])/30;[void][Desktop]::PostMessage($bar,0x202,[IntPtr]::Zero,[IntPtr]([int]($x*$scale)+([int](14*$scale) -shl 16)));Wait-Desktop 650}
try {
 [void][Desktop]::SetProcessDpiAwarenessContext([IntPtr](-4))
 Check 'No controller to disturb' ([Desktop]::MainWindow()-eq [IntPtr]::Zero)
 [IO.Directory]::CreateDirectory((Split-Path $configPath))|Out-Null
 $fixtureExcluded=@('Taskmgr.exe','mstsc.exe','msrdc.exe','ApplicationFrameHost.exe','vmconnect.exe')+@(Get-Process | Where-Object {$_.SessionId -eq [Diagnostics.Process]::GetCurrentProcess().SessionId -and $_.ProcessName -ne 'powershell'} | ForEach-Object {$_.ProcessName+'.exe'} | Select-Object -Unique)
$fixtureDefaults=(@{excluded_processes=$fixtureExcluded;start_tiling=$false}|ConvertTo-Json -Depth 5)
[IO.File]::WriteAllText($configPath,$fixtureDefaults,[Text.UTF8Encoding]::new($false));$configTouched=$true
 $originalTaskbar=[Desktop]::TaskbarState()
 $app=Start-Process (Join-Path $root 'OmarchyOverlay.exe') -PassThru
 $deadline=[DateTime]::UtcNow.AddSeconds(30)
 do{Wait-Desktop 200;$main=[Desktop]::MainWindow()}while(-not [Desktop]::Ready($main)-and [DateTime]::UtcNow -lt $deadline)
 Check 'Controller opens with legacy config' ([Desktop]::Ready($main));[Desktop]::CloseMenu()
 Click $main 14;$settings=[ModulesTest]::Find('OmarchyOverlay.BarSettings');Click $settings 214
 $modules=[ModulesTest]::Find('OmarchyOverlay.ModuleSettings')
 Check 'Module appearance editor opens from Bar settings' ([Desktop]::IsWindowVisible($modules))
 Check 'Config without bar settings gets the icon menu' ([Desktop]::SendMessage([Desktop]::GetDlgItem($modules,403),0x147,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32()-eq 2)
 Check 'Config without bar settings gets icon-only app buttons' ([Desktop]::SendMessage([Desktop]::GetDlgItem($modules,407),0x147,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32()-eq 2)
 Choose $modules 403 2;Choose $modules 402 1;Choose $modules 410 1;Choose $modules 409 0;Choose $modules 407 2;Click $modules 451
 $saved=Get-Content -Raw $configPath|ConvertFrom-Json
 Check 'Independent module display choices save' ($saved.bar.module_display.menu -eq 'icon' -and $saved.bar.module_display.sound -eq 'icon-text' -and $saved.bar.module_display.clock -eq 'text' -and $saved.bar.module_display.tasks -eq 'icon')
 [ModulesTest]::Top($modules);Wait-Desktop 300;Capture $modules 'overlay-module-settings.png'
 [void][Desktop]::SendMessage($modules,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 Input-Text $settings 201 'menu, launcher, start';Input-Text $settings 202 'clock';Input-Text $settings 203 'sound, network, power, controls';Click $settings 211
 $saved=Get-Content -Raw $configPath|ConvertFrom-Json
 Check 'Applying bar layout preserves display choices' ($saved.bar.module_display.menu -eq 'icon' -and $saved.bar.module_display.sound -eq 'icon-text')
 [void][Desktop]::SendMessage($settings,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 [void][Desktop]::SendMessage($main,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 Bar-Click 20
 Check 'Icon-only menu retains its click action' ([Desktop]::IsWindowVisible([Desktop]::Overlay()))
 [Desktop]::CloseMenu();Wait-Desktop 300
 # 0.16: the launcher module opens the Omarchy Overlay app picker.
 Bar-Click 65;$overlayDeadline=[DateTime]::UtcNow.AddSeconds(5)
 while(-not [Desktop]::IsWindowVisible([Desktop]::Overlay()) -and [DateTime]::UtcNow -lt $overlayDeadline){Wait-Desktop 100}
 Check 'Launcher module opens the Omarchy Overlay app picker' ([Desktop]::IsWindowVisible([Desktop]::Overlay()))
 [ModulesTest]::Top([Desktop]::Overlay());Wait-Desktop 700;Capture ([Desktop]::Overlay()) 'overlay-overlay-picker.png'
 [Desktop]::CloseMenu();Wait-Desktop 300
 # The 0.15 search window, still opened from the dashboard, keeps its owner-drawn icon rows.
 [void][Desktop]::PostMessage($main,0x0111,[IntPtr]2,[IntPtr]::Zero);$picker=[Desktop]::Launcher();$searchDeadline=[DateTime]::UtcNow.AddSeconds(5)
 while(-not [Desktop]::IsWindowVisible($picker) -and [DateTime]::UtcNow -lt $searchDeadline){Wait-Desktop 100;$picker=[Desktop]::Launcher()}
 $list=[Desktop]::GetDlgItem($picker,11)
 Check '0.15 app search opens from the dashboard' ([Desktop]::IsWindowVisible($picker))
 Check 'App picker uses icon-capable owner-drawn rows' (([ModulesTest]::GetWindowLongPtr($list,-16).ToInt64()-band 0x10)-ne 0)
 Check 'Owner-drawn rows retain searchable native strings' ([Desktop]::SendMessage($list,0x18b,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32()-gt 5)
 [ModulesTest]::Top($picker);Wait-Desktop 700;Capture $picker 'overlay-picker-icons.png'
 [void][Desktop]::SendMessage($picker,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 $bar=[Desktop]::Bar();$r=Bounds $bar;$scale=($r[3]-$r[1])/30
 [void][ModulesTest]::SetCursorPos(($r[0]+[int](20*$scale)),($r[1]+[int](14*$scale)));Wait-Desktop 1300
 $tooltip=[ModulesTest]::Tooltip($bar)
 Check 'Bar owns a tooltip window' ($tooltip-ne [IntPtr]::Zero)
 $natural=[Desktop]::IsWindowVisible($tooltip)
 $diagnostic="natural=$natural bar=$bar tooltip=$tooltip tools=$([Desktop]::SendMessage($tooltip,1037,[IntPtr]::Zero,[IntPtr]::Zero)) cursor=$([ModulesTest]::CursorWindow()) text=$([ModulesTest]::Tip($tooltip,$bar))"
 if(-not $natural){[void][Desktop]::PostMessage($bar,0x200,[IntPtr]::Zero,[IntPtr](24+(16 -shl 16)));Wait-Desktop 700;$diagnostic+=" after-move=$([Desktop]::IsWindowVisible($tooltip)) text=$([ModulesTest]::Tip($tooltip,$bar))";[void][Desktop]::PostMessage($bar,0x2a1,[IntPtr]::Zero,[IntPtr](24+(16 -shl 16)));Wait-Desktop 300;$diagnostic+=" after-hover=$([Desktop]::IsWindowVisible($tooltip))"}
 Check 'Hover shows a visible native tooltip' $natural $diagnostic
 Capture $tooltip 'overlay-tooltip.png'
 $tip=[ModulesTest]::Tip($tooltip,$bar)
 Check 'Icon-only menu tooltip explains its action' ($tip.Contains('Omarchy Overlay menu')) $tip
 Capture $bar 'overlay-module-bar.png'
 # Start is the third item: icon menu 28 + gap, launcher icon/text 72 + gap.
 [ModulesTest]::Click(($r[0]+[int](145*$scale)),($r[1]+[int](14*$scale)));Wait-Desktop 1500
 $pidValue=0;[void][ModulesTest]::GetWindowThreadProcessId([Desktop]::GetForegroundWindow(),[ref]$pidValue)
 $process=Get-Process -Id $pidValue -ErrorAction SilentlyContinue
 Capture ([Desktop]::GetForegroundWindow()) 'overlay-windows-start.png'
 Check 'Start module opens the native Windows Start shell' ($process.ProcessName -in @('StartMenuExperienceHost','SearchHost')) $process.ProcessName
 [Desktop]::keybd_event(0x1b,0,0,[UIntPtr]::Zero);[Desktop]::keybd_event(0x1b,0,2,[UIntPtr]::Zero);Wait-Desktop 400
 [void][Desktop]::PostMessage($main,0x312,[IntPtr]105,[IntPtr]::Zero);Wait-Desktop 1200
 Check 'Exit removes bar and its tooltip' (-not [Desktop]::IsWindowVisible($tooltip))
 $app=Start-Process (Join-Path $root 'OmarchyOverlay.exe') -PassThru
 $deadline=[DateTime]::UtcNow.AddSeconds(30)
 do{Wait-Desktop 200;$main=[Desktop]::MainWindow()}while(-not [Desktop]::Ready($main)-and [DateTime]::UtcNow -lt $deadline);[Desktop]::CloseMenu()
 Click $main 14;$settings=[ModulesTest]::Find('OmarchyOverlay.BarSettings');Click $settings 214;$modules=[ModulesTest]::Find('OmarchyOverlay.ModuleSettings')
 Check 'Module choices survive restarting Omarchy Overlay' ([Desktop]::SendMessage([Desktop]::GetDlgItem($modules,403),0x147,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32()-eq 2)
 Click $modules 452;Click $modules 451
 $saved=Get-Content -Raw $configPath|ConvertFrom-Json
 Check 'Restore defaults sets icons except the clock' ($saved.bar.module_display.menu -eq 'icon' -and $saved.bar.module_display.tasks -eq 'icon' -and $saved.bar.module_display.clock -eq 'text')
} catch{$results.Add(@{name='Module/picker harness';passed=$false;detail=$_.Exception.ToString()})}
finally{
 if($null -ne $app -and -not $app.HasExited){[void][Desktop]::PostMessage([Desktop]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero);Wait-Desktop 1500}
 if($null -ne $originalTaskbar){[Desktop]::SetTaskbarState($originalTaskbar)}
 if($configTouched){if($null -ne $originalConfig){[IO.File]::WriteAllText($configPath,$originalConfig,[Text.UTF8Encoding]::new($false))}elseif(Test-Path $configPath){Remove-Item $configPath}}
 if($configTouched){
  if($null -ne $originalUpgradeBackup){[IO.File]::WriteAllBytes($upgradeBackupPath,$originalUpgradeBackup)}
  elseif(Test-Path $upgradeBackupPath){Remove-Item -LiteralPath $upgradeBackupPath}
 }
 $results|ConvertTo-Json -Depth 5|Set-Content -Encoding UTF8 $resultFile
}

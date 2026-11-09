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

  public static IntPtr ByClass(string cls) { return FindWindow(cls, null); }
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


function Wait-Panel([string]$class){
 $until=[DateTime]::UtcNow.AddSeconds(8)
 do{$window=[Desktop]::ByClass($class);if([Desktop]::IsWindowVisible($window)){return $window};Wait-Desktop 100}while([DateTime]::UtcNow -lt $until)
 return $window
}
function Key-Action([int]$id){[void][Desktop]::PostMessage($main,0x312,[IntPtr](100+$id),[IntPtr]::Zero);Wait-Desktop 900}
$local=Split-Path $configPath
$terminalPath=Join-Path (Split-Path $local) 'Microsoft\Windows Terminal\Fragments\OmarchyOverlay\appearance.json'
$terminalBefore=if(Test-Path $terminalPath){[IO.File]::ReadAllText($terminalPath)}else{$null}
$wallpaperBefore=[Desktop]::Wallpaper()
try {
 [void][Desktop]::SetProcessDpiAwarenessContext([IntPtr](-4))
 Check 'No controller to disturb' ([Desktop]::MainWindow() -eq [IntPtr]::Zero)
 Check 'No pending appearance transaction to disturb' (-not (Test-Path "$local\wallpaper-restore.json") -and -not (Test-Path "$local\terminal-appearance-restore.json"))
 $originalTaskbar=[Desktop]::TaskbarState()
 $excluded=@(Get-Process|Where-Object {$_.SessionId -eq [Diagnostics.Process]::GetCurrentProcess().SessionId -and $_.ProcessName -ne 'powershell'}|ForEach-Object {$_.ProcessName+'.exe'}|Select-Object -Unique)
 $config=@{excluded_processes=$excluded;start_tiling=$false;rules=@(@{executable='powershell.exe';mode='ignore'},@{executable='powershell.exe';window_class='OmarchyOverlay.ResizeFixture';mode='tile'})}
 [IO.File]::WriteAllText($configPath,($config|ConvertTo-Json -Depth 8),[Text.UTF8Encoding]::new($false));$configTouched=$true
 for($i=0;$i -lt 2;$i++){$windows+=,[ResizeTest]::Fixture($i)};Wait-Desktop 300
 $app=Start-Process "$root\OmarchyOverlay.exe" -PassThru;Wait-Desktop 1800;$main=[Desktop]::MainWindow();[Desktop]::CloseMenu();Click $main 1
 $windows=@($windows|Sort-Object{(Bounds $_)[0]});$before=Bounds $windows[0]
 [void][Desktop]::SetForegroundWindow($windows[0]);Key-Action 47
 Check 'Keyboard resizing changes an internal split' ((Bounds $windows[0])[2] -gt $before[2]+15)
 $resized=Bounds $windows[0];Wait-Desktop 1000
 Check 'Keyboard split persists across the tiling timer' (((Bounds $windows[0])-join ',') -eq ($resized-join ','))
 [void][Desktop]::SetForegroundWindow($windows[0]);[Desktop]::Shortcut(0x58);Wait-Desktop 800
 Check 'Maximize shortcut maximizes the focused eligible window' ([Desktop]::IsZoomed($windows[0]))
 [Desktop]::Shortcut(0x58);Wait-Desktop 800
 Check 'Maximize shortcut restores the window on its second invocation' (-not [Desktop]::IsZoomed($windows[0]))
 [void][Desktop]::SetForegroundWindow($windows[0]);Key-Action 54
 $scratch=Bounds $windows[0];Key-Action 55
 Check 'Scratchpad toggle uses normal Windows minimization' ([Desktop]::IsIconic($windows[0]))
 Key-Action 55
 Check 'Scratchpad toggle restores the utility window' (-not [Desktop]::IsIconic($windows[0]))
 Check 'Scratchpad preserves its original floating placement' (((Bounds $windows[0])-join ',') -eq ($scratch-join ','))
 for($i=0;$i -lt 4;$i++) {
  [void][Desktop]::SetForegroundWindow($windows[0]);Key-Action (50+$i)
  $expected=@(217,230,242,250)[$i]
  Check "Opacity preset $i applies its native alpha" ([Math]::Abs([Desktop]::Alpha($windows[0])-$expected)-le 1)
  Key-Action (50+$i)
  Check "Opacity preset $i restores the original opaque state" (-not [Desktop]::Layered($windows[0]))
 }
 $close=[ResizeTest]::Fixture(9)
 # Let automatic tiling place the new window before clicking its caption.
 Wait-Desktop 1200
 [void][Desktop]::SetForegroundWindow($close);Wait-Desktop 200
 Check 'Close fixture has focus before sending the shortcut' ([Desktop]::GetForegroundWindow() -eq $close)
 [Desktop]::Shortcut(0x43);Wait-Desktop 700
 Check 'Close shortcut sends a normal window-close request' (-not [Desktop]::IsWindow($close)) ("fixture=$close; foreground=$([Desktop]::GetForegroundWindow())")
 Click $main 8;Click ([Desktop]::Appearance()) 38;$appearance=Wait-Panel 'OmarchyOverlay.AppearanceProfiles'
 Check 'Appearance profiles panel opens' ([Desktop]::IsWindowVisible($appearance)) ("main=$main original=$([Desktop]::Appearance()) profiles=$appearance exited=$($app.HasExited)")
 [void][Desktop]::SendMessage([Desktop]::GetDlgItem($appearance,801),0x186,[IntPtr]1,[IntPtr]::Zero)
 Click $appearance (801+(1 -shl 16));Click $appearance 808
 $saved=Get-Content $configPath -Raw|ConvertFrom-Json
 Check 'Applying a profile coordinates palette, gaps and focus feedback' ($saved.theme -eq 'tokyo-night' -and $saved.gap -eq 8 -and $saved.focus_outline)
 Input-Text $appearance 802 'Work Tokyo';Click $appearance 809
 Check 'Named appearance profile saves with the portable preferences' ((Get-Content $configPath -Raw|ConvertFrom-Json).appearance_profiles[0].name -eq 'Work Tokyo')
 [void][Desktop]::PostMessage($appearance,0x10,[IntPtr]::Zero,[IntPtr]::Zero);[void][Desktop]::PostMessage([Desktop]::Appearance(),0x10,[IntPtr]::Zero,[IntPtr]::Zero);Wait-Desktop 400
 [void][Desktop]::SetForegroundWindow($windows[0]);Wait-Desktop 1000
 $outline=[Desktop]::ByClass('OmarchyOverlay.FocusOutline')
 Check 'Focus outline follows an eligible external window' ([Desktop]::IsWindowVisible($outline))
 Check 'Focus outline is hollow and leaves the app interior unobstructed' ([Desktop]::Hollow($outline))
 [void][Desktop]::PostMessage($main,0x8028,[IntPtr]0,[IntPtr]::Zero);Wait-Desktop 250
 $osd=[Desktop]::ByClass('OmarchyOverlay.OSD')
 Check 'On-screen volume feedback is visible' ([Desktop]::IsWindowVisible($osd))
 Wait-Desktop 1400
 Check 'On-screen feedback dismisses automatically' (-not [Desktop]::IsWindowVisible($osd))
 Click $main 5;$help=Wait-Panel 'OmarchyOverlay.Help';Click $help 506
 Check 'Optional Omarchy keymap reports compatible keys and skipped conflicts' ([Desktop]::ControlText($help,507) -match 'Applied.*compatible')
 Check 'Optional keymap keeps a restorable shortcut backup' (Test-Path "$local\shortcuts-before-omarchy.json")
 Click $help 508
 Check 'Undo keymap restores prior shortcut choices' ([Desktop]::ControlText($help,507) -match 'Previous shortcut choices restored')
 [void][Desktop]::PostMessage($help,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 Click $main 8;Click ([Desktop]::Appearance()) 38;$appearance=Wait-Panel 'OmarchyOverlay.AppearanceProfiles'
 $bitmap=[Drawing.Bitmap]::new(32,32);$wallpaper=Join-Path $root ('theme-fixture-'+[Guid]::NewGuid().ToString('N')+'.bmp');try{$bitmap.Save($wallpaper,[Drawing.Imaging.ImageFormat]::Bmp)}finally{$bitmap.Dispose()}
 Input-Text $appearance 814 $wallpaper;Click $appearance 815
 Check 'Optional wallpaper uses the selected image' ([Desktop]::Wallpaper() -eq $wallpaper)
 Check 'Wallpaper change preserves a recovery record' (Test-Path "$local\wallpaper-restore.json")
 Click $appearance 816
 $fragment=Get-Content $terminalPath -Raw|ConvertFrom-Json
 Check 'Terminal integration adds a separate native profile and scheme' ($fragment.profiles[0].name -eq 'Omarchy Overlay' -and $fragment.schemes.Count -eq 1 -and $null -eq $fragment.defaultProfile)
 Check 'Terminal integration preserves a recovery record' (Test-Path "$local\terminal-appearance-restore.json")
 Capture $appearance 'appearance-profiles.png'
 Click $appearance 817
 Check 'Restore returns the original wallpaper' ([Desktop]::Wallpaper() -eq $wallpaperBefore) "now '$([Desktop]::Wallpaper())', before '$wallpaperBefore', status '$([Desktop]::ControlText($appearance,819))'"
 Check 'Restore returns the original fragment or removes only its new file' ($(if($null -eq $terminalBefore){-not (Test-Path $terminalPath)}else{[IO.File]::ReadAllText($terminalPath) -eq $terminalBefore}))
 Check 'Successful appearance restore clears its journals' (-not (Test-Path "$local\wallpaper-restore.json") -and -not (Test-Path "$local\terminal-appearance-restore.json"))
 Click $appearance 816
 $ownedFragment=[IO.File]::ReadAllText($terminalPath)
 try {
  $externalFragment=$ownedFragment+"`r`n "
  [IO.File]::WriteAllText($terminalPath,$externalFragment,[Text.UTF8Encoding]::new($false));Click $appearance 817
  Check 'Restore preserves a Terminal fragment changed outside Omarchy Overlay' ([IO.File]::ReadAllText($terminalPath) -eq $externalFragment)
  Check 'External change retains the recovery record for review' (Test-Path "$local\terminal-appearance-restore.json")
 } finally {
  [IO.File]::WriteAllText($terminalPath,$ownedFragment,[Text.UTF8Encoding]::new($false));Click $appearance 817
 }
 Input-Text $appearance 814 $wallpaper;Click $appearance 815;Click $appearance 816
 $app.Kill();[void]$app.WaitForExit(5000);$app=$null
 $until=[DateTime]::UtcNow.AddSeconds(12)
 do{Wait-Desktop 250}while(((Test-Path "$local\wallpaper-restore.json") -or (Test-Path "$local\terminal-appearance-restore.json")) -and [DateTime]::UtcNow -lt $until)
 Check 'Recovery guard restores wallpaper after controller termination' ([Desktop]::Wallpaper() -eq $wallpaperBefore)
 Check 'Recovery guard restores the optional Terminal fragment after termination' ($(if($null -eq $terminalBefore){-not (Test-Path $terminalPath)}else{[IO.File]::ReadAllText($terminalPath) -eq $terminalBefore}))
 $app=Start-Process "$root\OmarchyOverlay.exe" -PassThru;$main=Wait-Panel 'OmarchyOverlay.Main'
 Click $main 8;Click ([Desktop]::Appearance()) 38;$appearance=Wait-Panel 'OmarchyOverlay.AppearanceProfiles'
 Input-Text $appearance 814 $wallpaper;Click $appearance 815;Click $appearance 816
 [void][Desktop]::PostMessage($main,0x312,[IntPtr]105,[IntPtr]::Zero)
 $until=[DateTime]::UtcNow.AddSeconds(8)
 do{Wait-Desktop 100}while(-not $app.HasExited -and [DateTime]::UtcNow -lt $until)
 Check 'Normal exit restores optional wallpaper before the controller ends' ($app.HasExited -and [Desktop]::Wallpaper() -eq $wallpaperBefore)
 Check 'Normal exit restores the optional Terminal fragment' ($(if($null -eq $terminalBefore){-not (Test-Path $terminalPath)}else{[IO.File]::ReadAllText($terminalPath) -eq $terminalBefore}))

} catch{$results.Add(@{name='Appearance/actions harness';passed=$false;detail=$_.Exception.ToString()})}
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

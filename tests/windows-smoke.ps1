$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$resultFile = Join-Path $root 'smoke-result.json'
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
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd,StringBuilder text,int max);
  public static string Title(IntPtr hwnd){var text=new StringBuilder(2048);GetWindowText(hwnd,text,2048);return text.ToString();}
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
  [StructLayout(LayoutKind.Sequential)] public struct POINT { public int x, y; }
  [StructLayout(LayoutKind.Sequential)] struct GUITHREADINFO { public int size,flags; public IntPtr active,focus,capture,menu,move,caret; public RECT caretRect; }
  [DllImport("user32.dll")] static extern bool GetGUIThreadInfo(uint thread, ref GUITHREADINFO info);
  [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr hwnd, out RECT rect);
  [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr hwnd, ref POINT point);
  public static IntPtr Focus(IntPtr hwnd) { uint pid; var info=new GUITHREADINFO(); info.size=Marshal.SizeOf(info); return GetGUIThreadInfo(GetWindowThreadProcessId(hwnd,out pid),ref info) ? info.focus : IntPtr.Zero; }
  public static bool InsideClient(IntPtr parent, IntPtr child) {
    RECT client, control; var origin=new POINT();
    return GetClientRect(parent,out client) && ClientToScreen(parent,ref origin) && GetWindowRect(child,out control)
      && control.left>=origin.x && control.top>=origin.y && control.right<=origin.x+client.right && control.bottom<=origin.y+client.bottom;
  }
  public static string ClientBounds(IntPtr hwnd) { RECT r; var p=new POINT(); GetClientRect(hwnd,out r); ClientToScreen(hwnd,ref p); return String.Format("{0},{1},{2},{3}",p.x,p.y,p.x+r.right,p.y+r.bottom); }
  public static bool MoveResize(IntPtr hwnd, int x, int y, int width, int height) { return SetWindowPos(hwnd,IntPtr.Zero,x,y,width,height,0x14); }
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
try {
    [void][Desktop]::SetProcessDpiAwarenessContext([IntPtr](-4))
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    Check 'Runs without an administrator token' (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) $identity.Name
    $existing = [Desktop]::MainWindow()
    Check 'No existing Omarchy Overlay instance to disturb' ($existing -eq [IntPtr]::Zero)
    # Run against defaults; restore the user's original preferences in finally.
    [IO.Directory]::CreateDirectory((Split-Path $configPath)) | Out-Null
    $fixtureExcluded=@('Taskmgr.exe','mstsc.exe','msrdc.exe','ApplicationFrameHost.exe','vmconnect.exe')+@(Get-Process | Where-Object {$_.SessionId -eq [Diagnostics.Process]::GetCurrentProcess().SessionId -and $_.ProcessName -ne 'powershell'} | ForEach-Object {$_.ProcessName+'.exe'} | Select-Object -Unique)
$fixtureDefaults=(@{excluded_processes=$fixtureExcluded;start_tiling=$false}|ConvertTo-Json -Depth 5)
[IO.File]::WriteAllText($configPath,$fixtureDefaults,[Text.UTF8Encoding]::new($false))
    $configTouched = $true
    $originalArea = [Desktop]::WorkArea()
    $originalTaskbar = [Desktop]::TaskbarState()
    $startMenu = [Environment]::GetFolderPath('Programs')
    $shortcut = Join-Path $startMenu ("Omarchy Overlay Test Launcher " + [Guid]::NewGuid().ToString() + '.lnk')
    $link = (New-Object -ComObject WScript.Shell).CreateShortcut($shortcut)
    $link.TargetPath = Join-Path $env:SystemRoot 'System32\cmd.exe'
    $link.Arguments = '/d /c echo launched>"' + $marker + '"'
    $link.Save()
    if (Test-Path $marker) { Remove-Item $marker }
    for ($i=0; $i -lt 3; $i++) {
        $form = [System.Windows.Forms.Form]::new()
        $form.Text = "Omarchy Overlay test window $($i+1)"
        $form.StartPosition = 'Manual'
        $form.SetBounds((120 + $i*100), (120 + $i*70), 560, 380)
        $form.BackColor = [System.Drawing.Color]::FromArgb(24 + $i*12, 32 + $i*10, 46 + $i*10)
        $label = [System.Windows.Forms.Label]::new()
        $label.Text = "WINDOW $($i+1)`r`n`r`nDisposable Omarchy Overlay test fixture"
        $label.ForeColor = [System.Drawing.Color]::White
        $label.Font = [System.Drawing.Font]::new('Segoe UI', 20)
        $label.SetBounds(24,24,500,160)
        $form.Controls.Add($label)
        $form.Show()
        $forms += $form
    }
    Wait-Desktop 300
    $originals = @($forms | ForEach-Object { ,(Bounds $_.Handle) })
    $app = Start-Process (Join-Path $root 'OmarchyOverlay.exe') -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    do {
        Wait-Desktop 250
        $main = [Desktop]::MainWindow()
    } while ((-not [Desktop]::Ready($main)) -and -not $app.HasExited -and [DateTime]::UtcNow -lt $deadline)
    Check 'Portable executable opens the Omarchy Overlay menu' ([Desktop]::IsWindowVisible([Desktop]::Overlay())) "Process exited: $($app.HasExited)"
    Check 'The 0.15 dashboard stays hidden at startup' (-not [Desktop]::IsWindowVisible($main))
    [Desktop]::CloseMenu()
    # Omarchy Overlay may still be finishing startup, so allow a few seconds for the posted Escape.
    $closeDeadline = [DateTime]::UtcNow.AddSeconds(5)
    do { Wait-Desktop 200 } while ([Desktop]::IsWindowVisible([Desktop]::Overlay()) -and [DateTime]::UtcNow -lt $closeDeadline)
    Check 'Escape closes the Omarchy Overlay menu' (-not [Desktop]::IsWindowVisible([Desktop]::Overlay()))
    Wait-Desktop 1200
    $bar = [Desktop]::Bar()
    Check 'Top bar opens without elevation' ($bar -ne [IntPtr]::Zero -and [Desktop]::IsWindowVisible($bar))
    $barBounds = Bounds $bar
    $reserved = [Desktop]::WorkArea()
    Check 'Windows reserves desktop space beneath the bar' ($reserved.top -ge $barBounds[3] -and $reserved.top -gt $originalArea.top) "$($reserved.top) / $($barBounds[3])"
    Check 'Windows taskbar is auto-hidden' ([Desktop]::TaskbarState() -eq 1)
    [void][Desktop]::PostMessage($bar, 0x0202, [IntPtr]::Zero, [IntPtr](20 + (10 -shl 16)))
    Wait-Desktop 400
    $launcher = [Desktop]::Launcher()
    Check 'Clicking MENU opens the Omarchy Overlay menu' ([Desktop]::IsWindowVisible([Desktop]::Overlay()))
    [Desktop]::CloseMenu()
    Wait-Desktop 300
    # The tray's "Open the 0.15 settings window" shows the same dashboard.
    Check 'The 0.15 dashboard still opens' ([Desktop]::ShowDashboard($main))
    Wait-Desktop 300
    # A short viewport forces Tab navigation to scroll the dashboard.
    $mainBounds = Bounds $main
    $area = [Desktop]::WorkArea()
    Check 'Dashboard can be sized to a short display' ([Desktop]::MoveResize($main, $area.left, $area.top, ($mainBounds[2]-$mainBounds[0]), 400))
    Wait-Desktop 200
    $lowerIds = @(35,28,10,11,20,13,7)
    Check 'Lower dashboard controls start clipped' (-not [Desktop]::InsideClient($main, [Desktop]::GetDlgItem($main,35)))
    Check 'Dashboard receives keyboard input' ([Desktop]::GetForegroundWindow() -eq $main -or [Desktop]::SetForegroundWindow($main))
    $reached = @{}
    for ($i=0; $i -lt 60 -and $reached.Count -lt $lowerIds.Count; $i++) {
        [Desktop]::keybd_event(0x09,0,0,[UIntPtr]::Zero)
        [Desktop]::keybd_event(0x09,0,2,[UIntPtr]::Zero)
        Wait-Desktop 50
        $focus = [Desktop]::Focus($main)
        foreach ($id in $lowerIds) {
            if ($focus -eq [Desktop]::GetDlgItem($main,$id)) {
                Check "Tab reveals dashboard control $id" ([Desktop]::InsideClient($main,$focus)) "control=$((Bounds $focus) -join ','); client=$([Desktop]::ClientBounds($main))"
                $reached[$id] = $true
            }
        }
    }
    Check 'Tab reaches every lower dashboard control' ($reached.Count -eq $lowerIds.Count) "reached=$($reached.Keys -join ',')"
    [void][Desktop]::MoveResize($main, $mainBounds[0], $mainBounds[1], ($mainBounds[2]-$mainBounds[0]), ($mainBounds[3]-$mainBounds[1]))
    [void][Desktop]::SendMessage($main,0x0115,[IntPtr]6,[IntPtr]::Zero)
    Wait-Desktop 200
    foreach ($entry in @(
        # 0.17: layouts, bar items and custom apps are pages of the Omarchy Overlay menu (OverlayApp.Overlay).
        @(29,'OmarchyOverlay.Overlay'), @(30,'OmarchyOverlay.Overlay'),
        @(31,'OmarchyOverlay.AppearanceProfiles'), @(32,'OmarchyOverlay.AppearanceOptions'),
        @(33,'OmarchyOverlay.AppRules'), @(34,'OmarchyOverlay.Overlay'), @(35,'OmarchyOverlay.Profiles')
    )) {
        Click $main $entry[0]
        Wait-Desktop 1500
        $editor = [Desktop]::FindWindow($entry[1], [System.Management.Automation.Language.NullString]::Value)
        Check "Dashboard opens $($entry[1])" ([Desktop]::IsWindowVisible($editor)) "control=$([Desktop]::GetDlgItem($main, $entry[0])); editor=$editor"
        if ($entry[1] -eq 'OmarchyOverlay.Overlay') { [void][Desktop]::SendMessage($editor, 0x0100, [IntPtr]0x1b, [IntPtr]::Zero); Wait-Desktop 300 }
        else { [void][Desktop]::SendMessage($editor, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) }
        if ($entry[0] -eq 34) {
            Check 'Closing dashboard custom apps does not open search' (-not [Desktop]::IsWindowVisible($launcher))
        }
    }
    Click $main 2
    $launcher=[Desktop]::Launcher();$searchDeadline=[DateTime]::UtcNow.AddSeconds(5)
    while(-not [Desktop]::IsWindowVisible($launcher) -and [DateTime]::UtcNow -lt $searchDeadline){Wait-Desktop 100;$launcher=[Desktop]::Launcher()}
    Check 'Dashboard opens command search' ([Desktop]::IsWindowVisible($launcher))
    # 0.17: the search's custom-apps button opens the Custom apps page of the Omarchy Overlay menu.
    Click $launcher 16
    Wait-Desktop 800
    $editor = [Desktop]::FindWindow('OmarchyOverlay.Overlay', [System.Management.Automation.Language.NullString]::Value)
    Check 'Search opens the custom apps menu page' ([Desktop]::IsWindowVisible($editor) -and -not [Desktop]::IsWindowVisible($launcher))
    [void][Desktop]::PostMessage($editor, 0x0100, [IntPtr]0x1b, [IntPtr]::Zero)
    Wait-Desktop 300
    Check 'Escape closes the custom apps menu page' (-not [Desktop]::IsWindowVisible($editor))
    if ([Desktop]::IsWindowVisible($launcher)) { [void][Desktop]::SendMessage($launcher, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) }
    [void][Desktop]::PostMessage($main, 0x0111, [IntPtr]6, [IntPtr]::Zero)
    Wait-Desktop 1000
    Check 'Top bar can be hidden from the controls' ([Desktop]::Bar() -eq [IntPtr]::Zero)
    Check 'Hiding the bar restores the taskbar preference' ([Desktop]::TaskbarState() -eq $originalTaskbar) ("expected=$originalTaskbar; actual=$([Desktop]::TaskbarState())")
    Check 'Hiding the bar releases reserved desktop space' ([Desktop]::WorkArea().top -eq $originalArea.top)
    [void][Desktop]::PostMessage($main, 0x0111, [IntPtr]6, [IntPtr]::Zero)
    Wait-Desktop 1000
    Check 'Top bar can be shown again' ([Desktop]::Bar() -ne [IntPtr]::Zero)
    $barBounds = Bounds ([Desktop]::Bar())
    [void][Desktop]::PostMessage($main, 0x0111, [IntPtr]7, [IntPtr]::Zero)
    Wait-Desktop 600
    Check 'Windows taskbar can be shown while keeping the top bar' ([Desktop]::TaskbarState() -eq 0 -and [Desktop]::Bar() -ne [IntPtr]::Zero)
    [void][Desktop]::PostMessage($main, 0x0111, [IntPtr]7, [IntPtr]::Zero)
    Wait-Desktop 600
    Check 'Windows taskbar can be auto-hidden again' ([Desktop]::TaskbarState() -eq 1)
    for ($i=0; $i -lt 3; $i++) {
        Check "Starts paused: fixture $i unchanged" (((Bounds $forms[$i].Handle) -join ',') -eq ($originals[$i] -join ','))
    }
    [Desktop]::Shortcut(0x54)
    Wait-Desktop 2300
    $tiled = @($forms | ForEach-Object { ,(Bounds $_.Handle) })
    Check 'Actual global shortcut enables tiling' (($tiled[0] -join ',') -ne ($originals[0] -join ','))
    $area = [System.Windows.Forms.Screen]::PrimaryScreen.WorkingArea
    for ($i=0; $i -lt 3; $i++) {
        $r = $tiled[$i]
        Check "Fixture $i stays beneath the top bar" ($r[1] -ge $barBounds[3]) ($r -join ',')
    }
    [void][Desktop]::SetForegroundWindow($forms[0].Handle)
    Wait-Desktop 200
    $beforeFocus = [Desktop]::GetForegroundWindow()
    # Every tiled window has at least one directional neighbor.
    foreach ($key in @(0x48,0x4a,0x4b,0x4c)) {
        [Desktop]::Shortcut($key)
        Wait-Desktop 150
        if ([Desktop]::GetForegroundWindow() -ne $beforeFocus) { break }
    }
    Check 'Directional focus changes the active window' ([Desktop]::GetForegroundWindow() -ne $beforeFocus) ("before=$beforeFocus '$([Desktop]::Title($beforeFocus))' after=$([Desktop]::GetForegroundWindow()) target=$($forms[0].Handle) positions=$($tiled|ForEach-Object{$_ -join ','})")
    # Choose the two vertically adjacent stack panes. The full-height main pane's
    # downward neighbor changes with work-area height and WinForms minimum size.
    $stackLeft=($tiled|ForEach-Object {$_[0]}|Measure-Object -Maximum).Maximum
    $stack=@(0..2|Where-Object {$tiled[$_][0] -eq $stackLeft}|Sort-Object {$tiled[$_][1]})
    Check 'Two stack fixtures have a vertical shared boundary' ($stack.Count -eq 2)
    [void][Desktop]::SetForegroundWindow($forms[$stack[0]].Handle)
    [Desktop]::Shortcut(0x4a, $true)
    Wait-Desktop 1000
    Check 'Swap-down exchanges neighboring panes' ((((Bounds $forms[$stack[0]].Handle) -join ',') -eq ($tiled[$stack[1]] -join ',')) -and (((Bounds $forms[$stack[1]].Handle) -join ',') -eq ($tiled[$stack[0]] -join ',')))
    $beforeResize = Bounds $forms[0].Handle
    [Desktop]::Shortcut(0xdd)
    Wait-Desktop 1000
    Check 'Growing the main pane updates the stack' (((Bounds $forms[0].Handle) -join ',') -ne ($beforeResize -join ','))
    $newForm = [System.Windows.Forms.Form]::new()
    $newForm.Text = 'Omarchy Overlay temporary new-window fixture'
    $newForm.StartPosition = 'Manual'
    $newForm.SetBounds(250,250,500,350)
    $newForm.Show()
    Wait-Desktop 1500
    $newBounds = Bounds $newForm.Handle
    Check 'A newly opened window is automatically tiled' (($newBounds -join ',') -ne '250,250,750,600')
    $newForm.Close()
    $newForm.Dispose()
    Wait-Desktop 1000
    [void][Desktop]::SetForegroundWindow($forms[0].Handle)
    Wait-Desktop 450
    [Desktop]::Shortcut(0x20)
    Wait-Desktop 500
    Check 'Ctrl+Alt+Space opens the Omarchy Overlay menu' ([Desktop]::IsWindowVisible([Desktop]::Overlay()))
    [Desktop]::CloseMenu()
    Wait-Desktop 300
    # The 0.15 search window is still reachable from the dashboard.
    Click $main 2
    $launcher = [Desktop]::Launcher();$searchDeadline=[DateTime]::UtcNow.AddSeconds(5)
    while(-not [Desktop]::IsWindowVisible($launcher) -and [DateTime]::UtcNow -lt $searchDeadline){Wait-Desktop 100;$launcher=[Desktop]::Launcher()}
    Check 'Launcher opens from the dashboard' ([Desktop]::IsWindowVisible($launcher))
    $edit = [Desktop]::GetDlgItem($launcher, 10)
    $list = [Desktop]::GetDlgItem($launcher, 11)
    Check 'Launcher search box exists' ($edit -ne [IntPtr]::Zero -and $list -ne [IntPtr]::Zero)
    [void][Desktop]::SendText($edit, 0x000c, [IntPtr]::Zero, 'File Explorer')
    Wait-Desktop 200
    $matches = [Desktop]::SendMessage($list, 0x018b, [IntPtr]::Zero, [IntPtr]::Zero).ToInt32()
    Check 'Launcher filters installed app entries' ($matches -ge 1 -and $matches -le 3) $matches
    [void][Desktop]::SendText($edit, 0x000c, [IntPtr]::Zero, 'NoSuchApp_87341')
    Wait-Desktop 150
    $matches = [Desktop]::SendMessage($list, 0x018b, [IntPtr]::Zero, [IntPtr]::Zero).ToInt32()
    Check 'Launcher handles an empty search' ($matches -eq 1) $matches
    [void][Desktop]::SendText($edit, 0x000c, [IntPtr]::Zero, ([IO.Path]::GetFileNameWithoutExtension($shortcut)))
    Wait-Desktop 150
    [void][Desktop]::PostMessage($edit, 0x0100, [IntPtr]0x0d, [IntPtr]::Zero)
    $launchDeadline=[DateTime]::UtcNow.AddSeconds(10)
    do{Wait-Desktop 100}while(-not (Test-Path $marker) -and [DateTime]::UtcNow -lt $launchDeadline)
    Check 'Enter launches the selected Start-menu shortcut' (Test-Path $marker)
    # The 0.16 app picker (bar launcher and Super+Space) searches the same Start-menu entries.
    Remove-Item $marker
    # Let the launched console close first so it cannot take focus from the picker.
    Wait-Desktop 1500
    [void][Desktop]::SetForegroundWindow($forms[0].Handle)
    Wait-Desktop 300
    [void][Desktop]::PostMessage($main, 0x8000 + 10, [IntPtr]::Zero, [IntPtr]::Zero)
    $picker = [Desktop]::Overlay()
    # The 0.15 search window shares the app list, so the picker rescans the Start menu first.
    $opened = [Diagnostics.Stopwatch]::StartNew()
    do { Wait-Desktop 50 } while (-not [Desktop]::IsWindowVisible($picker) -and $opened.ElapsedMilliseconds -lt 5000)
    Check 'Bar launcher opens the Omarchy Overlay app picker' ([Desktop]::IsWindowVisible($picker)) ("opened after $($opened.ElapsedMilliseconds) ms")
    foreach ($c in ([IO.Path]::GetFileNameWithoutExtension($shortcut)).Substring(0, 31).ToCharArray()) {
        [void][Desktop]::PostMessage($picker, 0x0102, [IntPtr][int]$c, [IntPtr]::Zero)
    }
    Wait-Desktop 300
    [void][Desktop]::PostMessage($picker, 0x0100, [IntPtr]0x0d, [IntPtr]::Zero)
    $launchDeadline=[DateTime]::UtcNow.AddSeconds(10)
    do{Wait-Desktop 100}while(-not (Test-Path $marker) -and [DateTime]::UtcNow -lt $launchDeadline)
    Check 'Typing and Enter in the app picker launches the shortcut' (Test-Path $marker)
    Check 'The app picker closes after launching' (-not [Desktop]::IsWindowVisible($picker))
    [void][Desktop]::SetForegroundWindow($forms[0].Handle)
    [Desktop]::Shortcut(0xbc)
    Wait-Desktop 500
    Check 'Opacity hotkey fades a normal window' ([Desktop]::Alpha($forms[0].Handle) -eq 242) ("alpha=$([Desktop]::Alpha($forms[0].Handle)); foreground=$([Desktop]::GetForegroundWindow()); fixture=$($forms[0].Handle); bounds=$((Bounds $forms[0].Handle) -join ',')")
    [void][Desktop]::SetForegroundWindow($forms[0].Handle)
    Wait-Desktop 200
    [Desktop]::Shortcut(0xbe)
    Wait-Desktop 500
    Check 'Increasing opacity restores an originally opaque window' (-not [Desktop]::Layered($forms[0].Handle)) ("alpha=$([Desktop]::Alpha($forms[0].Handle)); foreground=$([Desktop]::GetForegroundWindow()); fixture=$($forms[0].Handle)")
    [Desktop]::Shortcut(0xbc)
    Wait-Desktop 400
    [Desktop]::Shortcut(0x08)
    Wait-Desktop 400
    Check 'Restore-opacity hotkey removes only the added layered style' (-not [Desktop]::Layered($forms[0].Handle))
    [void][Desktop]::PostMessage($main,0x0111,[IntPtr]8,[IntPtr]::Zero)
    Wait-Desktop 500
    $appearance = [Desktop]::Appearance()
    Check 'Appearance controls open without elevation' ([Desktop]::IsWindowVisible($appearance))
    $combo = [Desktop]::GetDlgItem($appearance,30)
    $previousColor = [Desktop]::Pixel([Desktop]::Bar())
    foreach ($index in @(1,2,3,4,5,0)) {
        [void][Desktop]::SendMessage($combo,0x014e,[IntPtr]$index,[IntPtr]::Zero)
        [void][Desktop]::PostMessage($appearance,0x0111,[IntPtr](30 + (1 -shl 16)),$combo)
        Wait-Desktop 450
        $color = [Desktop]::Pixel([Desktop]::Bar())
        Check "Theme $index repaints the actual top bar" ($color -ne $previousColor) $color
        $previousColor = $color
    }
    Set-Slider $appearance 31 80
    Check 'Top-bar opacity slider changes native alpha' ([Desktop]::Alpha([Desktop]::Bar()) -eq 204)
    Set-Slider $appearance 33 75
    Check 'Selected-window opacity slider applies to the remembered app' ([Desktop]::Alpha($forms[0].Handle) -eq 191)
    [void][Desktop]::PostMessage($appearance,0x0111,[IntPtr]35,[IntPtr]::Zero)
    Wait-Desktop 450
    Check 'Restore selected removes its opacity change' (-not [Desktop]::Layered($forms[0].Handle))
    [void][Desktop]::PostMessage($appearance,0x0111,[IntPtr]36,[IntPtr]::Zero)
    Wait-Desktop 450
    Set-Slider $appearance 32 85
    Check 'Inactive opacity fades background windows' ([Desktop]::Alpha($forms[2].Handle) -eq 217)
    [void][Desktop]::SetForegroundWindow($forms[2].Handle)
    Wait-Desktop 900
    Check 'Focusing an automatically faded window restores its native opacity' (-not [Desktop]::Layered($forms[2].Handle))
    [Desktop]::Shortcut(0x08,$true)
    Wait-Desktop 500
    Check 'Emergency restore-all disables inactive fading' (-not [Desktop]::Layered($forms[0].Handle) -and -not [Desktop]::Layered($forms[1].Handle))
    [Desktop]::NativeOpacity($forms[1].Handle,200)
    [void][Desktop]::SetForegroundWindow($forms[1].Handle)
    [Desktop]::Shortcut(0xbc)
    Wait-Desktop 450
    Check 'Can adjust a window with existing constant-alpha transparency' ([Desktop]::Alpha($forms[1].Handle) -lt 200)
    [Desktop]::Shortcut(0x08)
    Wait-Desktop 450
    Check 'Restore preserves an application original alpha' ([Desktop]::Alpha($forms[1].Handle) -eq 200)
    [Desktop]::Shortcut(0xbc)
    Wait-Desktop 450
    [void][Desktop]::SetForegroundWindow($forms[0].Handle)
    [Desktop]::Shortcut(0xbc)
    Wait-Desktop 450
    Check 'Opacity recovery record exists before exit' (Test-Path (Join-Path (Split-Path $configPath) 'opacity-restore.json'))
    Click $main 3
    $settings=[Desktop]::Settings()
    Check 'Settings and app rules open without elevation' ([Desktop]::IsWindowVisible($settings))
    $monitors=[Desktop]::SendMessage([Desktop]::GetDlgItem($settings,49),0x0146,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32()
    Check 'Preferred-monitor choices include the detected display' ($monitors -ge 2) $monitors
    Click $settings 50
    $exe=([Diagnostics.Process]::GetCurrentProcess()).ProcessName+'.exe'
    Input-Text $settings 44 $exe
    Input-Text $settings 45 'Omarchy Overlay test window 1'
    Choose $settings 47 1
    Choose $settings 48 1
    Click $settings 51
    Wait-Desktop 700
    Check 'Floating app rule restores the original window placement' (((Bounds $forms[0].Handle) -join ',') -eq ($originals[0] -join ','))
    Check 'Native-opacity rule restores an already faded window' (-not [Desktop]::Layered($forms[0].Handle))
    $profile=Get-Content -Raw $configPath | ConvertFrom-Json
    Check 'App rule is saved with its title filter' (@($profile.rules).Count -eq 1 -and $profile.rules[0].title_contains -eq 'Omarchy Overlay test window 1')
    Select-Rule $settings
    Choose $settings 47 0
    Choose $settings 49 1
    Click $settings 51
    Wait-Desktop 700
    Check 'Changing the rule to tile adopts the window again' (((Bounds $forms[0].Handle) -join ',') -ne ($originals[0] -join ','))
    $profile=Get-Content -Raw $configPath | ConvertFrom-Json
    Check 'Preferred monitor is saved by Windows display name' ($profile.rules[0].monitor -like '*DISPLAY*')
    Select-Rule $settings
    Choose $settings 47 2
    Click $settings 51
    [void][Desktop]::SetForegroundWindow($forms[0].Handle)
    [Desktop]::Shortcut(0xbc)
    Wait-Desktop 450
    Check 'Ignored app stays at its original position and opacity' ((((Bounds $forms[0].Handle) -join ',') -eq ($originals[0] -join ',')) -and -not [Desktop]::Layered($forms[0].Handle))
    Select-Rule $settings
    Click $settings 52
    Wait-Desktop 650
    Check 'Deleting a rule returns the window to automatic layout' (((Bounds $forms[0].Handle) -join ',') -ne ($originals[0] -join ','))
    Choose $settings 60 1
    Input-Text $settings 61 '16'
    Input-Text $settings 62 '60'
    Click $settings 64
    Wait-Desktop 650
    $a=Bounds $forms[0].Handle; $b=Bounds $forms[1].Handle
    Check 'Columns layout creates distinct horizontal panes' ($a[0] -ne $b[0] -and [Math]::Abs($a[1]-$b[1]) -lt 3)
    Choose $settings 60 2
    Click $settings 64
    Wait-Desktop 650
    $a=Bounds $forms[0].Handle; $b=Bounds $forms[1].Handle
    Check 'Rows layout creates distinct vertical panes' ([Math]::Abs($a[0]-$b[0]) -lt 3 -and $a[1] -ne $b[1])
    Choose $settings 60 0
    Input-Text $settings 61 '10'
    Input-Text $settings 62 '55'
    Click $settings 64
    Click $settings 54
    $profile=Get-Content -Raw $configPath | ConvertFrom-Json
    Check 'Work starters include Edge Explorer Excel and Outlook variants' (@($profile.rules).Count -eq 5) (@($profile.rules).executable -join ',')
    while(@((Get-Content -Raw $configPath | ConvertFrom-Json).rules).Count -gt 0) {Select-Rule $settings; Click $settings 52}
    [void][Desktop]::SendMessage($settings,0x0010,[IntPtr]::Zero,[IntPtr]::Zero)
    [void][Desktop]::SendMessage($launcher, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero)
    [void][Desktop]::SetForegroundWindow($forms[0].Handle)
    [Desktop]::Shortcut(0x46)
    Wait-Desktop 1000
    Check 'Floating restores the selected window' (((Bounds $forms[0].Handle) -join ',') -eq ($originals[0] -join ','))
    [void][Desktop]::PostMessage($main, 0x0111, [IntPtr]6, [IntPtr]::Zero)
    Wait-Desktop 1000
    [Desktop]::Shortcut(0x54)
    Wait-Desktop 1100
    for ($i=0; $i -lt 3; $i++) {
        Check "Pause restores fixture $i" (((Bounds $forms[$i].Handle) -join ',') -eq ($originals[$i] -join ','))
    }
    [void][Desktop]::PostMessage($main, 0x0111, [IntPtr]6, [IntPtr]::Zero)
    Wait-Desktop 1000
    [Desktop]::Shortcut(0x54)
    Wait-Desktop 1100
    [Desktop]::Shortcut(0x1b)
    Wait-Desktop 1200
    Check 'Exit shortcut stops the background app' ($app.HasExited)
    Check 'Exit removes the top bar' ([Desktop]::Bar() -eq [IntPtr]::Zero)
    Check 'Exit restores the original taskbar preference' ([Desktop]::TaskbarState() -eq $originalTaskbar)
    Check 'Exit releases reserved desktop space' ([Desktop]::WorkArea().top -eq $originalArea.top)
    Check 'Exit restores normal-window opacity' (-not [Desktop]::Layered($forms[0].Handle))
    Check 'Exit preserves pre-existing layered-window alpha' ([Desktop]::Alpha($forms[1].Handle) -eq 200)
    Check 'Successful exit removes the opacity recovery record' (-not (Test-Path (Join-Path (Split-Path $configPath) 'opacity-restore.json')))
    for ($i=0; $i -lt 3; $i++) {
        Check "Exit restores fixture $i" (((Bounds $forms[$i].Handle) -join ',') -eq ($originals[$i] -join ','))
    }
    # Verify that the show-taskbar control also works if auto-hide was already on.
    [Desktop]::SetTaskbarState(1)
    $app = Start-Process (Join-Path $root 'OmarchyOverlay.exe') -PassThru
    Wait-Desktop 1800
    $main = [Desktop]::MainWindow()
    Check 'Starts when the Windows taskbar was already auto-hidden' ($main -ne [IntPtr]::Zero)
    Check 'Bar opacity setting survives restarting Omarchy Overlay' ([Desktop]::Alpha([Desktop]::Bar()) -eq 204) ([Desktop]::Alpha([Desktop]::Bar()))
    [void][Desktop]::PostMessage($main, 0x0111, [IntPtr]7, [IntPtr]::Zero)
    Wait-Desktop 800
    Check 'Can show an originally auto-hidden taskbar' ([Desktop]::TaskbarState() -eq 0)
    [Desktop]::Shortcut(0x1b)
    Wait-Desktop 1200
    Check 'Exit preserves an original auto-hide preference' ([Desktop]::TaskbarState() -eq 1)
} catch {
    $results.Add(@{ name='Test harness'; passed=$false; detail=$_.Exception.ToString() })
} finally {
    if ($null -ne $app -and -not $app.HasExited) {
        [Desktop]::Shortcut(0x1b)
        Wait-Desktop 1200
    }
    foreach ($form in $forms) { $form.Close(); $form.Dispose() }
    if ($null -ne $shortcut -and (Test-Path $shortcut)) { Remove-Item $shortcut }
    if (Test-Path $marker) { Remove-Item $marker }
    if ($null -ne $originalTaskbar) { [Desktop]::SetTaskbarState($originalTaskbar) }
    if ($configTouched) {
        if ($null -ne $originalConfig) { [IO.File]::WriteAllText($configPath,$originalConfig,[Text.UTF8Encoding]::new($false)) }
        elseif (Test-Path $configPath) { Remove-Item $configPath }
    }
 if($configTouched){
  if($null -ne $originalUpgradeBackup){[IO.File]::WriteAllBytes($upgradeBackupPath,$originalUpgradeBackup)}
  elseif(Test-Path $upgradeBackupPath){Remove-Item -LiteralPath $upgradeBackupPath}
 }
    $results | ConvertTo-Json -Depth 5 | Set-Content -Encoding UTF8 $resultFile
}

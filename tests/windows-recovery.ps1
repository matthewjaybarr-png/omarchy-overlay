$ErrorActionPreference='Stop'
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
$local=Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'OmarchyOverlay'
$results=[Collections.Generic.List[object]]::new()
$app=$null;$forms=@();$originalTaskbar=$null
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
  [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] static extern IntPtr ReadText(IntPtr h,uint m,IntPtr size,System.Text.StringBuilder text);
  public static string ControlText(IntPtr h,int id){var b=new System.Text.StringBuilder(128);ReadText(GetDlgItem(h,id),13,(IntPtr)b.Capacity,b);return b.ToString();}
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
  [DllImport("dwmapi.dll")] static extern int DwmGetWindowAttribute(IntPtr hwnd,uint attribute,out uint value,uint size);
  public static long Dwm(IntPtr hwnd,uint attribute) { uint v; return DwmGetWindowAttribute(hwnd,attribute,out v,4)>=0?(long)v:-1; }
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
    do{Pump 100;$hwnd=[RecoverTest]::MainWindow();$owner=0;[void][RecoverTest]::GetWindowThreadProcessId($hwnd,[ref]$owner)}while(($hwnd -eq [IntPtr]::Zero -or $owner -ne $process.Id -or -not [RecoverTest]::Ready($hwnd))-and [DateTime]::UtcNow -lt $deadline -and -not $process.HasExited)
    if($hwnd -eq [IntPtr]::Zero -or $owner -ne $process.Id -or -not [RecoverTest]::Ready($hwnd)){throw 'Controller did not start.'};[RecoverTest]::CloseMenu()
    Pump 700;return $process
}
function Stop-App($process){[void][RecoverTest]::PostMessage([RecoverTest]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero);$until=[DateTime]::UtcNow.AddSeconds(8);while(-not $process.HasExited -and [DateTime]::UtcNow -lt $until){Pump 100};Pump 700;if(-not $process.HasExited){throw 'Controller did not exit.'}}
function Read-Profile($folder){return Get-Content -Raw (Join-Path $folder 'overlay-profile.json')|ConvertFrom-Json}
function Wait-Gap([string]$gap){$until=[DateTime]::UtcNow.AddSeconds(5);do{Pump 100;$value=[RecoverTest]::ControlText([RecoverTest]::Settings(),61)}while($value -ne $gap -and [DateTime]::UtcNow -lt $until);return $value -eq $gap}
function Choose-File([string]$path,[bool]$save){
    $dialog=[RecoverTest]::FileDialog($save)
    if($dialog -eq [IntPtr]::Zero){throw 'Expected profile file dialog did not open.'}
    [void][RecoverTest]::SetForegroundWindow($dialog);Pump 100
    if([RecoverTest]::GetForegroundWindow() -ne $dialog){throw 'Profile dialog did not have focus; no path was typed.'}
    [Windows.Forms.SendKeys]::SendWait('%n');[Windows.Forms.SendKeys]::SendWait('^a');[Windows.Forms.SendKeys]::SendWait($path);[Windows.Forms.SendKeys]::SendWait('{ENTER}');Pump 900
}
try{
    [void][RecoverTest]::SetProcessDpiAwarenessContext([IntPtr](-4))
    $principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
    Check 'Recovery test runs with a limited user token' (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))
    Check 'No existing controller to disturb' ([RecoverTest]::MainWindow() -eq [IntPtr]::Zero)
    $originalTaskbar=[RecoverTest]::Taskbar()
    $originalTop=[RecoverTest]::WorkTop()
    $a=Join-Path $root 'portable-a';$b=Join-Path $root 'portable-b'
    New-Item -ItemType Directory -Force $a,$b|Out-Null
    foreach($folder in @($a,$b)){Copy-Item (Join-Path $root 'OmarchyOverlay.exe') (Join-Path $folder 'OmarchyOverlay.exe') -Force}
    $id='vm-profile-'+[Guid]::NewGuid().ToString('N')
    $excluded=@(Get-Process|Where-Object {$_.SessionId -eq [Diagnostics.Process]::GetCurrentProcess().SessionId -and $_.ProcessName -ne 'powershell'}|ForEach-Object {$_.ProcessName+'.exe'}|Select-Object -Unique)
    $profile=@{format='overlay-profile';version=1;id=$id;settings=@{excluded_processes=$excluded;gap=17;theme='nord';auto_hide_taskbar=$true;show_top_bar=$true;inactive_opacity=100;rules=@()}}
    [IO.File]::WriteAllText((Join-Path $a 'overlay-profile.json'),($profile|ConvertTo-Json -Depth 10),[Text.UTF8Encoding]::new($false))
    $localConfig=if(Test-Path (Join-Path $local 'config.json')){[IO.File]::ReadAllText((Join-Path $local 'config.json'))}else{''}
    # Theme effects change the Windows accent and wallpaper; recovery must put both back.
    $dwmKey='HKCU:\Software\Microsoft\Windows\DWM';$deskKey='HKCU:\Control Panel\Desktop'
    $accentBefore=(Get-ItemProperty $dwmKey -ErrorAction SilentlyContinue).AccentColor;$wallBefore=(Get-ItemProperty $deskKey).WallPaper
    $app=Start-App $a
    Click ([RecoverTest]::MainWindow()) 3
    $settings=[RecoverTest]::Settings();Click $settings 68
    Check 'Profiles and recovery panel opens without admin' ([RecoverTest]::IsWindowVisible([RecoverTest]::Profiles()))
    [void][RecoverTest]::SendMessage([RecoverTest]::GetDlgItem($settings,61),0x14e,[IntPtr]23,[IntPtr]::Zero);Click $settings 64
    Check 'Settings edits save beside a portable executable' ((Read-Profile $a).settings.gap -eq 23)
    Check 'Portable edits leave the existing local profile unchanged' ($(if(Test-Path (Join-Path $local 'config.json')){[IO.File]::ReadAllText((Join-Path $local 'config.json'))}else{''}) -eq $localConfig)
    $exported=Join-Path $root ('exported-'+$id+'.json')
    [void][RecoverTest]::SetForegroundWindow([RecoverTest]::Profiles())
    Click ([RecoverTest]::Profiles()) 81;Choose-File $exported $true
    Check 'Native Save dialog exports a validated portable profile' ((Get-Content -Raw $exported|ConvertFrom-Json).format -eq 'overlay-profile')
    $import=Get-Content -Raw $exported|ConvertFrom-Json;$import.settings.gap=25
    [IO.File]::WriteAllText($exported,($import|ConvertTo-Json -Depth 10),[Text.UTF8Encoding]::new($false))
    Click ([RecoverTest]::Profiles()) 82;Choose-File $exported $false
    Check 'Native Open dialog imports preferences into the active profile' ((Read-Profile $a).settings.gap -eq 25)
    Check 'Imported preferences finish refreshing Settings before external edits' (Wait-Gap '25')
    Check 'Import preserves a local last-preferences backup' (Test-Path (Join-Path $local 'profile-backup.json'))
    $raw=[IO.File]::ReadAllText((Join-Path $a 'overlay-profile.json'))
    Check 'Portable data contains no window handles or recovery records' ($raw -notmatch '"hwnd"|"pid"|"marker"|placement-restore|opacity-restore')
    # Reload an externally edited, valid shared profile through the real UI.
    $edited=Read-Profile $a;$edited.settings.gap=19
    [IO.File]::WriteAllText((Join-Path $a 'overlay-profile.json'),($edited|ConvertTo-Json -Depth 10),[Text.UTF8Encoding]::new($false))
    Click ([RecoverTest]::Profiles()) 83
    Check 'Reload applies externally edited preferences in Settings' (Wait-Gap '19')
    Check 'Reload creates a recoverable last-preferences backup' (Test-Path (Join-Path $local 'profile-backup.json'))
    Stop-App $app;$app=$null
    Copy-Item (Join-Path $a 'overlay-profile.json') (Join-Path $b 'overlay-profile.json') -Force
    $app=Start-App $b
    Click ([RecoverTest]::MainWindow()) 3
    $settings=[RecoverTest]::Settings()
    $buffer=[Text.StringBuilder]::new(128)
    # Read edit text with WM_GETTEXT across processes through an unmanaged helper below.
    Check 'Copied profile retains its edited preferences' ((Read-Profile $b).settings.gap -eq 19) ("source=$((Read-Profile $a).settings.gap); copied=$((Read-Profile $b).settings.gap)")
    Stop-App $app;$app=$null
    Check 'Normal exit leaves no placement journal' (-not (Test-Path (Join-Path $local 'placement-restore.json')))
    for($i=0;$i -lt 2;$i++){
        $form=[Windows.Forms.Form]::new();$form.Text="Omarchy Overlay recovery fixture $i";$form.StartPosition='Manual';$form.SetBounds((160+140*$i),(180+90*$i),600,400);$form.Show();$forms+=$form
    }
    Pump 300;$originals=@($forms|ForEach-Object{Bounds $_.Handle})
    $app=Start-App $a
    $guard=@(Get-CimInstance Win32_Process -Filter "ParentProcessId=$($app.Id)"|Where-Object Name -eq 'OmarchyOverlay.exe')
    Check 'Recovery guard is another mode of the same executable' ($guard.Count -eq 1 -and $guard[0].CommandLine -match '--watchdog')
    [RecoverTest]::Shortcut(0x54);Pump 1800
    Check 'Crash fixture is tiled before the test' ((Bounds $forms[0].Handle) -ne $originals[0])
    $focused=[RecoverTest]::SetForegroundWindow($forms[0].Handle);Pump 150
    Check 'Disposable fixture has foreground focus before opacity shortcut' ([RecoverTest]::GetForegroundWindow() -eq $forms[0].Handle) $focused
    [RecoverTest]::Shortcut(0xbc);Pump 500
    Check 'Crash fixture has a native opacity change before the test' ([RecoverTest]::Alpha($forms[0].Handle) -eq 242)
    # Borders are on by default: square corners (33 = 1) and a theme border color (34).
    Check 'Crash fixture has square corners before the test' ([RecoverTest]::Dwm($forms[0].Handle,33) -eq 1) ([RecoverTest]::Dwm($forms[0].Handle,33))
    Check 'Theme accent is applied before the crash' ((Test-Path (Join-Path $local 'windows-accent-restore.json')) -and (Get-ItemProperty $dwmKey).AccentColor -ne $accentBefore) ((Get-ItemProperty $dwmKey).AccentColor)
    # DWM cannot read the border color back, so the journal must record its default for recovery.
    $frames=if(Test-Path (Join-Path $local 'window-theme-restore.json')){[IO.File]::ReadAllText((Join-Path $local 'window-theme-restore.json'))}else{''}
    Check 'Window-frame journal records the default border color' ($frames -match '\[\s*34,\s*4294967295\s*\]') $frames
    Check 'Original placement is journaled locally before a crash' (Test-Path (Join-Path $local 'placement-restore.json'))
    Check 'Original opacity is journaled locally before a crash' (Test-Path (Join-Path $local 'opacity-restore.json'))
    Check 'The profile folder contains no machine recovery files' (@(Get-ChildItem $a -Filter '*restore*').Count -eq 0)
    $beforeCrash=[IO.File]::ReadAllText((Join-Path $a 'overlay-profile.json'))
    # The guard writes its report after the journals are gone, so an older report must not be read instead.
    $reportPath=Join-Path $local 'recovery-report.json';Remove-Item $reportPath -ErrorAction SilentlyContinue
    # Only this controller PID is terminated; the guard and all other apps remain running.
    Stop-Process -Id $app.Id -Force
    $deadline=[DateTime]::UtcNow.AddSeconds(12)
    do{Pump 150}while(((Test-Path (Join-Path $local 'placement-restore.json')) -or (Test-Path (Join-Path $local 'opacity-restore.json')) -or (Test-Path (Join-Path $local 'window-theme-restore.json')) -or (Test-Path (Join-Path $local 'taskbar-restore.json')) -or (Test-Path (Join-Path $local 'windows-accent-restore.json')) -or (Test-Path (Join-Path $local 'wallpaper-restore.json')) -or -not (Test-Path $reportPath)) -and [DateTime]::UtcNow -lt $deadline)
    Check 'Guard removes the top bar after controller termination' ([RecoverTest]::Bar() -eq [IntPtr]::Zero)
    for($i=0;$i -lt 2;$i++){Check "Guard restores fixture $i without restarting the app" ((Bounds $forms[$i].Handle) -eq $originals[$i]) (Bounds $forms[$i].Handle)}
    Check 'Guard restores opacity without restarting the app' (-not [RecoverTest]::Layered($forms[0].Handle))
    Check 'Guard restores default corners' ([RecoverTest]::Dwm($forms[0].Handle,33) -eq 0) ([RecoverTest]::Dwm($forms[0].Handle,33))
    Check 'Guard restores the original taskbar preference' ([RecoverTest]::Taskbar() -eq $originalTaskbar)
    Check 'Guard releases the crashed top-bar reservation' ([RecoverTest]::WorkTop() -eq $originalTop)
    $accentNow=(Get-ItemProperty $dwmKey -ErrorAction SilentlyContinue).AccentColor
    Check 'Guard restores the Windows accent color' ($accentNow -eq $accentBefore -and -not (Test-Path (Join-Path $local 'windows-accent-restore.json'))) "$accentBefore -> $accentNow"
    $wallNow=(Get-ItemProperty $deskKey).WallPaper
    Check 'Guard restores the wallpaper' ($wallNow -eq $wallBefore -and -not (Test-Path (Join-Path $local 'wallpaper-restore.json'))) "$wallBefore -> $wallNow"
    Check 'Crash recovery preserves portable preferences' ([IO.File]::ReadAllText((Join-Path $a 'overlay-profile.json')) -eq $beforeCrash)
    $report=$null;$until=[DateTime]::UtcNow.AddSeconds(3)
    do{try{$report=Get-Content -Raw $reportPath -ErrorAction Stop|ConvertFrom-Json}catch{Pump 100}}while($null -eq $report -and [DateTime]::UtcNow -lt $until)
    Check 'Recovery report records success without errors' (@($report.errors).Count -eq 0 -and $report.placement_records -ge 2 -and $report.opacity_recovered)
    $app=$null
    # Invalid profiles are checked without creating UI or applying desktop changes.
    $bad=Join-Path $a 'invalid-profile.json';[IO.File]::WriteAllText($bad,'{"format":"overlay-profile","version":99}')
    $check=Start-Process (Join-Path $a 'OmarchyOverlay.exe') -ArgumentList @('--check-profile',('"'+$bad+'"')) -RedirectStandardOutput (Join-Path $root 'profile-check.json') -PassThru -Wait
    Check 'Unsupported profile is rejected by read-only validation' ($check.ExitCode -eq 2)
    Check 'Profile validation does not start a controller' ([RecoverTest]::MainWindow() -eq [IntPtr]::Zero)
    $app=Start-App $a
    [RecoverTest]::Shortcut(0x54);Pump 1200
    Click ([RecoverTest]::MainWindow()) 3;Click ([RecoverTest]::Settings()) 68
    Click ([RecoverTest]::Profiles()) 86;Pump 700
    Check 'Restore-desktop action exits the controller' ($app.HasExited)
    for($i=0;$i -lt 2;$i++){Check "Restore-desktop action restores fixture $i" ((Bounds $forms[$i].Handle) -eq $originals[$i])}
    $app=$null
}catch{$results.Add(@{name='Recovery harness';passed=$false;detail=$_.Exception.ToString()})}
finally{
    if($null -ne $app -and -not $app.HasExited){try{Stop-App $app}catch{}}
    foreach($form in $forms){$form.Close();$form.Dispose()}
    if($null -ne $originalTaskbar){[RecoverTest]::RestoreTaskbar($originalTaskbar)}
    $results|ConvertTo-Json -Depth 6|Set-Content -Encoding UTF8 (Join-Path $root 'recovery-result.json')
}

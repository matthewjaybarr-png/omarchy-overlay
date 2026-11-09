$ErrorActionPreference='Stop'
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
$local=Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'OmarchyOverlay'
$results=[Collections.Generic.List[object]]::new()
$app=$null;$controllerWindow=[IntPtr]::Zero;$forms=@();$created=[Collections.Generic.List[string]]::new();$originalDesktop=$null
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -ReferencedAssemblies System.Windows.Forms -TypeDefinition @'
using System; using System.Runtime.InteropServices;
[ComImport,Guid("A5CD92FF-29BE-454C-8D04-D82879FB3F1B"),InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface IDesktopManager {
 [PreserveSig] int IsWindowOnCurrentVirtualDesktop(IntPtr hwnd,[MarshalAs(UnmanagedType.Bool)] out bool current);
 [PreserveSig] int GetWindowDesktopId(IntPtr hwnd,out Guid id);
 [PreserveSig] int MoveWindowToDesktop(IntPtr hwnd,ref Guid id);
}
public static class RecoverTest {
 public static bool OnCurrent(IntPtr hwnd) {var manager=(IDesktopManager)Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("AA509086-5CA9-4C25-8F95-589D3C07B48A")));try {bool result;return manager.IsWindowOnCurrentVirtualDesktop(hwnd,out result)==0&&result;}finally{Marshal.ReleaseComObject(manager);}}
 public static Guid DesktopId(IntPtr hwnd) {var manager=(IDesktopManager)Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("AA509086-5CA9-4C25-8F95-589D3C07B48A")));try {Guid result;Marshal.ThrowExceptionForHR(manager.GetWindowDesktopId(hwnd,out result));return result;}finally{Marshal.ReleaseComObject(manager);}}
 public static void NativeDesktop(byte key) {keybd_event(0x5b,0,0,UIntPtr.Zero);keybd_event(0x11,0,0,UIntPtr.Zero);keybd_event(key,0,0,UIntPtr.Zero);keybd_event(key,0,2,UIntPtr.Zero);keybd_event(0x11,0,2,UIntPtr.Zero);keybd_event(0x5b,0,2,UIntPtr.Zero);}

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
  [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hwnd);
  public static IntPtr ModuleSettings() { return FindWindow("OmarchyOverlay.ModuleSettings",null); }
 public static IntPtr BarSettings() { return FindWindow("OmarchyOverlay.BarSettings",null); }
  public static IntPtr Profiles() { return FindWindow("OmarchyOverlay.Profiles",null); }
  public static IntPtr Bar() { return FindWindow("OmarchyOverlay.TopBar",null); }
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr hwnd,System.Text.StringBuilder value,int count);
  public static string Caption(IntPtr hwnd) {var text=new System.Text.StringBuilder(4096);GetWindowText(hwnd,text,text.Capacity);return text.ToString();}
  [DllImport("user32.dll",EntryPoint="SendMessageW",CharSet=CharSet.Unicode)] static extern IntPtr ReadText(IntPtr hwnd,uint msg,IntPtr count,System.Text.StringBuilder value);
  public static string ControlText(IntPtr hwnd,int id) {var value=new System.Text.StringBuilder(4096);ReadText(GetDlgItem(hwnd,id),13,(IntPtr)value.Capacity,value);return value.ToString();}
  [DllImport("user32.dll")] public static extern bool RegisterHotKey(IntPtr hwnd,int id,uint modifiers,uint key);
  [DllImport("user32.dll")] public static extern bool UnregisterHotKey(IntPtr hwnd,int id);
  [DllImport("user32.dll",EntryPoint="GetClassLongPtrW")] public static extern IntPtr ClassIcon(IntPtr hwnd,int index);
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] static extern IntPtr LoadLibraryEx(string path,IntPtr file,uint flags);
  [DllImport("kernel32.dll")] static extern IntPtr FindResource(IntPtr module,IntPtr name,IntPtr type);
  [DllImport("kernel32.dll")] static extern IntPtr LoadResource(IntPtr module,IntPtr resource);
  [DllImport("kernel32.dll")] static extern IntPtr LockResource(IntPtr resource);
  [DllImport("kernel32.dll")] static extern uint SizeofResource(IntPtr module,IntPtr resource);
  [DllImport("kernel32.dll")] static extern bool FreeLibrary(IntPtr module);
  public static byte[] Resource(string path,int name,int type) {var module=LoadLibraryEx(path,IntPtr.Zero,0x22);if(module==IntPtr.Zero)throw new Exception("LoadLibraryEx");try{var resource=FindResource(module,(IntPtr)name,(IntPtr)type);if(resource==IntPtr.Zero)throw new Exception("Missing resource");var bytes=new byte[SizeofResource(module,resource)];Marshal.Copy(LockResource(LoadResource(module,resource)),bytes,0,bytes.Length);return bytes;}finally{FreeLibrary(module);}}
  public static IntPtr AboutDialog() { return FindWindow("#32770","About Omarchy Overlay"); }
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
  public static int WorkBottom() {var d=new MONITOR();d.size=(uint)Marshal.SizeOf(d);GetMonitorInfo(MonitorFromWindow(IntPtr.Zero,1),ref d);return d.work.b;}
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
    $script:controllerWindow=$hwnd
    Pump 700;return $process
}
function Stop-App($process){[void][RecoverTest]::PostMessage($controllerWindow,0x312,[IntPtr]105,[IntPtr]::Zero);$until=[DateTime]::UtcNow.AddSeconds(8);while(-not $process.HasExited -and [DateTime]::UtcNow -lt $until){Pump 100};Pump 700;if(-not $process.HasExited){throw 'Controller did not exit.'}}
function Read-Profile($folder){return Get-Content -Raw (Join-Path $folder 'overlay-profile.json')|ConvertFrom-Json}

function State {
 $key='HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\VirtualDesktops'
 $raw=Get-ItemProperty $key -ErrorAction SilentlyContinue
 $ids=@();if($null -ne $raw -and $null -ne $raw.VirtualDesktopIDs){$bytes=[byte[]]$raw.VirtualDesktopIDs;for($i=0;$i+16 -le $bytes.Length;$i+=16){$ids+=([Guid]::new([byte[]]$bytes[$i..($i+15)])).ToString()}}
 $current=$null
 foreach($path in @($key,('HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\SessionInfo\'+[Diagnostics.Process]::GetCurrentProcess().SessionId+'\VirtualDesktops'))){$data=Get-ItemProperty $path -ErrorAction SilentlyContinue;if($null -ne $data -and $null -ne $data.CurrentVirtualDesktop){$id=([Guid]::new([byte[]]$data.CurrentVirtualDesktop)).ToString();if($ids -contains $id){$current=$id;break}}}
 return @{ids=$ids;current=$current}
}
function Wait-Current([string]$id){$until=[DateTime]::UtcNow.AddSeconds(10);do{Pump 150;$state=State}while($state.current -ne $id -and [DateTime]::UtcNow -lt $until);return $state.current -eq $id}
function Select-Desktop([int]$index){[RecoverTest]::Shortcut([byte](0x31+$index));return Wait-Current ((State).ids[$index])}
function Input-Text($hwnd,[int]$id,[string]$text) {
 $h=[RecoverTest]::GetDlgItem($hwnd,$id)
 if([RecoverTest]::SendMessage($h,0x146,[IntPtr]::Zero,[IntPtr]::Zero).ToInt64() -gt 0){
  $index=[RecoverTest]::Text($h,0x158,[IntPtr](-1),$text)
  if($index.ToInt64() -lt 0){throw "Dropdown option not found: $id $text"}
  [void][RecoverTest]::SendMessage($h,0x14e,$index,[IntPtr]::Zero)
 }else{[void][RecoverTest]::Text($h,12,[IntPtr]::Zero,$text)}
}
function Capture($hwnd,[string]$file){$r=[RecoverTest+RECT]::new();[void][RecoverTest]::GetWindowRect($hwnd,[ref]$r);$bitmap=[Drawing.Bitmap]::new(($r.r-$r.l),($r.b-$r.t));$graphics=[Drawing.Graphics]::FromImage($bitmap);try{$graphics.CopyFromScreen($r.l,$r.t,0,0,$bitmap.Size);$bitmap.Save((Join-Path $root $file),[Drawing.Imaging.ImageFormat]::Png)}finally{$graphics.Dispose();$bitmap.Dispose()}}
try {
 [void][RecoverTest]::SetProcessDpiAwarenessContext([IntPtr](-4))
 $principal=[Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
 Check 'Workspace tests use a limited desktop token' (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))
 Check 'No existing Omarchy Overlay controller to disturb' ([RecoverTest]::MainWindow() -eq [IntPtr]::Zero)
 $f=[Windows.Forms.Form]::new();$f.Text='Omarchy Overlay desktop anchor';$f.SetBounds(180,150,520,340);$f.Show();$forms+=,$f;Pump
 $originalDesktop=[RecoverTest]::DesktopId($f.Handle).ToString()
 # Only create missing test desktops. Their exact GUIDs are tracked for cleanup.
 $initial=State
 while(@((State).ids).Count -lt 4){
  $before=State;[RecoverTest]::NativeDesktop(0x44);Pump 1400;$after=State
  $new=@($after.ids|Where-Object{$_ -notin $before.ids -and $_ -ne $originalDesktop})
  foreach($id in $new){if(-not $created.Contains($id)){$created.Add($id)}}
  if($after.ids.Count -le $before.ids.Count){throw 'Windows did not create a test desktop.'}
 }
 Check 'Four real Windows desktops are available' (@((State).ids).Count -ge 4)
 $folder=Join-Path $root ('bar-workspaces-'+[Guid]::NewGuid().ToString('N'));New-Item -ItemType Directory -Force $folder|Out-Null
 Copy-Item (Join-Path $root 'OmarchyOverlay.exe') (Join-Path $folder 'OmarchyOverlay.exe') -Force
 $profilePath=Join-Path $folder 'overlay-profile.json'
 $originalProfile=$null

 $profile=@{format='overlay-profile';version=1;id=('workspace-test-'+[Guid]::NewGuid().ToString('N'));settings=@{inactive_opacity=100}}
 [IO.File]::WriteAllText($profilePath,($profile|ConvertTo-Json -Depth 12),[Text.UTF8Encoding]::new($false))
 Seed-Paused $folder;$originalProfile=[IO.File]::ReadAllText($profilePath)
 $app=Start-App $folder
 $bar=[RecoverTest]::Bar();$r=[RecoverTest+RECT]::new();[void][RecoverTest]::GetWindowRect($bar,[ref]$r)
 $dpi=[RecoverTest]::GetDpiForWindow($bar)
 Check 'Compact default bar scales its 30 logical pixels to display DPI' (($r.b-$r.t) -eq [Math]::Floor(30*$dpi/96)) ("height=$($r.b-$r.t); dpi=$dpi")
 Check 'Bar belongs to the active native desktop' ([RecoverTest]::OnCurrent($bar))
 Click $controllerWindow 3;Click ([RecoverTest]::Settings()) 69
 $panel=[RecoverTest]::BarSettings()
 Check 'Bar and workspace settings open' ([RecoverTest]::IsWindowVisible($panel))
 Capture $panel 'overlay-bar-settings.png'
 Input-Text $panel 204 '40';Input-Text $panel 206 '9';[void][RecoverTest]::SendMessage([RecoverTest]::GetDlgItem($panel,219),0x14e,[IntPtr]2,[IntPtr]::Zero);[void][RecoverTest]::PostMessage($panel,0x111,[IntPtr](219+(1 -shl 16)),[RecoverTest]::GetDlgItem($panel,219));Pump
 Input-Text $panel 201 'menu, divider, workspaces, divider, tiling'
 Input-Text $panel 203 'network, divider, sound, power, controls';Click $panel 211
 $saved=Get-Content -Raw $profilePath|ConvertFrom-Json
 Check 'Workspace labels save to the portable profile' (($saved.settings.bar.workspace_labels -join ',') -eq 'CAD,Code,Office,Files')
 Check 'Module order saves to the portable profile' (($saved.settings.bar.right -join ',') -eq 'network,divider,sound,power,controls')
 $bar=[RecoverTest]::Bar();[void][RecoverTest]::GetWindowRect($bar,[ref]$r)
 Check 'Height changes the actual appbar reservation live' (($r.b-$r.t) -eq [Math]::Floor(40*$dpi/96) -and [RecoverTest]::WorkTop() -eq $r.b)
 # Restore compact defaults before the workspace click tests.
 Click $panel 212;Click $panel 211
 [void][RecoverTest]::SendMessage($panel,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 [void][RecoverTest]::SendMessage([RecoverTest]::Settings(),0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 [void][RecoverTest]::SendMessage([RecoverTest]::MainWindow(),0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 for($i=0;$i -lt 4;$i++){
  Check "Real Ctrl+Alt+$($i+1) selects desktop $($i+1)" (Select-Desktop $i)
  Pump 700
  Check "Bar follows desktop $($i+1)" ([RecoverTest]::OnCurrent([RecoverTest]::Bar()))
 }
 # MB branding and in-place compatibility against the old-format fixture profile.
 $exe=Join-Path $folder 'OmarchyOverlay.exe'
 $version=[Diagnostics.FileVersionInfo]::GetVersionInfo($exe)
 Check 'Executable exposes Omarchy Overlay product and filename metadata' ($version.ProductName -eq 'Omarchy Overlay' -and $version.OriginalFilename -eq 'OmarchyOverlay.exe' -and $version.ProductVersion -match '^\d+\.\d+\.\d+$') "$($version.ProductName) $($version.OriginalFilename) $($version.ProductVersion)"
 Check 'Dashboard carries the Omarchy Overlay product name' ([RecoverTest]::Caption($controllerWindow) -like 'Omarchy Overlay*')
 Check 'Window class loads the embedded application icon' ([RecoverTest]::ClassIcon($controllerWindow,-14) -ne [IntPtr]::Zero)
 $group=[RecoverTest]::Resource($exe,101,14);$sizes=@();for($i=0;$i -lt [BitConverter]::ToUInt16($group,4);$i++){$size=[int]$group[6+14*$i];if($size -eq 0){$size=256};$sizes+=$size}
 Check 'EXE embeds all nine icon sizes made from Omarchy Overlay.ico (0.19)' (((@($sizes|Sort-Object)) -join ',') -eq '16,20,24,32,40,48,64,128,256') ($sizes -join ',')
 $manifest=[Text.Encoding]::UTF8.GetString([RecoverTest]::Resource($exe,1,24))
 Check 'Rebrand retains the normal-user manifest' ($manifest -match 'asInvoker' -and $manifest -match 'uiAccess="false"')
 # 0.21: visual styles (Common Controls 6) are what let the dropdowns and lists use Windows' dark styles.
 Check 'Manifest asks for Common Controls 6' ($manifest -match 'Microsoft.Windows.Common-Controls' -and $manifest -match '6\.0\.0\.0')
 Check 'Old portable profile is preserved for rollback before editing' ([IO.File]::ReadAllText([IO.Path]::ChangeExtension($profilePath,'pre-0.10.json')) -eq $originalProfile)
 Click $controllerWindow 28
 Check 'About identifies the personal prototype and its version' ([RecoverTest]::ControlText([RecoverTest]::AboutDialog(),65535) -match ('Omarchy Overlay '+[regex]::Escape("$($v=[Diagnostics.FileVersionInfo]::GetVersionInfo($exe);$v.FileMajorPart).$($v.FileMinorPart).$($v.FileBuildPart)")))
 [void][RecoverTest]::PostMessage([RecoverTest]::AboutDialog(),0x10,[IntPtr]::Zero,[IntPtr]::Zero);Pump 300
 Check 'About closes before the workspace tests continue' ([RecoverTest]::AboutDialog() -eq [IntPtr]::Zero)
 Click $controllerWindow 3;$settings=[RecoverTest]::Settings()
 [void][RecoverTest]::SendMessage($settings,0x115,[IntPtr]7,[IntPtr]::Zero);Pump 200
 $panelRect=[RecoverTest+RECT]::new();[void][RecoverTest]::GetWindowRect($settings,[ref]$panelRect)
 $editorRect=[RecoverTest+RECT]::new();[void][RecoverTest]::GetWindowRect([RecoverTest]::GetDlgItem($settings,71),[ref]$editorRect)
 $statusRect=[RecoverTest+RECT]::new();[void][RecoverTest]::GetWindowRect([RecoverTest]::GetDlgItem($settings,74),[ref]$statusRect)
 Check 'Settings fits the monitor work area on a shorter screen' ($panelRect.t -ge [RecoverTest]::WorkTop() -and $panelRect.b -le [RecoverTest]::WorkBottom())
 Check 'Scrolling exposes the shortcut editor and its status inside Settings' ($editorRect.t -gt $panelRect.t -and $statusRect.b -lt $panelRect.b)
 Check 'Old profile gains the independent default without resetting bar choices' ([RecoverTest]::ControlText($settings,71) -eq 'Ctrl+Alt+F11' -and (Read-Profile $folder).settings.bar.workspace_labels.Count -eq 4)
 Check 'Retired Rainmeter removal control is absent' ([RecoverTest]::GetDlgItem($settings,70) -eq [IntPtr]::Zero)
 [void][RecoverTest]::PostMessage($settings,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 # The PC may already have more than four desktops; wrap from the real last one.
 $last=@((State).ids)[-1];for($n=0;(State).current -ne $last -and $n -lt 10;$n++){[RecoverTest]::NativeDesktop(0x27);Pump 700};Pump 1500
 Check 'The last Windows desktop is current before global workspace cycling' ((State).current -eq $last)
 $cycleForm=[Windows.Forms.Form]::new();$cycleForm.Text='Workspace cycle target';$cycleForm.SetBounds(220,190,520,340);$cycleForm.Show();$forms+=,$cycleForm;Pump 300
 $focused=$false;for($n=0;-not $focused -and $n -lt 3;$n++){$focused=[RecoverTest]::SetForegroundWindow($cycleForm.Handle);if(-not $focused){Pump 500}}
 Check 'An external fixture is focused before global workspace cycling' $focused
 [RecoverTest]::Shortcut(0x7a)
 Check 'Global Ctrl+Alt+F11 wraps last desktop to first while another app is focused' (Wait-Current ((State).ids[0]))
 Pump 1500;Check 'Bar follows wrapped workspace cycle' ([RecoverTest]::OnCurrent([RecoverTest]::Bar()))
 [RecoverTest]::Shortcut(0x7a)
 Check 'Default workspace cycle selects the next desktop' (Wait-Current ((State).ids[1]))
 Pump 1500
 Click $controllerWindow 3;$settings=[RecoverTest]::Settings()
 Input-Text $settings 71 'Ctrl+Alt+F10';Click $settings 73
 Check 'Shortcut editor saves an independent custom chord' ((Read-Profile $folder).settings.cycle_workspace_shortcut -eq 'Ctrl+Alt+F10')
 [void][RecoverTest]::PostMessage($settings,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 [RecoverTest]::Shortcut(0x79)
 Check 'Edited global shortcut cycles the actual Windows desktop' (Wait-Current ((State).ids[2]));Pump 1500
 Click $controllerWindow 3;$settings=[RecoverTest]::Settings()
 $blocked=[RecoverTest]::RegisterHotKey($f.Handle,902,3,0x78)
 Check 'Fixture owns a conflicting shortcut for the test' $blocked
 Input-Text $settings 71 'Ctrl+Alt+F9';Click $settings 73
 Check 'External registration conflict is visible in Settings' ([RecoverTest]::ControlText($settings,74) -like 'Unavailable:*')
 [void][RecoverTest]::UnregisterHotKey($f.Handle,902)
 Input-Text $settings 71 'Ctrl+Alt+T';Click $settings 73
 Check 'Conflicts with Omarchy Overlay actions are detected before registration' ([RecoverTest]::ControlText($settings,74) -like 'Unavailable:*')
 Input-Text $settings 71 'Ctrl+Alt+F12';Click $settings 73
 Check 'Reserved F12 is rejected with a visible reason' ([RecoverTest]::ControlText($settings,74) -match 'reserves F12' -and (Read-Profile $folder).settings.cycle_workspace_shortcut -eq 'Ctrl+Alt+T')
 Input-Text $settings 71 '';Click $settings 73
 Check 'Blank shortcut disables cycling and releases the chord' ([RecoverTest]::ControlText($settings,74) -match 'disabled' -and (Read-Profile $folder).settings.cycle_workspace_shortcut -eq '')
 Check 'Disabled shortcut is available to another application' ([RecoverTest]::RegisterHotKey($f.Handle,903,3,0x79))
 [void][RecoverTest]::UnregisterHotKey($f.Handle,903)
 Click $settings 75;Click $settings 73
 Check 'Default button restores the chosen Ctrl+Alt+F11 chord' ((Read-Profile $folder).settings.cycle_workspace_shortcut -eq 'Ctrl+Alt+F11')
 [void][RecoverTest]::SendMessage($settings,0x115,[IntPtr]7,[IntPtr]::Zero);Pump 200
 Capture $settings 'overlay-shortcut-settings.png'
 [void][RecoverTest]::PostMessage($settings,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 Check 'Workspace cycle returns to the fourth desktop before native-navigation regression' (Select-Desktop 3)
 Pump 1500
 # A native shortcut outside Omarchy Overlay must update the bar as well.
 [RecoverTest]::NativeDesktop(0x25)
 Check 'External Windows desktop switch is observed' (Wait-Current ((State).ids[2]))
 Pump 700;Check 'Bar follows an external desktop switch' ([RecoverTest]::OnCurrent([RecoverTest]::Bar()))
 # MENU widens the left group; x=93 is inside the first workspace cell.
 $bar=[RecoverTest]::Bar();[void][RecoverTest]::PostMessage($bar,0x202,[IntPtr]::Zero,[IntPtr]([int](93*$dpi/96)+([int](12*$dpi/96) -shl 16)))
 Check 'Clicking workspace 1 selects the first actual Windows desktop' (Wait-Current ((State).ids[0]))
 Pump 700;Capture ([RecoverTest]::Bar()) 'overlay-omarchy-bar.png'
 # Exercise the optional compact previous/next controls against observed desktop IDs.
 Click $controllerWindow 14;$panel=[RecoverTest]::BarSettings();Click $panel 214
 $styles=[RecoverTest]::ModuleSettings()
 [void][RecoverTest]::SendMessage([RecoverTest]::GetDlgItem($styles,405),0x14e,[IntPtr]2,[IntPtr]::Zero);Click $styles 451
 [void][RecoverTest]::PostMessage($styles,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 Input-Text $panel 201 'workspace-nav';Input-Text $panel 202 '';Input-Text $panel 203 '';Click $panel 211
 [void][RecoverTest]::PostMessage($panel,0x10,[IntPtr]::Zero,[IntPtr]::Zero);Pump 500
 $bar=[RecoverTest]::Bar()
 [void][RecoverTest]::PostMessage($bar,0x202,[IntPtr]::Zero,[IntPtr]([int](20*$dpi/96)+([int](14*$dpi/96) -shl 16)));Pump 500
 Check 'Previous-desktop module stops at the first desktop' ((State).current -eq (State).ids[0])
 # The bar holds only Previous then Next, and Previous is a no-op on the first desktop, so walk right
 # until Next is hit: cell widths follow the theme font, so a fixed x breaks when the font changes.
 $next=$false;foreach($x in 40..200|Where-Object{$_ % 8 -eq 0}){[void][RecoverTest]::PostMessage($bar,0x202,[IntPtr]::Zero,[IntPtr]([int]($x*$dpi/96)+([int](14*$dpi/96) -shl 16)));$until=[DateTime]::UtcNow.AddMilliseconds(1200);do{Pump 150;$now=(State).current}while($now -eq (State).ids[0] -and [DateTime]::UtcNow -lt $until);if($now -ne (State).ids[0]){$next=$now -eq (State).ids[1];break}}
 Check 'Next-desktop icon selects the next observed desktop' $next
 Pump 500;Check 'Bar follows the next-desktop module' ([RecoverTest]::OnCurrent([RecoverTest]::Bar()))
 [void][RecoverTest]::PostMessage([RecoverTest]::Bar(),0x202,[IntPtr]::Zero,[IntPtr]([int](20*$dpi/96)+([int](14*$dpi/96) -shl 16)))
 Check 'Previous-desktop icon returns to the preceding desktop' (Wait-Current ((State).ids[0]))
 # The anchor stays on its original desktop; Omarchy Overlay never shuffles other app windows between desktops.
 Check 'Existing app keeps its native desktop assignment' ([RecoverTest]::DesktopId($f.Handle).ToString() -eq $originalDesktop)
 Stop-App $app;$app=$null
 $app=Start-App $folder;Click $controllerWindow 3
 Check 'Shortcut and bar settings survive a restart' ([RecoverTest]::ControlText([RecoverTest]::Settings(),71) -eq 'Ctrl+Alt+F11')
 [void][RecoverTest]::PostMessage([RecoverTest]::Settings(),0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 [RecoverTest]::Shortcut(0x7a)
 Check 'Saved global cycle shortcut binds again after restart' (Wait-Current ((State).ids[1]));Pump 1500
 Stop-App $app;$app=$null
 Check 'Exit removes the bar on every desktop' ([RecoverTest]::Bar() -eq [IntPtr]::Zero)
 Check 'Portable workspace labels remain saved after exit' ((Get-Content -Raw $profilePath|ConvertFrom-Json).settings.bar.workspace_labels.Count -eq 4)
} catch {
 $results.Add(@{name='Workspace harness';passed=$false;detail=$_.Exception.ToString()});throw
} finally {
 $results|ConvertTo-Json -Depth 8|Set-Content -Encoding UTF8 (Join-Path $root 'workspaces-result.json')
 if([RecoverTest]::AboutDialog() -ne [IntPtr]::Zero){[void][RecoverTest]::PostMessage([RecoverTest]::AboutDialog(),0x10,[IntPtr]::Zero,[IntPtr]::Zero);Pump 300}
 if($null -ne $app -and -not $app.HasExited){Stop-App $app}
 foreach($form in $forms){$form.Close();$form.Dispose()}
 # Remove only desktops created by this test, then return to the original one.
 foreach($id in $created){
  $state=State;$index=[Array]::IndexOf([object[]]$state.ids,$id)
  if($index -ge 0){
   for($tries=0;$tries -lt 20 -and (State).current -ne $id;$tries++){$state=State;$at=[Array]::IndexOf([object[]]$state.ids,$state.current);[RecoverTest]::NativeDesktop($(if($index -lt $at){0x25}else{0x27}));Pump 700}
   if((State).current -eq $id){[RecoverTest]::NativeDesktop(0x73);Pump 1100}
  }
 }
 if($null -ne $originalDesktop){for($tries=0;$tries -lt 20 -and (State).current -ne $originalDesktop;$tries++){$state=State;$index=[Array]::IndexOf([object[]]$state.ids,$originalDesktop);$at=[Array]::IndexOf([object[]]$state.ids,$state.current);if($index -lt 0){break};[RecoverTest]::NativeDesktop($(if($index -lt $at){0x25}else{0x27}));Pump 700}}
 $results|ConvertTo-Json -Depth 8|Set-Content -Encoding UTF8 (Join-Path $root 'workspaces-result.json')
}

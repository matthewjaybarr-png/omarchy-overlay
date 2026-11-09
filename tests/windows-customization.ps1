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
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr hwnd);
 public static IntPtr Help() { return FindWindow("OmarchyOverlay.Help",null); }
 public static IntPtr Launcher() { return FindWindow("OmarchyOverlay.Launcher",null); }
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
function Menu{[RecoverTest]::Overlay()}
function Menu-Name{[RecoverTest]::Caption((Menu))}
function Menu-Key([int]$vk){[void][RecoverTest]::PostMessage((Menu),0x100,[IntPtr]$vk,[IntPtr]::Zero);Pump 250}
function Menu-Type([string]$text){foreach($c in $text.ToCharArray()){[void][RecoverTest]::PostMessage((Menu),0x102,[IntPtr][int]$c,[IntPtr]::Zero)};Pump 300}
function Menu-Go([string]$label){Menu-Type $label;Menu-Key 0x0d;Pump 300}
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
 Check 'No existing controller to disturb' ([RecoverTest]::MainWindow() -eq [IntPtr]::Zero)
 $folder=Join-Path $root ('custom-'+[Guid]::NewGuid().ToString('N'));New-Item -ItemType Directory "$folder\tools" -Force|Out-Null
 Copy-Item "$root\OmarchyOverlay.exe" "$folder\OmarchyOverlay.exe"
 $profilePath=Join-Path $folder 'overlay-profile.json'
 [IO.File]::WriteAllText($profilePath,'{"format":"overlay-profile","version":1,"id":"custom-test","settings":{"cycle_workspace_shortcut":"Ctrl+Alt+F11"}}')
 $fixture='using System;using System.IO;public class CustomFixture{public static void Main(string[] args){File.WriteAllText(args[0],Environment.CurrentDirectory);}}'
 Add-Type -TypeDefinition $fixture -OutputAssembly "$folder\tools\fixture.exe" -OutputType WindowsApplication
 $app=Start-App $folder
 Check '0.10 profile receives a separate rollback backup' (Test-Path "$folder\overlay-profile.pre-0.11.json")
 Click $controllerWindow 5;$help=[RecoverTest]::Help()
 Check 'Shortcut editor opens with all actions' ([RecoverTest]::SendMessage([RecoverTest]::GetDlgItem($help,501),0x18b,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32() -ge 39)
 [void][RecoverTest]::SendMessage([RecoverTest]::GetDlgItem($help,501),0x186,[IntPtr]1,[IntPtr]::Zero)
 Click $help (501+(1 -shl 16))
 Input-Text $help 502 'Ctrl+Alt+F10';Click $help 503
 Check 'Individual launcher shortcut saves' ((Read-Profile $folder).settings.shortcut_overrides.launcher -eq 'Ctrl+Alt+F10')
 # 0.16: the Launcher action opens the Omarchy Overlay menu.
 [RecoverTest]::Shortcut(0x79);Pump 700
 Check 'Actual custom global key opens the Omarchy Overlay menu' ([RecoverTest]::IsWindowVisible([RecoverTest]::Overlay()))
 [RecoverTest]::CloseMenu();Pump 300
 Input-Text $help 502 'Ctrl+Alt+T';Click $help 503
 Check 'Internal collision keeps previous binding' ([RecoverTest]::ControlText($help,507) -match 'Already assigned' -and (Read-Profile $folder).settings.shortcut_overrides.launcher -eq 'Ctrl+Alt+F10')
 Input-Text $help 502 'Ctrl+Alt+F12';Click $help 503
 Check 'Reserved F12 does not replace a working binding' ([RecoverTest]::ControlText($help,507) -match 'reserves F12')
 Click $help 505
 Check 'Disable saves an empty override' ((Read-Profile $folder).settings.shortcut_overrides.launcher -eq '')
 Click $help 504
 Check 'Use default removes the override' ($null -eq (Read-Profile $folder).settings.shortcut_overrides.launcher)
 Input-Text $help 502 'Ctrl+Alt+F10';Click $help 503
 Capture $help 'mb-shortcuts-editor.png'
 [void][RecoverTest]::PostMessage($help,0x10,[IntPtr]::Zero,[IntPtr]::Zero)
 # 0.17 (3.3): custom apps are the "Custom apps" page of the Omarchy Overlay menu. The menu is drawn
 # by hand; its window name is "<page> | <highlighted row> | <detail>" (see overlay.rs).
 Click $controllerWindow 2;Click ([RecoverTest]::Launcher()) 16;Pump 400
 Check 'Visible Manage apps button opens the Custom apps page' ([RecoverTest]::IsWindowVisible((Menu)) -and (Menu-Name) -like 'Custom apps*') (Menu-Name)
 Check 'Always-on-top picker is hidden while managing applications' (-not [RecoverTest]::IsWindowVisible([RecoverTest]::Launcher()))
 Menu-Go 'Add by typing a path';Menu-Type 'tools\fixture.exe';Menu-Key 0x0d;Pump 300
 Check 'Typed path adds an entry and opens it' ((Menu-Name) -like 'Custom app |*') (Menu-Name)
 Menu-Go 'Name:';Menu-Type 'Portable Fixture';Menu-Key 0x0d
 Menu-Go 'Working folder';Menu-Type 'tools';Menu-Key 0x0d
 $marker=Join-Path $folder 'launched.txt';Menu-Go 'Arguments';Menu-Type ('"'+$marker+'"');Menu-Key 0x0d
 $saved=Read-Profile $folder
 Check 'Relative app, arguments and working folder save in portable profile' ($saved.settings.applications[0].name -eq 'Portable Fixture' -and $saved.settings.applications[0].target -eq 'tools\fixture.exe' -and $saved.settings.applications[0].working_directory -eq 'tools' -and $saved.settings.applications[0].arguments -eq ('"'+$marker+'"')) ($saved.settings.applications|ConvertTo-Json -Compress)
 Capture (Menu) 'mb-custom-applications.png'
 Menu-Key 0x1b;Menu-Key 0x1b
 Check 'Esc closes the Custom apps page' (-not [RecoverTest]::IsWindowVisible((Menu)))
 Click $controllerWindow 2;Pump 400
 Input-Text ([RecoverTest]::Launcher()) 10 'Portable Fixture';Pump 300;Click ([RecoverTest]::Launcher()) 12
 # A freshly compiled .NET fixture can take a few seconds to start on a cold guest.
 $deadline=[DateTime]::UtcNow.AddSeconds(5);while(-not (Test-Path $marker) -and [DateTime]::UtcNow -lt $deadline){Pump 200}
 Check 'Custom picker entry launches the native fixture with its arguments' (Test-Path $marker) ('query='+[RecoverTest]::ControlText([RecoverTest]::Launcher(),10)+' detail='+[RecoverTest]::ControlText([RecoverTest]::Launcher(),15))
 Check 'Custom launch receives the requested working directory' ((Get-Content $marker -Raw) -eq "$folder\tools")
 Click $controllerWindow 2;Input-Text ([RecoverTest]::Launcher()) 10 'Portable Fixture';Click ([RecoverTest]::Launcher()) 13
 Click ([RecoverTest]::Launcher()) 16;Pump 400
 Menu-Go 'Add by typing a path';Menu-Type 'missing\app.exe';Menu-Key 0x0d;Pump 300
 Menu-Go 'Name:';Menu-Type 'Absent work app';Menu-Key 0x0d
 Check 'Missing apps can be retained for another PC' ((Read-Profile $folder).settings.applications.Count -eq 2)
 Menu-Key 0x1b
 Check 'Custom apps page marks a missing target' ((Menu-Name) -like 'Custom apps |*') (Menu-Name)
 Menu-Type 'Absent work app'
 Check 'Missing target is labelled on the Custom apps page' ((Menu-Name) -like '*Absent work app | missing on this PC') (Menu-Name)
 Menu-Key 0x1b
 Click $controllerWindow 2;Pump 400
 Input-Text ([RecoverTest]::Launcher()) 10 'Absent work app';Pump 300
 Check 'Missing target disables the visible Open button' (-not [RecoverTest]::IsWindowEnabled([RecoverTest]::GetDlgItem([RecoverTest]::Launcher(),12)))
 Check 'Picker explains missing custom target' ([RecoverTest]::ControlText([RecoverTest]::Launcher(),15) -match 'Missing')
 Stop-App $app;$app=$null
 $moved=$folder+'-moved';Copy-Item $folder $moved -Recurse
 # Without the original tools folder, "available" can only come from the moved profile.
 Remove-Item (Join-Path $folder 'tools') -Recurse -Force
 $app=Start-App $moved;[RecoverTest]::Shortcut(0x79);Pump 600
 Check 'Individual shortcut survives restart and profile move' ([RecoverTest]::IsWindowVisible([RecoverTest]::Overlay()))
 [RecoverTest]::CloseMenu();Pump 300;Click $controllerWindow 2;Pump 600
 Input-Text ([RecoverTest]::Launcher()) 10 'Portable Fixture';Pump 300
 Check 'Custom favorite remains pinned after profile move' ([RecoverTest]::ControlText([RecoverTest]::Launcher(),13) -eq 'Unpin favorite')
 Click ([RecoverTest]::Launcher()) 16;Pump 400
 Menu-Go 'Portable Fixture';Menu-Type 'Path:'
 Check 'Relative target resolves against the moved profile' ((Menu-Name) -like 'Custom app | Path: tools\fixture.exe | available') (Menu-Name)
 Menu-Key 0x08;Menu-Key 0x08;Menu-Key 0x08;Menu-Key 0x08;Menu-Key 0x08
 Menu-Go 'Delete entry';Menu-Key 0x0d;Pump 300
 Check 'Deleting entry leaves its executable intact' ((Read-Profile $moved).settings.applications.Count -eq 1 -and (Test-Path "$moved\tools\fixture.exe")) ((Read-Profile $moved).settings.applications.Count)
} catch { $results.Add(@{name='Customization harness';passed=$false;detail=$_.Exception.ToString()}) }
finally {
 if($null -ne $app -and -not $app.HasExited){Stop-App $app}
 $results|ConvertTo-Json -Depth 8|Set-Content -Encoding UTF8 (Join-Path $root 'customization-result.json')
}

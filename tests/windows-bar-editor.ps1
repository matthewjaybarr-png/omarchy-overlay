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
 public static IntPtr Rules(){return FindWindow("OmarchyOverlay.AppRules",null);}
 public static string Item(IntPtr h,int i){var t=new System.Text.StringBuilder(4096);ReadText(h,0x189,(IntPtr)i,t);return t.ToString();}

 public static bool OnCurrent(IntPtr hwnd) {var manager=(IDesktopManager)Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("AA509086-5CA9-4C25-8F95-589D3C07B48A")));try {bool result;return manager.IsWindowOnCurrentVirtualDesktop(hwnd,out result)==0&&result;}finally{Marshal.ReleaseComObject(manager);}}
 public static Guid DesktopId(IntPtr hwnd) {var manager=(IDesktopManager)Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("AA509086-5CA9-4C25-8F95-589D3C07B48A")));try {Guid result;Marshal.ThrowExceptionForHR(manager.GetWindowDesktopId(hwnd,out result));return result;}finally{Marshal.ReleaseComObject(manager);}}
 public static void NativeDesktop(byte key) {keybd_event(0x5b,0,0,UIntPtr.Zero);keybd_event(0x11,0,0,UIntPtr.Zero);keybd_event(key,0,0,UIntPtr.Zero);keybd_event(key,0,2,UIntPtr.Zero);keybd_event(0x11,0,2,UIntPtr.Zero);keybd_event(0x5b,0,2,UIntPtr.Zero);}

  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int l,t,r,b; }
  [StructLayout(LayoutKind.Sequential)] struct APPBAR { public uint size; public IntPtr hwnd; public uint callback,edge; public RECT rect; public IntPtr data; }
  [StructLayout(LayoutKind.Sequential)] struct MONITOR { public uint size;public RECT screen,work;public uint flags; }
  [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string cls,string title);
  public static IntPtr MainWindow() { return FindWindow("OmarchyOverlay.Main",null); }
  public static IntPtr Overlay() { return FindWindow("OmarchyOverlay.Overlay", null); }
  // 0.16 opens the Keyspace menu at startup; the 0.15 dashboard stays hidden until asked for.
  public static bool Ready(IntPtr main) { return main != IntPtr.Zero && (IsWindowVisible(main) || IsWindowVisible(Overlay())); }
  [DllImport("user32.dll", EntryPoint="PostMessageW")] static extern bool PostToKeyspace(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
  public static void CloseMenu() { IntPtr menu = Overlay(); if (IsWindowVisible(menu)) PostToKeyspace(menu, 0x100, (IntPtr)0x1b, IntPtr.Zero); }
  [DllImport("user32.dll", EntryPoint="ShowWindow")] static extern bool ShowKeyspaceWindow(IntPtr hwnd, int command);
  public static bool ShowDashboard(IntPtr main) { ShowKeyspaceWindow(main, 1); return IsWindowVisible(main); }
  [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr hwnd);
  public static IntPtr Apps() { return FindWindow("OmarchyOverlay.Applications",null); }
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
function Seed-Paused([string]$folder){$path=Join-Path $folder 'keyspace-profile.json';if(-not (Test-Path $path)){return};$p=[IO.File]::ReadAllText($path)|ConvertFrom-Json;if($null -eq $p.settings){$p|Add-Member settings ([pscustomobject]@{})};if($null -ne $p.settings.PSObject.Properties['start_tiling']){return};$p.settings|Add-Member start_tiling $false;[IO.File]::WriteAllText($path,($p|ConvertTo-Json -Depth 20),[Text.UTF8Encoding]::new($false))}
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
function Read-Profile($folder){return Get-Content -Raw (Join-Path $folder 'keyspace-profile.json')|ConvertFrom-Json}

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

function Menu{[RecoverTest]::Overlay()}
function Menu-Name{[RecoverTest]::Caption((Menu))}
function Menu-Key([int]$vk){[void][RecoverTest]::PostMessage((Menu),0x100,[IntPtr]$vk,[IntPtr]::Zero);Pump 250}
function Menu-Type([string]$text){foreach($c in $text.ToCharArray()){[void][RecoverTest]::PostMessage((Menu),0x102,[IntPtr][int]$c,[IntPtr]::Zero)};Pump 300}
function Menu-Go([string]$label){Menu-Type $label;Menu-Key 0x0d;Pump 300}
try {
 [void][RecoverTest]::SetProcessDpiAwarenessContext([IntPtr](-4))
 Check 'No controller to disturb' ([RecoverTest]::MainWindow() -eq [IntPtr]::Zero)
 $originalTaskbar=[RecoverTest]::Taskbar()
 $folder=Join-Path $root ('bar-editor-'+[Guid]::NewGuid().ToString('N'));New-Item -ItemType Directory $folder|Out-Null;Copy-Item "$root\OmarchyOverlay.exe" "$folder\OmarchyOverlay.exe"
 $profile=@{format='keyspace-profile';version=1;id='bar-editor-test';settings=@{bar=@{height=24;font_size=12;spacing=3;font_family='JetBrains Mono NL';left=@('tasks');center=@('clock');right=@('controls');task_buttons=2}}}
 [IO.File]::WriteAllText("$folder\keyspace-profile.json",($profile|ConvertTo-Json -Depth 8),[Text.UTF8Encoding]::new($false))
 # 0.17 (3.3): the bar items editor is the "Bar items" page of the Keyspace menu. The menu is
 # drawn by hand; its window name is "<page> | <highlighted row> | <detail>" (see overlay.rs).
 $app=Start-App $folder;Click $controllerWindow 14;$settings=[RecoverTest]::BarSettings();Click $settings 218;Pump 500
 Check 'Bar items page opens from Bar settings' ([RecoverTest]::IsWindowVisible((Menu)) -and (Menu-Name) -like 'Bar items*') (Menu-Name)
 Menu-Go 'Left section'
 Check 'Legacy tasks migrate to independent drawer and app buttons' ((Menu-Name) -like 'Left section | Open-window drawer*') (Menu-Name)
 Menu-Key 0x28
 Check 'Running apps stay a separate module' ((Menu-Name) -like 'Left section | Running apps*') (Menu-Name)
 [void][RecoverTest]::SetForegroundWindow((Menu));Capture (Menu) 'bar-editor.png'
 Menu-Go 'Add a module to this section'
 $catalog=@();for($i=0;$i -lt 40;$i++){$n=Menu-Name;if($n -in $catalog){break};$catalog+=$n;Menu-Key 0x28}
 Check 'Available module catalog includes native Windows controls and scrolling' ($catalog.Count -eq 23) ("count=$($catalog.Count)")
 Check 'Scrolling navigation is discoverable in the module catalog' (@($catalog|Where-Object{$_ -like 'Add a module | Scroll tiled windows*'}).Count -eq 1)
 Menu-Type 'Windows clipboard history'
 Check 'Module selection explains its behavior' ((Menu-Name) -match 'clipboard history.*\|.*clipboard history') (Menu-Name)
 Menu-Key 0x1b
 Menu-Go 'Open-window drawer';Menu-Go 'Remove from the bar'
 Check 'Drawer can be removed while retaining running-app buttons' (((Read-Profile $folder).settings.bar.left -join ',') -eq 'tasks') ((Read-Profile $folder).settings.bar.left -join ',')
 Menu-Key 0x1b;Menu-Go 'Right section';Menu-Go 'Add a module to this section';Menu-Go 'Open-window drawer'
 Check 'Drawer can be added independently to another section' ((Read-Profile $folder).settings.bar.right[-1] -eq 'app-drawer')
 Menu-Key 0x1b;Menu-Go 'Center section';Menu-Go 'Add a module to this section';Menu-Go 'Open-window drawer'
 Check 'Adding an existing module moves it instead of duplicating it' ((Read-Profile $folder).settings.bar.center[-1] -eq 'app-drawer' -and 'app-drawer' -notin (Read-Profile $folder).settings.bar.right)
 Menu-Go 'Open-window drawer';Menu-Go 'Move up'
 Check 'Move up changes the saved module order' ((Read-Profile $folder).settings.bar.center[0] -eq 'app-drawer')
 # An action keeps the typed search (only opening a page clears it); erase the 7 letters of 'Move up'.
 for($k=0;$k -lt 7;$k++){Menu-Key 0x08}
 Menu-Go 'Move down'
 Check 'Move down changes the saved module order' ((Read-Profile $folder).settings.bar.center[-1] -eq 'app-drawer')
 Menu-Key 0x1b;Menu-Go 'Add a module to this section';Menu-Go 'Divider';Menu-Go 'Add a module to this section';Menu-Go 'Divider'
 Check 'Dividers can be repeated' (@((Read-Profile $folder).settings.bar.center|Where-Object {$_ -eq 'divider'}).Count -eq 2)
 Menu-Key 0x1b;Menu-Go 'Left section';Menu-Go 'Running apps';Menu-Go 'Remove from the bar'
 Check 'Running-app buttons can be removed while retaining the drawer' ((Read-Profile $folder).settings.bar.left.Count -eq 0 -and 'app-drawer' -in (Read-Profile $folder).settings.bar.center)
 Check 'Migration marker prevents duplicate drawers after restart' ((Read-Profile $folder).settings.bar.split_task_modules)
 Stop-App $app;$app=$null;$app=Start-App $folder;Click $controllerWindow 14;Click ([RecoverTest]::BarSettings()) 218;Pump 500;Menu-Go 'Left section'
 Check 'Bar items reload saved sections after restart' ((Menu-Name) -like 'Left section | This section is empty*') (Menu-Name)
} catch {$results.Add(@{name='Visual bar harness';passed=$false;detail=$_.Exception.ToString()})}
finally {
 if($null -ne $app -and -not $app.HasExited){Stop-App $app}
 if($null -ne $originalTaskbar){[RecoverTest]::RestoreTaskbar($originalTaskbar)}
 $results|ConvertTo-Json -Depth 6|Set-Content -Encoding UTF8 (Join-Path $root 'bar-editor-result.json')
}

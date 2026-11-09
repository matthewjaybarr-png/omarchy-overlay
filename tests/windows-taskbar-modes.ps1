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
 [DllImport("user32.dll",CharSet=CharSet.Unicode,EntryPoint="FindWindowExW")] static extern IntPtr TbFindEx(IntPtr p,IntPtr a,string c,string t);
 [DllImport("user32.dll",EntryPoint="GetWindowThreadProcessId")] static extern uint TbPid(IntPtr h,out uint pid);
 // Explorer's own taskbar: Zebar makes a hidden Shell_TrayWnd that FindWindow can return first.
 public static IntPtr ExplorerTaskbar(){IntPtr first=IntPtr.Zero,h=IntPtr.Zero;while((h=TbFindEx(IntPtr.Zero,h,"Shell_TrayWnd",null))!=IntPtr.Zero){uint p;TbPid(h,out p);try{if(System.Diagnostics.Process.GetProcessById((int)p).ProcessName.Equals("explorer",System.StringComparison.OrdinalIgnoreCase))return h;}catch{}if(first==IntPtr.Zero)first=h;}return first;}
 public static IntPtr TaskbarWindow(){return ExplorerTaskbar();}
 [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h,int show);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern IntPtr GetProp(IntPtr h,string name);

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
function Seed-Paused([string]$folder){$path=Join-Path $folder 'overlay-profile.json';if(-not (Test-Path $path)){return};$p=[IO.File]::ReadAllText($path)|ConvertFrom-Json;if($null -eq $p.settings){$p|Add-Member settings ([pscustomobject]@{})};if($null -ne $p.settings.PSObject.Properties['start_tiling']){return};$p.settings|Add-Member start_tiling $false;[IO.File]::WriteAllText($path,($p|ConvertTo-Json -Depth 20),[Text.UTF8Encoding]::new($false))}
# The default bar has a system tray; hidden mode waits up to 4 s for its warm-up before hiding.
function Wait-Hidden{$until=[DateTime]::UtcNow.AddSeconds(8);while([RecoverTest]::IsWindowVisible($bar) -and [DateTime]::UtcNow -lt $until){Pump 200}}
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

function Choose($h,[int]$id,[int]$i){[void][RecoverTest]::SendMessage([RecoverTest]::GetDlgItem($h,$id),0x14e,[IntPtr]$i,[IntPtr]::Zero)}
function Mode([int]$mode){[void][RecoverTest]::PostMessage($controllerWindow,0x800a,[IntPtr]$mode,[IntPtr]::Zero);Pump 1000}
$journal=Join-Path $local 'taskbar-visibility-restore.json'
$bar=[RecoverTest]::TaskbarWindow();$originalVisible=[RecoverTest]::IsWindowVisible($bar);$originalTaskbar=[RecoverTest]::Taskbar();$cursor=[Windows.Forms.Cursor]::Position
try {
 [void][RecoverTest]::SetProcessDpiAwarenessContext([IntPtr](-4))
 Check 'No controller to disturb' ([RecoverTest]::MainWindow() -eq [IntPtr]::Zero)
 Check 'No pending visibility recovery to disturb' (-not (Test-Path $journal))
 $owner=0;[void][RecoverTest]::GetWindowThreadProcessId($bar,[ref]$owner)
 $folder=Join-Path $root ('taskbar-modes-'+[Guid]::NewGuid().ToString('N'));New-Item -ItemType Directory $folder|Out-Null;Copy-Item "$root\OmarchyOverlay.exe" "$folder\OmarchyOverlay.exe"
 $profile=@{format='overlay-profile';version=1;id='taskbar-modes-test';settings=@{taskbar_mode='hidden';show_top_bar=$true}}
 [IO.File]::WriteAllText("$folder\overlay-profile.json",($profile|ConvertTo-Json -Depth 8),[Text.UTF8Encoding]::new($false))
 $app=Start-App $folder
 Wait-Hidden
 Check 'Keep hidden hides the native taskbar while Explorer runs' (-not [RecoverTest]::IsWindowVisible($bar) -and $null -ne (Get-Process -Id $owner -ErrorAction SilentlyContinue)) "bar $bar visible $([RecoverTest]::IsWindowVisible($bar))"
 Check 'Keep hidden creates a local recovery record' (Test-Path $journal)
 Check 'Taskbar visibility change has an ownership marker' ([RecoverTest]::GetProp($bar,'OmarchyOverlay.TaskbarVisibilityOwner.v1') -ne [IntPtr]::Zero)
 [Windows.Forms.Cursor]::Position=[Drawing.Point]::new(600,([Windows.Forms.Screen]::PrimaryScreen.Bounds.Bottom-1))
 $revealed=$false;for($i=0;$i -lt 50;$i++){Pump 100;if([RecoverTest]::IsWindowVisible($bar)){$revealed=$true}}
 Check 'Keep hidden resists five seconds of edge hover' (-not $revealed)
 Mode 60
 Check 'Visible mode shows the Windows taskbar' ([RecoverTest]::IsWindowVisible($bar) -and [RecoverTest]::Taskbar() -eq 0)
 Check 'Visible mode clears completed visibility recovery' (-not (Test-Path $journal))
 Click $controllerWindow 7
 Check 'Taskbar button can return to the preferred Keep hidden mode' (-not [RecoverTest]::IsWindowVisible($bar) -and (Read-Profile $folder).settings.taskbar_mode -eq 'hidden')
 Mode 61
 Check 'Auto-hide remains a separate native mode' ([RecoverTest]::IsWindowVisible($bar) -and [RecoverTest]::Taskbar() -eq 1 -and (Read-Profile $folder).settings.taskbar_mode -eq 'auto-hide')
 $reloaded=Read-Profile $folder;$reloaded.settings.taskbar_mode='hidden'
 [IO.File]::WriteAllText("$folder\overlay-profile.json",($reloaded|ConvertTo-Json -Depth 12),[Text.UTF8Encoding]::new($false))
 Click $controllerWindow 3;Click ([RecoverTest]::Settings()) 68;Click ([RecoverTest]::Profiles()) 83;Pump 800
 Check 'Reloading a Keep hidden profile applies its taskbar choice' (-not [RecoverTest]::IsWindowVisible($bar))
 Click $controllerWindow 7
 Check 'Taskbar button shows a reloaded hidden taskbar' ([RecoverTest]::IsWindowVisible($bar))
 Click $controllerWindow 7
 Check 'Taskbar button remembers Keep hidden after profile reload' (-not [RecoverTest]::IsWindowVisible($bar) -and (Read-Profile $folder).settings.taskbar_mode -eq 'hidden')
 Mode 61

 Click $controllerWindow 14;$settings=[RecoverTest]::BarSettings();Choose $settings 216 3;Click $settings 211
 Check 'Bar settings can save Keep hidden without a command shortcut' ((Read-Profile $folder).settings.taskbar_mode -eq 'hidden' -and -not [RecoverTest]::IsWindowVisible($bar))
 [void][RecoverTest]::ShowWindowAsync($bar,4)
 $until=[DateTime]::UtcNow.AddSeconds(3);do{Pump 100}while([RecoverTest]::IsWindowVisible($bar) -and [DateTime]::UtcNow -lt $until)
 Check 'Shell re-show events are handled without stopping Explorer' (-not [RecoverTest]::IsWindowVisible($bar))
 Click $controllerWindow 6
 Check 'Hiding Omarchy Overlay bar restores access to the Windows taskbar' ([RecoverTest]::IsWindowVisible($bar))
 Click $controllerWindow 6
 Check 'Showing Omarchy Overlay bar reapplies its Keep hidden choice' (-not [RecoverTest]::IsWindowVisible($bar))
 Stop-App $app;$app=$null
 Check 'Normal exit restores original taskbar visibility and preference' ([RecoverTest]::IsWindowVisible($bar) -eq $originalVisible -and [RecoverTest]::Taskbar() -eq $originalTaskbar)
 Check 'Normal exit removes only its own visibility marker' ([RecoverTest]::GetProp($bar,'OmarchyOverlay.TaskbarVisibilityOwner.v1') -eq [IntPtr]::Zero)
 $app=Start-App $folder;Wait-Hidden;Check 'Saved Keep hidden mode survives restart' (-not [RecoverTest]::IsWindowVisible($bar))
 $app.Kill();[void]$app.WaitForExit(5000);$app=$null
 $until=[DateTime]::UtcNow.AddSeconds(12);do{Pump 200}while((Test-Path $journal) -and [DateTime]::UtcNow -lt $until)
 Check 'Recovery guard restores the taskbar after controller termination' ([RecoverTest]::IsWindowVisible($bar) -eq $originalVisible -and [RecoverTest]::Taskbar() -eq $originalTaskbar)
 Check 'Recovery guard clears a successful visibility journal' (-not (Test-Path $journal))
} catch {$results.Add(@{name='Taskbar modes harness';passed=$false;detail=$_.Exception.ToString()})}
finally {
 if($null -ne $app -and -not $app.HasExited){Stop-App $app}
 [Windows.Forms.Cursor]::Position=$cursor
 [RecoverTest]::RestoreTaskbar($originalTaskbar)
 if($originalVisible){[void][RecoverTest]::ShowWindowAsync($bar,4)}
 $results|ConvertTo-Json -Depth 6|Set-Content -Encoding UTF8 (Join-Path $root 'taskbar-modes-result.json')
}

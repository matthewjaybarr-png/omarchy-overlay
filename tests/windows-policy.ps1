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
 public static void Top(IntPtr h){SetWindowPos(h,(IntPtr)(-1),0,0,0,0,0x53);}
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
function Capture($hwnd,[string]$file){[RecoverTest]::Top($hwnd);Pump 500;$r=[RecoverTest+RECT]::new();[void][RecoverTest]::GetWindowRect($hwnd,[ref]$r);$bitmap=[Drawing.Bitmap]::new(($r.r-$r.l),($r.b-$r.t));$graphics=[Drawing.Graphics]::FromImage($bitmap);try{$graphics.CopyFromScreen($r.l,$r.t,0,0,$bitmap.Size);$bitmap.Save((Join-Path $root $file),[Drawing.Imaging.ImageFormat]::Png)}finally{$graphics.Dispose();$bitmap.Dispose()}}

function Choose($h,[int]$id,[int]$i){[void][RecoverTest]::SendMessage([RecoverTest]::GetDlgItem($h,$id),0x14e,[IntPtr]$i,[IntPtr]::Zero)}
function Select-App($h,[string]$exe){$list=[RecoverTest]::GetDlgItem($h,1001);$n=[RecoverTest]::SendMessage($list,0x18b,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32();for($i=0;$i -lt $n;$i++){if([RecoverTest]::Item($list,$i) -match [Regex]::Escape($exe)){[void][RecoverTest]::SendMessage($list,0x186,[IntPtr]$i,[IntPtr]::Zero);Click $h (1001+(1 -shl 16));return}};throw "App not found: $exe"}
$fixtures=@();$windows=@()
try {
 [void][RecoverTest]::SetProcessDpiAwarenessContext([IntPtr](-4))
 Check 'No controller to disturb' ([RecoverTest]::MainWindow() -eq [IntPtr]::Zero)
 Check 'No Outlook process to disturb' (@(Get-Process olk,OUTLOOK -ErrorAction SilentlyContinue).Count -eq 0)
 $originalTaskbar=[RecoverTest]::Taskbar()
 $folder=Join-Path $root ('policy-'+[Guid]::NewGuid().ToString('N'));New-Item -ItemType Directory $folder|Out-Null;Copy-Item "$root\OmarchyOverlay.exe" "$folder\OmarchyOverlay.exe"
 $fixture='using System;using System.Windows.Forms;public static class Fixture { [STAThread] public static void Main(string[] args){var f=new Form();f.StartPosition=FormStartPosition.Manual;f.Text=args[0];f.SetBounds(120,150,620,380);Application.Run(f);}}'
 Add-Type -ReferencedAssemblies System.Windows.Forms -TypeDefinition $fixture -OutputAssembly "$folder\olk.exe" -OutputType WindowsApplication
 Copy-Item "$folder\olk.exe" "$folder\OUTLOOK.exe"
 $fixtures+=Start-Process "$folder\olk.exe" -ArgumentList '"Omarchy Overlay new Outlook fixture"' -PassThru
 $fixtures+=Start-Process "$folder\OUTLOOK.exe" -ArgumentList '"Omarchy Overlay classic Outlook fixture"' -PassThru
 Pump 1500
 foreach($p in $fixtures){$p.Refresh();$windows+=,$p.MainWindowHandle}
 Check 'Two isolated Outlook-name fixtures opened' ($windows.Count -eq 2 -and $windows[0] -ne [IntPtr]::Zero -and $windows[1] -ne [IntPtr]::Zero)
 $original=@($windows|ForEach-Object{Bounds $_})
 $excluded=@(Get-Process|Where-Object {$_.SessionId -eq [Diagnostics.Process]::GetCurrentProcess().SessionId -and $_.ProcessName -notin @('olk','OUTLOOK')}|ForEach-Object {$_.ProcessName+'.exe'}|Select-Object -Unique)
 $profile=@{format='overlay-profile';version=1;id='policy-fixture';settings=@{default_mode='float';default_allow_opacity=$false;excluded_processes=$excluded;rules=@(@{executable='olk.exe';mode='float';allow_opacity=$true},@{executable='OUTLOOK.EXE';mode='tile';allow_opacity=$true})}}
 [IO.File]::WriteAllText("$folder\overlay-profile.json",($profile|ConvertTo-Json -Depth 8),[Text.UTF8Encoding]::new($false))
 $app=Start-App $folder;Click $controllerWindow 1;Pump 1200
 Check 'New Outlook keeps its explicit Float rule' ((Bounds $windows[0]) -eq $original[0]) ("before=$($original[0]); after=$(Bounds $windows[0])")
 Check 'Classic Outlook follows its separate Tile rule' ((Bounds $windows[1]) -ne $original[1])
 Click $controllerWindow 3;Click ([RecoverTest]::Settings()) 76;$panel=[RecoverTest]::Rules()
 Check 'Apps and defaults panel opens' ([RecoverTest]::IsWindowVisible($panel))
 Select-App $panel 'olk.exe'
 Check 'Chooser identifies actual new Outlook executable' ([RecoverTest]::ControlText($panel,1002) -match 'Outlook \(new\).*olk.exe')
 Choose $panel 1005 0;Input-Text $panel 1007 '90';Click $panel 1010
 Check 'Saving selected app changes olk.exe rather than classic Outlook' ((Read-Profile $folder).settings.rules[0].mode -eq 'tile' -and (Read-Profile $folder).settings.rules[1].mode -eq 'tile')
 [void][RecoverTest]::PostMessage($panel,0x10,[IntPtr]::Zero,[IntPtr]::Zero);Pump 1000
 Check 'Saving Tile adopts the selected app immediately' ((Bounds $windows[0]) -ne $original[0])
 [void][RecoverTest]::SetForegroundWindow($windows[0]);Pump 1000
 Check 'Per-app active opacity applies its chosen native alpha' ([Math]::Abs([RecoverTest]::Alpha($windows[0])-230)-le 1)
 [void][RecoverTest]::PostMessage($controllerWindow,0x312,[IntPtr]103,[IntPtr]::Zero);Pump 800
 Check 'Manual floating returns selected app to its original placement' ((Bounds $windows[0]) -eq $original[0])
 [void][RecoverTest]::SetForegroundWindow($windows[1]);[void][RecoverTest]::PostMessage($controllerWindow,0x312,[IntPtr]103,[IntPtr]::Zero);Pump 800
 $otherFloating=Bounds $windows[1]
 Click ([RecoverTest]::Settings()) 76;$panel=[RecoverTest]::Rules();Select-App $panel 'olk.exe';Click $panel 1010;Pump 800
 Check 'Saving Tile clears the matching app manual floating override' ((Bounds $windows[0]) -ne $original[0])
 Check 'Saving one app preserves another app manual floating choice' ((Bounds $windows[1]) -eq $otherFloating)
 Choose $panel 1000 1;Click $panel (1000+(1 -shl 16));Choose $panel 1005 2;Choose $panel 1006 1;Input-Text $panel 1007 '100';Input-Text $panel 1008 '98';Click $panel 1010
 $saved=Read-Profile $folder
 Check 'Global defaults save without replacing explicit app rules' ($saved.settings.default_mode -eq 'ignore' -and $saved.settings.rules[0].mode -eq 'tile' -and $saved.settings.rules[1].mode -eq 'tile')
 Capture $panel 'apps-and-defaults.png'
 # 0.20: in global mode the running-apps list stays enabled (readable); choosing an app there switches back to it.
 Check 'Global mode keeps the running-apps list enabled and readable' ([RecoverTest]::IsWindowEnabled([RecoverTest]::GetDlgItem($panel,1001)))
 # 0.21: no app stays highlighted in global mode, so the panel does not look like an app's rule.
 Check 'Global mode leaves no app highlighted in the list' ([RecoverTest]::SendMessage([RecoverTest]::GetDlgItem($panel,1001),0x188,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32() -eq -1)
 [void][RecoverTest]::SendMessage([RecoverTest]::GetDlgItem($panel,1001),0x186,[IntPtr]0,[IntPtr]::Zero)
 Click $panel (1001+(1 -shl 16))
 Check 'Choosing an app in global mode switches back to Selected running app' ([RecoverTest]::SendMessage([RecoverTest]::GetDlgItem($panel,1000),0x147,[IntPtr]::Zero,[IntPtr]::Zero).ToInt32() -eq 0)
 Stop-App $app;$app=$null
 Check 'Exit restores the original native opacity' (-not [RecoverTest]::Layered($windows[0]))
} catch {$results.Add(@{name='Policy harness';passed=$false;detail=$_.Exception.ToString()})}
finally {
 if($null -ne $app -and -not $app.HasExited){Stop-App $app}
 foreach($p in $fixtures){if(-not $p.HasExited){[void]$p.CloseMainWindow();[void]$p.WaitForExit(5000)}}
 if($null -ne $originalTaskbar){[RecoverTest]::RestoreTaskbar($originalTaskbar)}
 $results|ConvertTo-Json -Depth 6|Set-Content -Encoding UTF8 (Join-Path $root 'policy-result.json')
}

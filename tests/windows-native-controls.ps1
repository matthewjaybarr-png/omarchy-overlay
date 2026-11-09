# Verify injected native system chords without reading clipboard data or saving a capture.
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $MyInvocation.MyCommand.Path
$results=[Collections.Generic.List[object]]::new();$app=$null;$fixture=$null
Add-Type -AssemblyName System.Windows.Forms
Add-Type @'
using System;using System.Text;using System.Runtime.InteropServices;
public static class ControlsTest {
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern IntPtr FindWindow(string cls,string title);
 public static IntPtr MainWindow(){return FindWindow("OmarchyOverlay.Main",null);}
 public static IntPtr Overlay() { return FindWindow("OmarchyOverlay.Overlay", null); }
 // 0.16 opens the Omarchy Overlay menu at startup; the 0.15 dashboard stays hidden until asked for.
 public static bool Ready(IntPtr main) { return main != IntPtr.Zero && (IsWindowVisible(main) || IsWindowVisible(Overlay())); }
 [DllImport("user32.dll", EntryPoint="PostMessageW")] static extern bool PostToApp(IntPtr hwnd, uint message, IntPtr w, IntPtr l);
 public static void CloseMenu() { IntPtr menu = Overlay(); if (IsWindowVisible(menu)) PostToApp(menu, 0x100, (IntPtr)0x1b, IntPtr.Zero); }
 [DllImport("user32.dll", EntryPoint="ShowWindow")] static extern bool ShowAppWindow(IntPtr hwnd, int command);
 public static bool ShowDashboard(IntPtr main) { ShowAppWindow(main, 1); return IsWindowVisible(main); }
 [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
 [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
 [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
 [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h,uint m,IntPtr w,IntPtr l);
 [DllImport("user32.dll")] public static extern void keybd_event(byte key,byte scan,uint flags,UIntPtr extra);
 [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h,out uint pid);
 [DllImport("user32.dll",CharSet=CharSet.Unicode)] static extern int GetClassName(IntPtr h,StringBuilder t,int n);
 public static string Foreground(){var h=GetForegroundWindow();if(h==IntPtr.Zero)return "none";uint pid;GetWindowThreadProcessId(h,out pid);var t=new StringBuilder(256);GetClassName(h,t,256);return System.Diagnostics.Process.GetProcessById((int)pid).ProcessName+"/"+t;}
 [StructLayout(LayoutKind.Sequential)] struct Rect{public int l,t,r,b;}
 [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr h,out Rect r);
 [DllImport("user32.dll")] static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int w,int height,uint flags);
 [DllImport("user32.dll")] static extern bool SetCursorPos(int x,int y);
 [DllImport("user32.dll")] static extern void mouse_event(uint flags,uint x,uint y,uint data,UIntPtr extra);
 public static void Focus(IntPtr h){Rect r;GetWindowRect(h,out r);SetWindowPos(h,(IntPtr)(-1),0,0,0,0,0x13);SetCursorPos(r.l+100,r.t+20);mouse_event(2,0,0,0,UIntPtr.Zero);mouse_event(4,0,0,0,UIntPtr.Zero);SetWindowPos(h,(IntPtr)(-2),0,0,0,0,0x13);}
 delegate bool EnumProc(IntPtr h,IntPtr d);
 [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc callback,IntPtr d);
 [DllImport("dwmapi.dll")] static extern int DwmGetWindowAttribute(IntPtr h,uint attr,out uint value,uint size);
 public static string VisibleNative(string expected){string result="";EnumWindows((h,d)=>{uint pid;GetWindowThreadProcessId(h,out pid);if(pid==0||!IsWindowVisible(h))return true;string name;try{name=System.Diagnostics.Process.GetProcessById((int)pid).ProcessName;}catch{return true;}if(!System.Text.RegularExpressions.Regex.IsMatch(name,expected))return true;uint cloak=0;DwmGetWindowAttribute(h,14,out cloak,4);Rect r;GetWindowRect(h,out r);if(cloak==0&&r.r-r.l>100&&r.b-r.t>100){var t=new StringBuilder(256);GetClassName(h,t,256);result+=name+"/"+t+";";}return true;},IntPtr.Zero);return result;}
 delegate IntPtr HookProc(int code,IntPtr w,IntPtr l);
 [StructLayout(LayoutKind.Sequential)] struct KeyData{public uint key,scan,flags,time;public UIntPtr extra;}
 [DllImport("user32.dll")] static extern IntPtr SetWindowsHookEx(int kind,HookProc callback,IntPtr module,uint thread);
 [DllImport("user32.dll")] static extern bool UnhookWindowsHookEx(IntPtr hook);
 [DllImport("user32.dll")] static extern IntPtr CallNextHookEx(IntPtr hook,int code,IntPtr w,IntPtr l);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode)] static extern IntPtr GetModuleHandle(string module);
 static IntPtr hook;static HookProc callback=Observe;static System.Collections.Generic.List<uint> keys=new System.Collections.Generic.List<uint>();
 static IntPtr Observe(int code,IntPtr w,IntPtr l){if(code>=0&&(w.ToInt32()==0x100||w.ToInt32()==0x104)){var data=(KeyData)Marshal.PtrToStructure(l,typeof(KeyData));if((data.flags&16)!=0)keys.Add(data.key==160||data.key==161?16:data.key);}return CallNextHookEx(hook,code,w,l);}
 public static bool StartHook(){hook=SetWindowsHookEx(13,callback,GetModuleHandle(null),0);return hook!=IntPtr.Zero;}
 public static void StopHook(){if(hook!=IntPtr.Zero)UnhookWindowsHookEx(hook);hook=IntPtr.Zero;}
 public static void ClearKeys(){keys.Clear();}
 public static string Keys(){return string.Join(",",keys);}
 public static void Key(byte key,bool up){keybd_event(key,0,up?2u:0u,UIntPtr.Zero);}
}
'@
function Pump([int]$ms){$until=[DateTime]::UtcNow.AddMilliseconds($ms);while([DateTime]::UtcNow -lt $until){[Windows.Forms.Application]::DoEvents();Start-Sleep -Milliseconds 20}}
function Check($name,[bool]$ok,$detail=''){$results.Add(@{name=$name;passed=$ok;detail=$detail});if(-not $ok){throw "$name ($detail)"}}
try {
 Check 'No controller to disturb' ([ControlsTest]::MainWindow() -eq [IntPtr]::Zero)
 Check 'Read-only input observer is installed for the fixture' ([ControlsTest]::StartHook())
 $fixture=[Windows.Forms.Form]::new();$fixture.Text='Omarchy Overlay native control fixture';$fixture.SetBounds(180,160,600,400);$fixture.Show();Pump 300
 $app=Start-Process "$root\OmarchyOverlay.exe" -PassThru
 $until=[DateTime]::UtcNow.AddSeconds(20)
 do{Pump 100;$main=[ControlsTest]::MainWindow()}while(-not [ControlsTest]::Ready($main) -and [DateTime]::UtcNow -lt $until)
 Check 'Controller is ready for native control actions' ([ControlsTest]::Ready($main));[ControlsTest]::CloseMenu()
 foreach($action in @(56,57,200,201)) {
  # Native flyouts may finish dismissing after Escape. Wait for our fixture to
  # regain focus before testing dispatch; never resend the action itself.
  $focusDeadline=[DateTime]::UtcNow.AddSeconds(3)
  do {
   $fixture.Activate();[void][ControlsTest]::SetForegroundWindow($fixture.Handle)
   [ControlsTest]::Focus($fixture.Handle);Pump 400
  } while([ControlsTest]::GetForegroundWindow() -ne $fixture.Handle -and [DateTime]::UtcNow -lt $focusDeadline)
  Check "Fixture owns focus before native action $action" ([ControlsTest]::GetForegroundWindow() -eq $fixture.Handle) ([ControlsTest]::Foreground())
  $nativeProcess=if($action -eq 56){'^TextInputHost$'}elseif($action -eq 57){'^(ScreenClippingHost|SnippingTool)$'}else{'^(ShellExperienceHost|ShellHost|explorer)$'}
  [ControlsTest]::Key(0x11,$false);Pump 100;[ControlsTest]::ClearKeys()
  if($action -lt 200){[void][ControlsTest]::PostMessage($main,0x312,[IntPtr](100+$action),[IntPtr]::Zero)}else{[void][ControlsTest]::PostMessage($main,0x800a,[IntPtr]$action,[IntPtr]::Zero)};Pump 1000
  Check "Native action $action waits for held modifiers" ([ControlsTest]::Keys() -eq "")
  [ControlsTest]::Key(0x11,$true)
  $until=[DateTime]::UtcNow.AddSeconds(8)
  $expectedKeys=@{56='91,86';57='91,16,83';200='91,78';201='91,65';203='91,88'}[$action]
  do{Pump 100}while([ControlsTest]::Keys() -ne $expectedKeys -and [DateTime]::UtcNow -lt $until)
  $nativeVisible=[ControlsTest]::VisibleNative($nativeProcess)
  $label="Native action $action dispatches exact Windows chord after modifier release"
  Check $label ([ControlsTest]::Keys() -eq $expectedKeys) ("keys=$([ControlsTest]::Keys()); visible native host=$nativeVisible")
  [ControlsTest]::Key(0x1b,$false);[ControlsTest]::Key(0x1b,$true);Pump 1000
 }
 # 0.16: the POWER bar module opens the Omarchy Overlay System menu instead of sending Win+X.
 [ControlsTest]::ClearKeys();[void][ControlsTest]::PostMessage($main,0x800a,[IntPtr]203,[IntPtr]::Zero)
 $until=[DateTime]::UtcNow.AddSeconds(5)
 do{Pump 100}while(-not [ControlsTest]::IsWindowVisible([ControlsTest]::Overlay()) -and [DateTime]::UtcNow -lt $until)
 Check 'POWER module opens the Omarchy Overlay System menu without a Windows chord' ([ControlsTest]::IsWindowVisible([ControlsTest]::Overlay()) -and [ControlsTest]::Keys() -eq '') ("keys=$([ControlsTest]::Keys())")
 [ControlsTest]::CloseMenu();Pump 300
} catch {$results.Add(@{name='Native controls harness';passed=$false;detail=$_.Exception.ToString()})}
finally {
 [ControlsTest]::StopHook()
 [ControlsTest]::Key(0x11,$true);[ControlsTest]::Key(0x1b,$false);[ControlsTest]::Key(0x1b,$true)
 if($null -ne $app -and -not $app.HasExited){[void][ControlsTest]::PostMessage([ControlsTest]::MainWindow(),0x312,[IntPtr]105,[IntPtr]::Zero);Pump 1800}
 if($null -ne $fixture){$fixture.Close();$fixture.Dispose()}
 $results|ConvertTo-Json -Depth 5|Set-Content -Encoding UTF8 (Join-Path $root 'native-controls-result.json')
}

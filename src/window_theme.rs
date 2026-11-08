//! Reversible supported DWM frame colors and materials; app client rendering stays native.
use super::*;
const PROPERTY: &str = "OmarchyOverlay.WindowThemeOwner.v1";
const FILE: &str = "window-theme-restore.json";
const ATTRS: [u32; 5] = [20, 34, 35, 36, 38];
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Original {
    hwnd: usize,
    pid: u32,
    marker: usize,
    before: Vec<(u32, u32)>,
    acceptable: Vec<Vec<(u32, u32)>>,
}
unsafe fn read(h: HWND, attribute: u32) -> Option<u32> {
    let mut value = 0;
    (DwmGetWindowAttribute(h, attribute, &mut value as *mut _ as _, 4) >= 0).then_some(value)
}
unsafe fn write(h: HWND, attribute: u32, value: u32) -> bool {
    DwmSetWindowAttribute(h, attribute, &value as *const _ as _, 4) >= 0
}
impl Original {
    unsafe fn matches(&self) -> bool {
        let h = self.hwnd as HWND;
        let mut pid = 0;
        GetWindowThreadProcessId(h, &mut pid);
        IsWindow(h) != 0
            && pid == self.pid
            && GetPropW(h, wide(PROPERTY).as_ptr()) as usize == self.marker
    }
    unsafe fn restore(&self) -> bool {
        if !self.matches() {
            return true;
        }
        let h = self.hwnd as HWND;
        if !responsive(h) {
            return false;
        }
        let mut ok = true;
        for &(attribute, before) in &self.before {
            let current = read(h, attribute);
            if current.is_none() {
                // A transient DWM failure must not discard the only undo record.
                ok = false;
                continue;
            }
            let ours = self
                .acceptable
                .iter()
                .any(|values| values.contains(&(attribute, current.unwrap_or(before))));
            if current == Some(before) {
                continue;
            }
            if ours {
                ok &= write(h, attribute, before);
            } else if current.is_some() {
                log("A window frame was changed externally; that attribute was preserved.");
            }
        }
        if ok {
            RemovePropW(h, wide(PROPERTY).as_ptr());
            RedrawWindow(h, null(), null_mut(), RDW_INVALIDATE | RDW_FRAME);
        }
        ok
    }
}
pub(super) struct Engine {
    path: PathBuf,
    originals: HashMap<usize, Original>,
    blocked: bool,
}
impl Engine {
    pub(super) unsafe fn new(directory: &Path) -> Self {
        let path = directory.join(FILE);
        Self {
            blocked: path.exists(),
            path,
            originals: HashMap::new(),
        }
    }
    fn save(&self) -> Result<(), String> {
        if self.blocked {
            return Err("Window-frame recovery must finish before another change.".into());
        }
        if self.originals.is_empty() {
            if self.path.exists() {
                fs::remove_file(&self.path).map_err(|e| e.to_string())?;
            }
            Ok(())
        } else {
            write_json_atomic(&self.path, &self.originals.values().collect::<Vec<_>>())
        }
    }
    unsafe fn apply(&mut self, h: HWND, colors: bool, material: u32) -> Result<(), String> {
        if self.blocked {
            return Err("Window-frame recovery is pending; frame changes were skipped.".into());
        }
        if !responsive(h) {
            return Ok(());
        }
        let key = h as usize;
        if !self.originals.get(&key).is_some_and(|o| o.matches()) {
            self.originals.remove(&key);
            let before = ATTRS
                .iter()
                .filter_map(|&a| read(h, a).map(|v| (a, v)))
                .collect::<Vec<_>>();
            if before.is_empty() {
                return Ok(());
            }
            let mut pid = 0;
            GetWindowThreadProcessId(h, &mut pid);
            let marker = ((GetCurrentProcessId() as usize).wrapping_mul(16777619)
                ^ std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_nanos() as usize)
                | 1;
            if SetPropW(h, wide(PROPERTY).as_ptr(), marker as HANDLE) == 0 {
                return Err("Windows did not allow recoverable frame changes for this app.".into());
            }
            self.originals.insert(
                key,
                Original {
                    hwnd: key,
                    pid,
                    marker,
                    before,
                    acceptable: vec![],
                },
            );
        }
        let original = &self.originals[&key];
        let p = theme::current();
        let desired = original
            .before
            .iter()
            .map(|&(a, before)| {
                (
                    a,
                    match a {
                        20 if colors => u32::from(p.dark),
                        34 if colors => p.accent,
                        35 if colors => p.background,
                        36 if colors => p.text,
                        38 if material != 0 => material,
                        _ => before,
                    },
                )
            })
            .collect::<Vec<_>>();
        let changed = original.acceptable.last() != Some(&desired);
        if changed {
            let original = self.originals.get_mut(&key).unwrap();
            original.acceptable = original
                .acceptable
                .last()
                .cloned()
                .into_iter()
                .chain(std::iter::once(desired.clone()))
                .collect();
            if let Err(error) = self.save() {
                if let Some(o) = self.originals.remove(&key) {
                    o.restore();
                }
                return Err(error);
            }
        }
        for &(a, v) in &desired {
            if read(h, a) != Some(v) && !write(h, a, v) {
                log("An app declined a supported window-frame attribute; its client rendering was left native.");
            }
        }
        Ok(())
    }
    pub(super) unsafe fn poll(
        &mut self,
        windows: &[HWND],
        policy: impl Fn(HWND) -> Option<(bool, u32)>,
    ) {
        let old_count = self.originals.len();
        self.originals.retain(|_, o| o.matches());
        let release = self
            .originals
            .keys()
            .copied()
            .filter(|&key| !windows.contains(&(key as HWND)) || policy(key as HWND).is_none())
            .collect::<Vec<_>>();
        for key in release {
            if self.originals[&key].restore() {
                self.originals.remove(&key);
            }
        }
        if old_count != self.originals.len() {
            if let Err(e) = self.save() {
                log(&e)
            }
        }
        for &h in windows {
            if let Some((colors, material)) = policy(h) {
                if let Err(e) = self.apply(h, colors, material) {
                    log(&e)
                }
            }
        }
    }
    pub(super) unsafe fn restore_all(&mut self) -> Result<(), String> {
        self.originals.retain(|_, o| !o.restore());
        self.save()?;
        if self.originals.is_empty() {
            Ok(())
        } else {
            Err("Some app frames did not restore; recovery records were kept.".into())
        }
    }
}
pub(super) unsafe fn recover(directory: &Path) -> Result<(), String> {
    let path = directory.join(FILE);
    if !path.exists() {
        return Ok(());
    }
    let records: Vec<Original> =
        serde_json::from_str(&profile::read_text(&path)?).map_err(|e| e.to_string())?;
    if records.len() > 4096
        || records.iter().any(|o| {
            o.hwnd == 0
                || o.pid == 0
                || o.marker == 0
                || o.before.len() > ATTRS.len()
                || o.before.iter().any(|(a, _)| !ATTRS.contains(a))
                || o.acceptable.len() > 2
                || o.acceptable
                    .iter()
                    .any(|v| v.len() > ATTRS.len() || v.iter().any(|(a, _)| !ATTRS.contains(a)))
        })
    {
        return Err("Invalid window-frame recovery record; it was preserved.".into());
    }
    let mut e = Engine {
        path,
        originals: records.into_iter().map(|o| (o.hwnd, o)).collect(),
        blocked: false,
    };
    e.restore_all()
}
const BACKDROP_PROPERTY: &str = "OmarchyOverlay.BackdropEnabled.v1";
pub(super) unsafe fn backdrop_active(h: HWND) -> bool {
    !GetPropW(h, wide(BACKDROP_PROPERTY).as_ptr()).is_null()
}
pub(super) unsafe fn own(h: HWND, material: &str) {
    let dark = u32::from(theme::current().dark);
    write(h, 20, dark);
    write(h, 35, theme::current().background);
    write(h, 36, theme::current().text);
    let value = match material {
        "mica" => 2,
        "acrylic" => 3,
        _ => 1,
    };
    let margins = MARGINS {
        cxLeftWidth: if value > 1 { -1 } else { 0 },
        cxRightWidth: if value > 1 { -1 } else { 0 },
        cyTopHeight: if value > 1 { -1 } else { 0 },
        cyBottomHeight: if value > 1 { -1 } else { 0 },
    };
    if write(h, 38, value) && DwmExtendFrameIntoClientArea(h, &margins) >= 0 && value > 1 {
        SetPropW(h, wide(BACKDROP_PROPERTY).as_ptr(), 1usize as HANDLE);
    } else {
        RemovePropW(h, wide(BACKDROP_PROPERTY).as_ptr());
    }
    InvalidateRect(h, null(), 1);
}

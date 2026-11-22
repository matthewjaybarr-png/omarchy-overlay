//! Reversible whole-window opacity. Windows keep their own rendering and input.
use super::*;
const PROPERTY: &str = "OmarchyOverlay.OpacityOwner.v1";

#[derive(Serialize, Deserialize, Clone)]
struct Original {
    hwnd: usize,
    pid: u32,
    marker: usize,
    layered: bool,
    color: u32,
    alpha: u8,
    flags: u32,
}
pub(super) struct Engine {
    path: PathBuf,
    originals: HashMap<usize, Original>,
    manual: HashMap<usize, u32>,
    disabled: bool,
}
impl Original {
    unsafe fn matches(&self) -> bool {
        let mut pid = 0;
        GetWindowThreadProcessId(self.hwnd as HWND, &mut pid);
        IsWindow(self.hwnd as HWND) != 0
            && pid == self.pid
            && GetPropW(self.hwnd as HWND, wide(PROPERTY).as_ptr()) as usize == self.marker
    }
    unsafe fn restore(&self) -> bool {
        if !self.matches() {
            return true;
        }
        if !responsive(self.hwnd as HWND) {
            return false;
        }
        let hwnd = self.hwnd as HWND;
        let ok = if self.layered {
            SetLayeredWindowAttributes(hwnd, self.color, self.alpha, self.flags) != 0
        } else {
            let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetLastError(0);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style & !(WS_EX_LAYERED as isize));
            GetLastError() == 0
        };
        if ok {
            RemovePropW(hwnd, wide(PROPERTY).as_ptr());
            RedrawWindow(
                hwnd,
                null(),
                null_mut(),
                RDW_INVALIDATE | RDW_FRAME | RDW_ALLCHILDREN,
            );
        }
        ok
    }
}
impl Engine {
    pub(super) unsafe fn new(directory: &Path) -> Self {
        let path = directory.join("opacity-restore.json");
        let mut engine = Self {
            path,
            originals: HashMap::new(),
            manual: HashMap::new(),
            disabled: false,
        };
        if engine.path.exists() {
            match fs::read_to_string(&engine.path)
                .ok()
                .and_then(|s| serde_json::from_str::<Vec<Original>>(&s).ok())
            {
                Some(records) => {
                    for original in records {
                        if !original.restore() {
                            engine.originals.insert(original.hwnd, original);
                        }
                    }
                    if engine.save().is_err() {
                        engine.disabled = true;
                    }
                }
                None => {
                    engine.disabled = true;
                    log("Opacity recovery file could not be read; opacity controls disabled.");
                }
            }
        }
        engine
    }
    fn save(&self) -> Result<(), String> {
        if self.originals.is_empty() {
            if self.path.exists() {
                fs::remove_file(&self.path).map_err(|e| e.to_string())?;
            }
            return Ok(());
        }
        let records: Vec<_> = self.originals.values().collect();
        write_json_atomic(&self.path, &records)
    }
    unsafe fn capture(&mut self, hwnd: HWND) -> Result<(), String> {
        if self.disabled {
            return Err("Opacity recovery needs attention. See omarchy-overlay.log.".into());
        }
        if !responsive(hwnd) {
            return Err("This window is not responding; its opacity has not been changed.".into());
        }
        let key = hwnd as usize;
        if self.originals.get(&key).is_some_and(|o| o.matches()) {
            return Ok(());
        }
        self.originals.remove(&key);
        self.manual.remove(&key);
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        let layered = style & WS_EX_LAYERED != 0;
        let (mut color, mut alpha, mut flags) = (0, 255, LWA_ALPHA);
        if layered && GetLayeredWindowAttributes(hwnd, &mut color, &mut alpha, &mut flags) == 0 {
            return Err("This app manages its own transparency; Omarchy Overlay leaves it alone.".into());
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let marker = GetCurrentProcessId() as usize;
        if SetPropW(hwnd, wide(PROPERTY).as_ptr(), marker as HANDLE) == 0 {
            return Err("Windows did not allow Omarchy Overlay to change this window.".into());
        }
        self.originals.insert(
            key,
            Original {
                hwnd: key,
                pid,
                marker,
                layered,
                color,
                alpha,
                flags,
            },
        );
        if let Err(error) = self.save() {
            self.originals.remove(&key);
            RemovePropW(hwnd, wide(PROPERTY).as_ptr());
            return Err(format!("Could not save the original opacity: {error}"));
        }
        Ok(())
    }
    unsafe fn apply(&mut self, hwnd: HWND, percent: u8) -> Result<(), String> {
        let previously_tracked = self
            .originals
            .get(&(hwnd as usize))
            .is_some_and(|o| o.matches());
        self.capture(hwnd)?;
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if style & WS_EX_LAYERED as isize == 0 {
            SetLastError(0);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED as isize);
            if GetLastError() != 0 {
                if !previously_tracked {
                    let _ = self.clear(hwnd);
                }
                return Err("Windows refused this window's opacity change.".into());
            }
        }
        let original = &self.originals[&(hwnd as usize)];
        let alpha = ((u16::from(percent.clamp(50, 100)) * 255 + 50) / 100) as u8;
        if SetLayeredWindowAttributes(hwnd, original.color, alpha, original.flags | LWA_ALPHA) == 0
        {
            if !previously_tracked {
                let _ = self.clear(hwnd);
            }
            return Err("This window does not support the requested opacity.".into());
        }
        Ok(())
    }
    pub(super) unsafe fn manual(&mut self, hwnd: HWND, percent: u8) -> Result<(), String> {
        if percent >= 100 {
            self.clear(hwnd)?;
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, &mut pid);
            self.manual.insert(hwnd as usize, pid);
            return Ok(());
        }
        self.apply(hwnd, percent)?;
        self.manual
            .insert(hwnd as usize, self.originals[&(hwnd as usize)].pid);
        Ok(())
    }
    pub(super) unsafe fn percent(&self, hwnd: HWND) -> u8 {
        let mut alpha = 255;
        if GetLayeredWindowAttributes(hwnd, null_mut(), &mut alpha, null_mut()) != 0 {
            ((u16::from(alpha) * 100 + 127) / 255) as u8
        } else {
            100
        }
    }
    pub(super) unsafe fn clear(&mut self, hwnd: HWND) -> Result<(), String> {
        self.manual.remove(&(hwnd as usize));
        if let Some(original) = self.originals.get(&(hwnd as usize)) {
            if !original.restore() {
                return Err("Windows could not restore this window's original opacity.".into());
            }
        }
        self.originals.remove(&(hwnd as usize));
        self.save()
    }
    pub(super) unsafe fn restore_all(&mut self) -> Result<(), String> {
        if self.disabled {
            return Err(
                "Opacity recovery data could not be read; the file has been preserved.".into(),
            );
        }
        self.manual.clear();
        self.originals.retain(|_, original| !original.restore());
        if !self.originals.is_empty() {
            // A busy window can refuse once; one short retry puts most of them back.
            Sleep(100);
            self.originals.retain(|_, original| !original.restore());
        }
        self.save()?;
        if self.originals.is_empty() {
            Ok(())
        } else {
            let names: Vec<String> = self
                .originals
                .keys()
                .map(|&key| {
                    let h = key as HWND;
                    if IsWindow(h) != 0 {
                        format!("{} ({})", text_of(h), process_name(h))
                    } else {
                        "a window that has closed".into()
                    }
                })
                .collect();
            Err(format!(
                "These windows refused to restore their opacity: {}. The recovery record has been kept for the next launch.",
                names.join(", ")
            ))
        }
    }
    pub(super) unsafe fn enforce_rules(&mut self, allowed: impl Fn(HWND) -> bool) {
        let keys: HashSet<_> = self
            .originals
            .keys()
            .chain(self.manual.keys())
            .copied()
            .collect();
        for key in keys {
            if !allowed(key as HWND) {
                if let Err(error) = self.clear(key as HWND) {
                    log(&error);
                }
            }
        }
    }
    pub(super) unsafe fn poll(
        &mut self,
        windows: &[HWND],
        foreground: HWND,
        policy: impl Fn(HWND) -> (u8, u8),
    ) {
        let old_count = self.originals.len();
        self.originals.retain(|_, original| original.matches());
        self.manual.retain(|key, original_pid| {
            let mut pid = 0;
            GetWindowThreadProcessId(*key as HWND, &mut pid);
            IsWindow(*key as HWND) != 0 && pid == *original_pid
        });
        for &hwnd in windows {
            if self.manual.contains_key(&(hwnd as usize)) {
                continue;
            }
            let (active, inactive) = policy(hwnd);
            let desired = if hwnd == foreground { active } else { inactive };
            if desired < 100 {
                let original_percent = self
                    .originals
                    .get(&(hwnd as usize))
                    .map(|o| {
                        if o.layered && o.flags & LWA_ALPHA != 0 {
                            ((u16::from(o.alpha) * 100 + 127) / 255) as u8
                        } else {
                            100
                        }
                    })
                    .unwrap_or_else(|| self.percent(hwnd));
                let target = desired.min(original_percent);
                if self.percent(hwnd) != target {
                    let _ = self.apply(hwnd, target);
                }
            } else if self.originals.contains_key(&(hwnd as usize)) {
                let _ = self.clear(hwnd);
            }
        }
        // A minimized/closed/excluded window must not remain faded by the inactive rule.
        let release: Vec<_> = self
            .originals
            .keys()
            .filter(|key| !self.manual.contains_key(key) && !windows.contains(&(**key as HWND)))
            .copied()
            .collect();
        for key in release {
            let _ = self.clear(key as HWND);
        }
        if old_count != self.originals.len() {
            let _ = self.save();
        }
    }
}

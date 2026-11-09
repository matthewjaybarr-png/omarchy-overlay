use super::*;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Backup {
    modifiers: String,
    cycle: String,
    overrides: std::collections::BTreeMap<String, String>,
}
pub(super) unsafe fn apply(app: &mut App) -> Result<String, String> {
    let choices = &[
        ("launcher", "Win+Alt+Space"),
        ("terminal", "Win+Enter"),
        ("float", "Win+T"),
        ("help", "Win+K"),
        ("close-window", "Win+W"),
        ("maximize-window", "Win+F"),
        ("focus-left-arrow", "Win+Left"),
        ("focus-down-arrow", "Win+Down"),
        ("focus-up-arrow", "Win+Up"),
        ("focus-right-arrow", "Win+Right"),
        ("swap-left-arrow", "Win+Shift+Left"),
        ("swap-down-arrow", "Win+Shift+Down"),
        ("swap-up-arrow", "Win+Shift+Up"),
        ("swap-right-arrow", "Win+Shift+Right"),
        ("move-monitor-left", "Win+Ctrl+Shift+Left"),
        ("move-monitor-right", "Win+Ctrl+Shift+Right"),
        ("move-monitor-up", "Win+Ctrl+Shift+Up"),
        ("move-monitor-down", "Win+Ctrl+Shift+Down"),
    ];
    let mut candidate = app.config.clone();
    let mut skipped = vec![];
    let mut applied = 0;
    for (id, chord) in choices {
        let Some(index) = crate::shortcuts::ACTIONS
            .iter()
            .position(|action| action.id == *id)
        else {
            continue;
        };
        let binding = crate::shortcuts::parse(chord)?.ok_or("Invalid preset chord.")?;
        if crate::shortcuts::conflict(binding, index, &candidate).is_some() {
            skipped.push(*chord);
            continue;
        }
        let own = crate::shortcuts::ACTIONS.iter().enumerate().any(|(i, _)| {
            app.hotkeys
                .contains(&(BASE as i32 + crate::shortcuts::action_slot(i) as i32))
                && crate::shortcuts::parse(&crate::shortcuts::chord(
                    i,
                    &app.config.modifiers,
                    &app.config.shortcut_overrides,
                ))
                .ok()
                .flatten()
                    == Some(binding)
        });
        let probe = BASE as i32 + 513;
        if !own
            && RegisterHotKey(
                app.main,
                probe,
                binding.modifiers | MOD_NOREPEAT,
                binding.key,
            ) == 0
        {
            skipped.push(*chord);
            continue;
        }
        if !own {
            UnregisterHotKey(app.main, probe);
        }
        candidate
            .shortcut_overrides
            .insert((*id).into(), (*chord).into());
        applied += 1;
    }
    candidate.validate()?;
    let backup = Backup {
        modifiers: app.config.modifiers.clone(),
        cycle: app.config.cycle_workspace_shortcut.clone(),
        overrides: app.config.shortcut_overrides.clone(),
    };
    write_json_atomic(
        &app.directory.join("shortcuts-before-omarchy.json"),
        &backup,
    )?;
    app.config.shortcut_overrides = candidate.shortcut_overrides;
    app.rebind_hotkeys();
    app.save_config();
    Ok(format!("Applied {applied} compatible Omarchy-style keys. Skipped conflicts: {}. Undo preset restores the previous shortcuts.",if skipped.is_empty(){"none".into()}else{skipped.join(", ")}))
}
pub(super) unsafe fn restore(app: &mut App) -> Result<String, String> {
    let backup: Backup = serde_json::from_str(&profile::read_text(
        &app.directory.join("shortcuts-before-omarchy.json"),
    )?)
    .map_err(|e| e.to_string())?;
    let mut candidate = app.config.clone();
    candidate.modifiers = backup.modifiers;
    candidate.cycle_workspace_shortcut = backup.cycle;
    candidate.shortcut_overrides = backup.overrides;
    candidate.validate()?;
    app.config = candidate;
    app.rebind_hotkeys();
    app.save_config();
    set_text(
        GetDlgItem(app.settings.hwnd, 71),
        &app.config.cycle_workspace_shortcut,
    );
    Ok("Previous shortcut choices restored.".into())
}

//! Conservative selection from Explorer's SystemTrayIcon accessibility controls.
#[derive(Debug)]
pub struct Control {
    pub id: String,
    pub class: String,
    pub name: String,
}
pub fn select(controls: &[Control]) -> Result<usize, &'static str> {
    let unique = |matches: Vec<usize>| match matches.as_slice() {
        [one] => Ok(*one),
        [] => Err("No supported hidden-icons control was found"),
        _ => Err("Multiple hidden-icons controls matched; refusing to choose one"),
    };
    let known: Vec<_> = controls
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            (c.id == "SystemTrayIcon"
                && ["SystemTray.NormalButton", "SystemTray.ChevronIconView"]
                    .contains(&c.class.as_str()))
            .then_some(i)
        })
        .collect();
    if !known.is_empty() {
        return unique(known);
    }
    unique(
        controls
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let name = c
                    .name
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase();
                (c.id == "SystemTrayIcon"
                    && c.class == "SystemTray.AccentButton"
                    && ["show hidden icons", "hidden icons"].contains(&name.as_str()))
                .then_some(i)
            })
            .collect(),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn control(class: &str, name: &str) -> Control {
        Control {
            id: "SystemTrayIcon".into(),
            class: class.into(),
            name: name.into(),
        }
    }
    #[test]
    fn five_accent_buttons_select_only_hidden_icons() {
        let controls = [
            control("SystemTray.AccentButton", "Network Corporate"),
            control("SystemTray.AccentButton", "Volume 50%"),
            control("SystemTray.AccentButton", "Show Hidden Icons"),
            control("SystemTray.AccentButton", "Clock"),
            control("SystemTray.AccentButton", "Battery"),
        ];
        assert_eq!(select(&controls), Ok(2));
    }
    #[test]
    fn ambiguous_missing_and_wrong_id_fail_closed() {
        assert!(select(&[
            control("SystemTray.AccentButton", "Network"),
            control("SystemTray.AccentButton", "Volume")
        ])
        .is_err());
        assert!(select(&[
            control("SystemTray.AccentButton", "Show hidden icons"),
            control("SystemTray.AccentButton", "Hidden icons")
        ])
        .is_err());
        let mut other = control("SystemTray.AccentButton", "Show hidden icons");
        other.id = "NotifyItemIcon".into();
        assert!(select(&[other]).is_err());
    }
    #[test]
    fn existing_classes_keep_localized_support() {
        assert_eq!(
            select(&[control("SystemTray.NormalButton", "Mas icons")]),
            Ok(0)
        );
        assert_eq!(
            select(&[control("SystemTray.ChevronIconView", "Autres icones")]),
            Ok(0)
        );
        assert_eq!(
            select(&[control("SystemTray.AccentButton", " Show  hidden icons \n")]),
            Ok(0)
        );
    }
}

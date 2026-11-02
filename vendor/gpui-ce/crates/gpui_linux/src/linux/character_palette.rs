use gpui::BackgroundExecutor;
use smol::process::Command;

type PaletteCommand<'a> = (&'a str, &'a [&'a str]);

fn palette_commands(desktop: &str) -> [PaletteCommand<'static>; 5] {
    macro_rules! palette_command {
        ($name:ident, $program:literal $(, $argument:literal)*) => {
            const $name: PaletteCommand<'static> = ($program, &[$($argument),*]);
        };
    }

    // Upstream sources for the executable names:
    // [plasma-emojier](https://github.com/KDE/plasma-desktop/blob/master/emojier/app/CMakeLists.txt#L1),
    // [ibus-ui-emojier-plasma (legacy name)](https://github.com/KDE/plasma-desktop/blob/902cb77151e29e77b90c35bb7b9c2a265d2901aa/applets/kimpanel/backend/ibus/emojier/app/CMakeLists.txt#L1),
    // [ibus emoji command](https://github.com/ibus/ibus/blob/main/tools/ibus.1.in#L118-L125),
    // [gnome-characters symlink](https://github.com/GNOME/gnome-characters/blob/main/src/meson.build#L16-L20),
    // [kcharselect](https://github.com/KDE/kcharselect/blob/master/CMakeLists.txt#L52-L54).
    palette_command!(PLASMA, "plasma-emojier");
    palette_command!(LEGACY_PLASMA, "ibus-ui-emojier-plasma");
    palette_command!(IBUS, "ibus", "emoji");
    palette_command!(GNOME_CHARACTERS, "gnome-characters");
    palette_command!(KCHARSELECT, "kcharselect");

    if desktop
        .split(':')
        .any(|name| name.eq_ignore_ascii_case("KDE"))
    {
        [PLASMA, LEGACY_PLASMA, IBUS, GNOME_CHARACTERS, KCHARSELECT]
    } else {
        [GNOME_CHARACTERS, IBUS, PLASMA, LEGACY_PLASMA, KCHARSELECT]
    }
}

async fn launch_palette(commands: &[PaletteCommand<'_>], activation_token: Option<&str>) -> bool {
    for (program, arguments) in commands {
        let mut command = Command::new(program);
        command.args(*arguments);
        if let Some(token) = activation_token {
            command.env("XDG_ACTIVATION_TOKEN", token);
        }
        match command.status().await {
            Ok(status) if status.success() => return true,
            Ok(status) => log::warn!("Character palette {program} exited with {status}"),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => log::warn!("Failed to open character palette {program}: {error}"),
        }
    }
    false
}

pub(super) fn show_character_palette(
    executor: BackgroundExecutor,
    activation_token: Option<String>,
) {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    executor
        .spawn(async move {
            if !launch_palette(&palette_commands(&desktop), activation_token.as_deref()).await {
                log::warn!(
                    "No character palette is available; install Plasma's Emoji Selector, IBus, \
                     GNOME Characters, or KCharSelect"
                );
            }
        })
        .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_the_desktop_picker() {
        assert_eq!(
            palette_commands("ubuntu:GNOME")[0],
            ("gnome-characters", &[][..])
        );
        assert_eq!(palette_commands("KDE")[0].0, "plasma-emojier");
        assert_eq!(palette_commands("other:kde")[0].0, "plasma-emojier");
    }

    #[test]
    fn falls_back_after_missing_or_failing_commands() {
        assert!(smol::block_on(launch_palette(
            &[
                ("/nonexistent/gpui-character-palette", &[]),
                ("/bin/sh", &["-c", "exit 1"]),
                ("/bin/sh", &["-c", "exit 0"]),
            ],
            None,
        )));
    }

    #[test]
    fn reports_unavailable_pickers() {
        assert!(!smol::block_on(launch_palette(
            &[
                ("/nonexistent/gpui-character-palette", &[]),
                ("/bin/sh", &["-c", "exit 1"]),
            ],
            None,
        )));
    }

    #[test]
    fn passes_the_wayland_activation_token() {
        assert!(smol::block_on(launch_palette(
            &[(
                "/bin/sh",
                &["-c", "test \"$XDG_ACTIVATION_TOKEN\" = gpui-palette-test"]
            )],
            Some("gpui-palette-test"),
        )));
    }
}

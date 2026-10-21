//! Which keys do what, by mode. Pure, so it tests without Bevy.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    World,
    CommandLine,
    Inventory,
    Settings,
    /// The key bindings screen (K).
    Keys,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyName {
    A,
    E,
    G,
    D,
    X,
    I,
    L,
    M,
    C,
    T,
    K,
    Space,
    Tab,
    Escape,
    Enter,
    Digit(u8),
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Apply,
    Get,
    /// Quick slot 0..=3 (keys 1-4).
    Slot(usize),
    OpenInventory,
    CloseInventory,
    OpenSettings,
    CloseSettings,
    /// K: the key bindings screen.
    OpenKeys,
    CloseKeys,
    ToggleLog,
    ToggleMute,
    /// Snap the camera straight behind the player.
    CameraBehind,
    /// T: turn to face the nearest creature.
    FaceNearest,
    /// Space: fire the readied bow or wand.
    Fire,
    /// Tab: the top view on or off.
    TopView,
    CloseBox,
    OpenCommand,
    Nothing,
}

/// The spec's key table. In the command line the text box owns the keys; in the
/// inventory only I and Escape act here (the inventory screen handles the rest).
pub fn act(mode: Mode, key: KeyName, box_open: bool) -> Act {
    match (mode, key) {
        (Mode::CommandLine, _) => Act::Nothing,
        (Mode::Inventory, KeyName::I | KeyName::Escape) => Act::CloseInventory,
        (Mode::Inventory, _) => Act::Nothing,
        (Mode::Settings, KeyName::Escape) => Act::CloseSettings,
        (Mode::Settings, _) => Act::Nothing,
        (Mode::Keys, KeyName::K | KeyName::Escape) => Act::CloseKeys,
        (Mode::Keys, _) => Act::Nothing,
        (Mode::World, KeyName::E | KeyName::Escape) if box_open => Act::CloseBox,
        (Mode::World, KeyName::Escape) => Act::OpenSettings,
        (Mode::World, KeyName::A | KeyName::E) => Act::Apply,
        (Mode::World, KeyName::G) => Act::Get,
        (Mode::World, KeyName::Digit(n @ 1..=4)) => Act::Slot(n as usize - 1),
        (Mode::World, KeyName::I) => Act::OpenInventory,
        (Mode::World, KeyName::K) => Act::OpenKeys,
        (Mode::World, KeyName::L) => Act::ToggleLog,
        (Mode::World, KeyName::M) => Act::ToggleMute,
        (Mode::World, KeyName::C) => Act::CameraBehind,
        (Mode::World, KeyName::T) => Act::FaceNearest,
        (Mode::World, KeyName::Space) => Act::Fire,
        (Mode::World, KeyName::Tab) => Act::TopView,
        (Mode::World, KeyName::Enter) => Act::OpenCommand,
        (Mode::World, _) => Act::Nothing,
    }
}

/// Bound keys run only while playing in the world, never over a screen or the command box.
pub fn bindings_active(mode: Mode) -> bool {
    mode == Mode::World
}

/// M mutes only while playing in the world, never while typing (login form, command box).
pub fn mute_key_allowed(in_game: bool, mode: Mode) -> bool {
    in_game && mode == Mode::World
}

/// Enter may open (or use) the command box only from the world or the box itself.
pub fn command_line_allowed(mode: Mode) -> bool {
    matches!(mode, Mode::World | Mode::CommandLine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn k_opens_and_closes_keys() {
        assert_eq!(act(Mode::World, KeyName::K, false), Act::OpenKeys);
        assert_eq!(act(Mode::Keys, KeyName::K, false), Act::CloseKeys);
        assert_eq!(act(Mode::Keys, KeyName::Escape, false), Act::CloseKeys);
        assert_eq!(act(Mode::Keys, KeyName::A, false), Act::Nothing);
    }

    #[test]
    fn bindings_only_in_world() {
        assert!(bindings_active(Mode::World));
        for m in [Mode::CommandLine, Mode::Inventory, Mode::Settings, Mode::Keys] {
            assert!(!bindings_active(m));
        }
    }

    #[test]
    fn enter_does_not_open_command_box_over_screens() {
        assert!(command_line_allowed(Mode::World));
        assert!(command_line_allowed(Mode::CommandLine));
        assert!(!command_line_allowed(Mode::Settings));
        assert!(!command_line_allowed(Mode::Inventory));
    }

    #[test]
    fn m_on_login_form_does_not_mute() {
        assert!(!mute_key_allowed(false, Mode::World));
        assert!(!mute_key_allowed(true, Mode::CommandLine));
        assert!(mute_key_allowed(true, Mode::World));
    }

    #[test]
    fn e_closes_box_before_applying() {
        assert_eq!(act(Mode::World, KeyName::E, true), Act::CloseBox);
        assert_eq!(act(Mode::World, KeyName::E, false), Act::Apply);
        assert_eq!(act(Mode::World, KeyName::Escape, true), Act::CloseBox);
        assert_eq!(act(Mode::World, KeyName::Escape, false), Act::OpenSettings);
    }

    #[test]
    fn escape_opens_settings_only_when_nothing_open() {
        assert_eq!(act(Mode::World, KeyName::Escape, false), Act::OpenSettings);
        assert_eq!(act(Mode::World, KeyName::Escape, true), Act::CloseBox);
        assert_eq!(act(Mode::Inventory, KeyName::Escape, false), Act::CloseInventory);
        assert_eq!(act(Mode::CommandLine, KeyName::Escape, false), Act::Nothing);
    }

    #[test]
    fn escape_closes_settings() {
        assert_eq!(act(Mode::Settings, KeyName::Escape, false), Act::CloseSettings);
    }

    #[test]
    fn settings_swallows_keys() {
        for k in [KeyName::E, KeyName::I, KeyName::M, KeyName::Digit(1), KeyName::Enter] {
            assert_eq!(act(Mode::Settings, k, false), Act::Nothing, "{k:?}");
        }
        assert!(!mute_key_allowed(true, Mode::Settings));
    }

    #[test]
    fn c_snaps_camera_in_world_only() {
        assert_eq!(act(Mode::World, KeyName::C, false), Act::CameraBehind);
        assert_eq!(act(Mode::Inventory, KeyName::C, false), Act::Nothing);
        assert_eq!(act(Mode::CommandLine, KeyName::C, false), Act::Nothing);
        assert_eq!(act(Mode::Settings, KeyName::C, false), Act::Nothing);
    }

    #[test]
    fn a_applies_and_tab_toggles_top_view() {
        assert_eq!(act(Mode::World, KeyName::A, false), Act::Apply);
        assert_eq!(act(Mode::World, KeyName::Tab, false), Act::TopView);
        assert_eq!(act(Mode::Inventory, KeyName::Tab, false), Act::Nothing);
    }

    #[test]
    fn world_keys() {
        assert_eq!(act(Mode::World, KeyName::G, false), Act::Get);
        assert_eq!(act(Mode::World, KeyName::I, false), Act::OpenInventory);
        assert_eq!(act(Mode::World, KeyName::L, false), Act::ToggleLog);
        assert_eq!(act(Mode::World, KeyName::M, false), Act::ToggleMute);
        assert_eq!(act(Mode::World, KeyName::Enter, false), Act::OpenCommand);
        assert_eq!(act(Mode::World, KeyName::D, false), Act::Nothing); // D walks west
        assert_eq!(act(Mode::World, KeyName::X, false), Act::Nothing);
    }

    #[test]
    fn digits_use_slots_in_world() {
        assert_eq!(act(Mode::World, KeyName::Digit(1), false), Act::Slot(0));
        assert_eq!(act(Mode::World, KeyName::Digit(4), false), Act::Slot(3));
        assert_eq!(act(Mode::World, KeyName::Digit(5), false), Act::Nothing);
        assert_eq!(act(Mode::World, KeyName::Digit(0), false), Act::Nothing);
    }

    #[test]
    fn command_line_swallows_letters() {
        for k in [KeyName::E, KeyName::G, KeyName::I, KeyName::M, KeyName::Digit(1), KeyName::Escape, KeyName::Enter] {
            assert_eq!(act(Mode::CommandLine, k, true), Act::Nothing, "{k:?}");
        }
    }

    #[test]
    fn inventory_swallows_world_keys() {
        for k in [KeyName::E, KeyName::G, KeyName::M, KeyName::L, KeyName::Digit(1), KeyName::Enter] {
            assert_eq!(act(Mode::Inventory, k, false), Act::Nothing, "{k:?}");
        }
    }

    #[test]
    fn escape_closes_inventory() {
        assert_eq!(act(Mode::Inventory, KeyName::Escape, false), Act::CloseInventory);
        assert_eq!(act(Mode::Inventory, KeyName::I, false), Act::CloseInventory);
    }
}

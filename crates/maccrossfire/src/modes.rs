//! Which keys do what, by mode. Pure, so it tests without Bevy.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    World,
    CommandLine,
    Inventory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyName {
    E,
    G,
    D,
    X,
    I,
    L,
    M,
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
    ToggleLog,
    ToggleMute,
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
        (Mode::World, KeyName::E | KeyName::Escape) if box_open => Act::CloseBox,
        (Mode::World, KeyName::E) => Act::Apply,
        (Mode::World, KeyName::G) => Act::Get,
        (Mode::World, KeyName::Digit(n @ 1..=4)) => Act::Slot(n as usize - 1),
        (Mode::World, KeyName::I) => Act::OpenInventory,
        (Mode::World, KeyName::L) => Act::ToggleLog,
        (Mode::World, KeyName::M) => Act::ToggleMute,
        (Mode::World, KeyName::Enter) => Act::OpenCommand,
        (Mode::World, _) => Act::Nothing,
    }
}

/// M mutes only while playing in the world, never while typing (login form, command box).
pub fn mute_key_allowed(in_game: bool, mode: Mode) -> bool {
    in_game && mode == Mode::World
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(act(Mode::World, KeyName::Escape, false), Act::Nothing);
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

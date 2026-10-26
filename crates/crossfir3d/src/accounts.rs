//! The accounts used on this machine and their characters, for the login screen.
//! Passwords are never kept.

use std::path::{Path, PathBuf};

use cfproto::CharacterInfo;

#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub name: String,
    pub characters: Vec<CharacterInfo>,
}

/// Lines `account[<TAB>character<TAB>race<TAB>class<TAB>level]`, the last used account first.
pub fn load_accounts(path: &Path) -> Vec<Account> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut accounts: Vec<Account> = Vec::new();
    for line in text.lines().filter(|l| !l.is_empty()) {
        let mut f = line.split('\t');
        let name = f.next().unwrap_or_default();
        if accounts.last().is_none_or(|a| a.name != name) {
            accounts.push(Account { name: name.to_string(), characters: vec![] });
        }
        if let (Some(c), Some(race), Some(class), Some(level)) = (f.next(), f.next(), f.next(), f.next()) {
            let info = CharacterInfo { name: c.into(), race: race.into(), class: class.into(), level: level.parse().unwrap_or(0), face: None };
            accounts.last_mut().expect("pushed above").characters.push(info);
        }
    }
    accounts
}

/// Puts this account and its characters first and keeps the other accounts.
pub fn remember_account(path: &Path, account: &str, characters: &[CharacterInfo]) -> std::io::Result<()> {
    let bad = |s: &str| s.is_empty() || s.contains(['\t', '\n', '\r']);
    if bad(account) {
        return Ok(());
    }
    let mut lines = vec![account.to_string()];
    for c in characters.iter().filter(|c| ![&c.name, &c.race, &c.class].iter().any(|s| bad(s))) {
        lines.push(format!("{account}\t{}\t{}\t{}\t{}", c.name, c.race, c.class, c.level));
    }
    let old = std::fs::read_to_string(path).unwrap_or_default();
    lines.extend(old.lines().filter(|l| l.split('\t').next() != Some(account)).map(String::from));
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // A crash mid-write must not lose the list: write a copy, then swap it in.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, lines.join("\n") + "\n")?;
    std::fs::rename(tmp, path)
}

/// Next to the quick slots and settings.
pub fn default_accounts_path() -> PathBuf {
    crate::settings::data_dir().join("accounts.txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ch(name: &str) -> CharacterInfo {
        CharacterInfo { name: name.into(), race: "Human".into(), class: "Wizard".into(), level: 3, face: None }
    }

    #[test]
    fn last_used_account_comes_first_with_its_characters() {
        let dir = std::env::temp_dir().join(format!("cf3d-accounts-{}", std::process::id()));
        let path = dir.join("accounts.txt");
        let _ = std::fs::remove_file(&path);
        assert!(load_accounts(&path).is_empty());
        remember_account(&path, "ann", &[ch("Ada"), ch("Bea")]).unwrap();
        remember_account(&path, "bob", &[]).unwrap();
        remember_account(&path, "bad\tname", &[ch("X")]).unwrap();
        let got = load_accounts(&path);
        assert_eq!(got.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["bob", "ann"]);
        assert_eq!(got[1].characters, [ch("Ada"), ch("Bea")]);
        // Logging in again replaces the account's characters and moves it first.
        remember_account(&path, "ann", &[ch("Cy")]).unwrap();
        let got = load_accounts(&path);
        assert_eq!(got[0], Account { name: "ann".into(), characters: vec![ch("Cy")] });
        assert_eq!(got.len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }
}

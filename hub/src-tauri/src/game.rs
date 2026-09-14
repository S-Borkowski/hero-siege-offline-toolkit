//! Is Hero Siege running, and is Easy Anti-Cheat up?
//!
//! Two of the D5 interlocks depend on the answer, and the status bar shows it so
//! that "staged for next launch" reads as a consequence of something visible
//! rather than as the hub being awkward.

use serde::Serialize;

const GAME_EXE: &str = "hero_siege.exe";
const EAC_PREFIX: &str = "easyanticheat";

#[derive(Debug, Clone, Default, Serialize)]
pub struct GameStatus {
    pub running: bool,
    pub pid: Option<u32>,
    pub exe_path: Option<String>,
    /// Whether an EasyAntiCheat process is up. The toolkit's tools all require
    /// offline single-player with EAC off, so this is a warning surface, not a
    /// thing the hub ever changes.
    pub eac_running: bool,
}

/// Read the game's state from a snapshot the caller already has.
///
/// `build_view` needs this alongside two other answers from the same process
/// table, so it takes the snapshot once and passes it in rather than having each
/// question enumerate for itself.
pub fn status_from(snapshot: &crate::procs::Snapshot) -> GameStatus {
    read_status(snapshot.iter().map(|proc| {
        (
            proc.name.clone(),
            proc.pid,
            proc.exe.as_ref().map(|p| p.to_string_lossy().to_string()),
        )
    }))
}

/// For callers with no snapshot of their own -- the `game_status` command.
pub fn status() -> GameStatus {
    status_from(&crate::procs::Snapshot::take())
}

/// The decision, separated from the process enumeration so it can be tested.
fn read_status(
    processes: impl Iterator<Item = (String, u32, Option<String>)>,
) -> GameStatus {
    let mut status = GameStatus::default();
    for (name, pid, exe) in processes {
        let lowered = name.to_ascii_lowercase();
        if lowered == GAME_EXE && !status.running {
            status.running = true;
            status.pid = Some(pid);
            status.exe_path = exe;
        } else if lowered.starts_with(EAC_PREFIX) {
            status.eac_running = true;
        }
    }
    status
}

/// Why an install must not be applied right now, if it must not be.
///
/// Returns the sentence the Updates view shows. ForgePact patches the game's PE
/// and holds file IPC through `bp_ipc/cmd.txt`; HSSaveEditor documents a
/// game-closed requirement. Writing over either while the game is up is how a
/// player loses a character, so a staged install waits and says so.
pub fn install_blocked_by(
    tool_name: &str,
    tool_is_running: bool,
    game: &GameStatus,
) -> Option<String> {
    if tool_is_running {
        return Some(format!(
            "{tool_name} is running. The update will be applied when it is closed."
        ));
    }
    if game.running {
        return Some(
            "Hero Siege is running. The update will be applied when the game is closed."
                .to_string(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn processes(items: &[(&str, u32)]) -> Vec<(String, u32, Option<String>)> {
        items
            .iter()
            .map(|(name, pid)| (name.to_string(), *pid, None))
            .collect()
    }

    #[test]
    fn the_game_is_found_whatever_case_windows_reports() {
        let status = read_status(processes(&[("Hero_Siege.exe", 4242)]).into_iter());
        assert!(status.running);
        assert_eq!(status.pid, Some(4242));

        let status = read_status(processes(&[("hero_siege.exe", 7)]).into_iter());
        assert!(status.running);
    }

    #[test]
    fn an_unrelated_process_is_not_the_game() {
        let status = read_status(
            processes(&[("explorer.exe", 1), ("hero_siege_launcher.exe", 2)]).into_iter(),
        );
        assert!(!status.running);
        assert!(!status.eac_running);
    }

    #[test]
    fn eac_is_noticed_under_either_of_its_names() {
        for name in ["EasyAntiCheat.exe", "EasyAntiCheat_EOS.exe"] {
            let status = read_status(processes(&[(name, 3)]).into_iter());
            assert!(status.eac_running, "{name}");
        }
    }

    #[test]
    fn nothing_running_means_nothing_blocked() {
        let idle = GameStatus::default();
        assert_eq!(install_blocked_by("ForgePact", false, &idle), None);
    }

    #[test]
    fn a_running_tool_blocks_its_own_update() {
        let idle = GameStatus::default();
        let reason = install_blocked_by("ForgePact", true, &idle).unwrap();
        assert!(reason.contains("ForgePact is running"), "{reason}");
    }

    #[test]
    fn a_running_game_blocks_every_update() {
        let playing = GameStatus {
            running: true,
            pid: Some(1),
            ..GameStatus::default()
        };
        let reason = install_blocked_by("HSCraftSim", false, &playing).unwrap();
        assert!(reason.contains("Hero Siege is running"), "{reason}");
    }

    #[test]
    fn the_tool_being_up_is_reported_before_the_game_being_up() {
        // Both are true; the message that helps says which thing to close first.
        let playing = GameStatus {
            running: true,
            ..GameStatus::default()
        };
        let reason = install_blocked_by("ForgePact", true, &playing).unwrap();
        assert!(reason.starts_with("ForgePact"), "{reason}");
    }

    #[test]
    fn enumerating_the_real_process_table_does_not_panic() {
        // This machine is not running Hero Siege during a test run, so all that
        // is asserted is that the syscall path works.
        let _ = status();
    }
}

//! One look at the process table, and the three questions asked of it.
//!
//! `build_view` needs to know whether Hero Siege is up, whether the PIDs it
//! tracks are still alive, and whether a tool is running that it never started.
//! Those were three separate enumerations: a full `sysinfo` snapshot in
//! `game::status`, one single-PID refresh per running tool in `launch::is_alive`,
//! and up to four blocking HTTP probes. `build_view` runs on every `announce`
//! and on a ten-second poll, so that added up for no reason -- one snapshot
//! answers all three.
//!
//! It also closes a hole. `Hub.running` lives in memory, so every tracked PID is
//! lost when the hub restarts. The fallback was a health probe or a declared
//! port, and four tools have neither: `hssaveeditor`, `hs-value-editor`,
//! `hs-offline-loot-forge` and `hs-stat-forge` became invisible while running,
//! and their cards offered Launch for something already open -- which, for the
//! ones with a single-instance lock, then failed. A process whose executable
//! lies inside the hub's install directory for a tool *is* that tool, whatever
//! it is called and whoever started it, so that is what gets matched.

use std::path::{Path, PathBuf};

/// The fields of a process this hub has any use for.
#[derive(Debug, Clone)]
pub struct Proc {
    pub pid: u32,
    pub parent: Option<u32>,
    pub name: String,
    pub exe: Option<PathBuf>,
}

/// A point-in-time copy of the process table.
///
/// One snapshot rather than several, so the answers cannot disagree with each
/// other -- and so a PID cannot be reused between two of them and have the hub
/// act on the wrong process.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    procs: Vec<Proc>,
}

impl Snapshot {
    pub fn take() -> Self {
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        Self::from_system(&system)
    }

    /// Read from a `System` the caller keeps.
    ///
    /// `stop` needs both this and the live `Process` handles to call `kill` on,
    /// and it has to be the *same* enumeration for both: deciding the tree from
    /// one and killing from another leaves a window where a PID is reused and
    /// the kill lands on something unrelated.
    pub fn from_system(system: &sysinfo::System) -> Self {
        Self {
            procs: system
                .processes()
                .iter()
                .map(|(pid, process)| Proc {
                    pid: pid.as_u32(),
                    parent: process.parent().map(|p| p.as_u32()),
                    name: process.name().to_string_lossy().to_string(),
                    exe: process.exe().map(|p| p.to_path_buf()),
                })
                .collect(),
        }
    }

    /// Build one from parts, for tests.
    pub fn of(procs: Vec<Proc>) -> Self {
        Self { procs }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Proc> {
        self.procs.iter()
    }

    pub fn is_alive(&self, pid: u32) -> bool {
        self.procs.iter().any(|p| p.pid == pid)
    }

    /// The PID of a process running out of `dir`, if there is one.
    ///
    /// Prefers the lowest PID for stability across refreshes: a PyInstaller
    /// one-file build shows up twice -- the bootloader and the child it
    /// unpacked -- and the card should not flip between them every ten seconds.
    pub fn find_under(&self, dir: &Path) -> Option<u32> {
        self.procs
            .iter()
            .filter(|p| p.exe.as_deref().is_some_and(|exe| is_under(exe, dir)))
            .map(|p| p.pid)
            .min()
    }

    /// Depth-first walk of a process tree, children before parents.
    ///
    /// The PID the hub spawns is not always the application: a PyInstaller
    /// one-file build runs a bootloader that unpacks itself, spawns the real
    /// program as a child, and waits. Killing only the tracked PID killed the
    /// bootloader -- measured on ForgePact 1.3.16 as an 8 MB parent and a 102 MB
    /// child still serving its port.
    pub fn tree_order(&self, root: u32) -> Vec<u32> {
        let mut children: std::collections::HashMap<u32, Vec<u32>> =
            std::collections::HashMap::new();
        for proc in &self.procs {
            if let Some(parent) = proc.parent {
                children.entry(parent).or_default().push(proc.pid);
            }
        }
        // A process reported as its own ancestor would loop forever. It should
        // not happen; `seen` means it cannot.
        let mut seen = std::collections::HashSet::new();
        let mut order = Vec::new();
        let mut stack = vec![root];
        while let Some(pid) = stack.pop() {
            if !seen.insert(pid) {
                continue;
            }
            order.push(pid);
            if let Some(kids) = children.get(&pid) {
                stack.extend(kids.iter().copied());
            }
        }
        // Pre-order visits a parent before its children; reversed, children die
        // first, so a parent cannot outlive the kill and respawn one.
        order.reverse();
        order
    }
}

/// Is `exe` inside `dir`?
///
/// Case-insensitive, because Windows paths are: the process table reports
/// whatever casing the caller used to start the program, and the hub's own
/// record of the install directory comes from `%LOCALAPPDATA%`, so the two
/// routinely differ in case while naming the same file.
///
/// The boundary check is what stops `...\tools\forgepact\1.3.16` matching a
/// process under `...\tools\forgepact\1.3.160`.
pub fn is_under(exe: &Path, dir: &Path) -> bool {
    let normalise = |path: &Path| {
        path.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_lowercase()
    };
    let exe = normalise(exe);
    let dir = normalise(dir);
    if dir.is_empty() || exe.len() <= dir.len() {
        return false;
    }
    exe.starts_with(&dir) && exe.as_bytes()[dir.len()] == b'\\'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proc(pid: u32, parent: Option<u32>, name: &str, exe: Option<&str>) -> Proc {
        Proc {
            pid,
            parent,
            name: name.to_string(),
            exe: exe.map(PathBuf::from),
        }
    }

    const INSTALL: &str = r"C:\Users\a\AppData\Local\Hero Siege Toolkit\tools\hs-stat-forge\2.5.0";

    #[test]
    fn an_exe_inside_the_install_directory_is_the_tool() {
        assert!(is_under(
            Path::new(&format!(r"{INSTALL}\HSStatForge.exe")),
            Path::new(INSTALL)
        ));
        // Nested too -- some tools keep their executable in a subdirectory.
        assert!(is_under(
            Path::new(&format!(r"{INSTALL}\bin\HSStatForge.exe")),
            Path::new(INSTALL)
        ));
    }

    #[test]
    fn the_same_filename_somewhere_else_is_not_the_tool() {
        // A copy the player keeps on the desktop, or a previous manual install.
        for elsewhere in [
            r"C:\Users\a\Desktop\HSStatForge.exe",
            r"C:\Program Files\HSStatForge\HSStatForge.exe",
            r"C:\Users\a\AppData\Local\Hero Siege Toolkit\tools\forgepact\1.3.16\HSStatForge.exe",
        ] {
            assert!(
                !is_under(Path::new(elsewhere), Path::new(INSTALL)),
                "{elsewhere} should not match"
            );
        }
    }

    #[test]
    fn a_difference_in_case_still_matches() {
        // Windows paths are case-insensitive, and the casing the process table
        // reports is whatever was used to launch the program.
        assert!(is_under(
            Path::new(&INSTALL.to_uppercase()).join("HSSTATFORGE.EXE").as_path(),
            Path::new(INSTALL)
        ));
        assert!(is_under(
            Path::new(&format!(r"{}\hsstatforge.exe", INSTALL.to_lowercase())),
            Path::new(INSTALL)
        ));
    }

    #[test]
    fn a_sibling_directory_with_a_longer_name_does_not_match() {
        // The boundary check: 1.3.16 must not match 1.3.160.
        let dir = r"C:\t\tools\forgepact\1.3.16";
        assert!(!is_under(Path::new(r"C:\t\tools\forgepact\1.3.160\ForgePact.exe"), Path::new(dir)));
        assert!(is_under(Path::new(r"C:\t\tools\forgepact\1.3.16\ForgePact.exe"), Path::new(dir)));
    }

    #[test]
    fn forward_and_back_slashes_are_the_same_path() {
        assert!(is_under(
            Path::new("C:/t/tools/forgepact/1.3.16/ForgePact.exe"),
            Path::new(r"C:\t\tools\forgepact\1.3.16")
        ));
    }

    #[test]
    fn the_directory_itself_is_not_a_process_in_it() {
        assert!(!is_under(Path::new(INSTALL), Path::new(INSTALL)));
    }

    #[test]
    fn an_empty_directory_matches_nothing() {
        assert!(!is_under(Path::new(r"C:\anything\at\all.exe"), Path::new("")));
    }

    #[test]
    fn find_under_reports_the_process_running_from_the_install() {
        let snapshot = Snapshot::of(vec![
            proc(1, None, "explorer.exe", Some(r"C:\Windows\explorer.exe")),
            proc(4242, Some(1), "HSStatForge.exe", Some(&format!(r"{INSTALL}\HSStatForge.exe"))),
            proc(9, Some(1), "HSStatForge.exe", Some(r"C:\Users\a\Desktop\HSStatForge.exe")),
        ]);
        assert_eq!(snapshot.find_under(Path::new(INSTALL)), Some(4242));
    }

    #[test]
    fn find_under_prefers_the_lowest_pid_so_the_card_does_not_flip() {
        // A PyInstaller one-file build appears twice: bootloader and child.
        let snapshot = Snapshot::of(vec![
            proc(500, Some(1), "ForgePact.exe", Some(&format!(r"{INSTALL}\ForgePact.exe"))),
            proc(900, Some(500), "ForgePact.exe", Some(&format!(r"{INSTALL}\ForgePact.exe"))),
        ]);
        assert_eq!(snapshot.find_under(Path::new(INSTALL)), Some(500));
        assert_eq!(snapshot.find_under(Path::new(INSTALL)), Some(500));
    }

    #[test]
    fn find_under_answers_none_when_nothing_is_running_from_there() {
        let snapshot = Snapshot::of(vec![proc(1, None, "explorer.exe", Some(r"C:\Windows\explorer.exe"))]);
        assert_eq!(snapshot.find_under(Path::new(INSTALL)), None);
    }

    #[test]
    fn a_process_with_no_readable_exe_path_is_skipped_not_matched() {
        // Access to another user's process, or a system process, gives None.
        let snapshot = Snapshot::of(vec![proc(4, Some(0), "System", None)]);
        assert_eq!(snapshot.find_under(Path::new(INSTALL)), None);
        assert!(snapshot.is_alive(4));
    }

    #[test]
    fn liveness_comes_from_the_same_snapshot() {
        let snapshot = Snapshot::of(vec![proc(7, None, "a.exe", None)]);
        assert!(snapshot.is_alive(7));
        assert!(!snapshot.is_alive(8));
    }

    #[test]
    fn a_process_tree_is_ordered_children_first() {
        let snapshot = Snapshot::of(vec![
            proc(1, None, "a", None),
            proc(100, Some(1), "b", None),
            proc(200, Some(100), "c", None),
            proc(300, Some(100), "d", None),
            proc(400, Some(200), "e", None),
            proc(999, Some(1), "unrelated", None),
        ]);
        let order = snapshot.tree_order(100);

        assert_eq!(order.len(), 4, "{order:?}");
        assert!(!order.contains(&1), "walked up to the parent: {order:?}");
        assert!(!order.contains(&999), "picked up an unrelated process: {order:?}");

        let at = |pid: u32| order.iter().position(|p| *p == pid).unwrap();
        assert!(at(400) < at(200), "grandchild must die before its parent");
        assert!(at(200) < at(100), "child must die before its parent");
        assert!(at(300) < at(100), "child must die before its parent");
    }

    #[test]
    fn a_parent_cycle_does_not_hang_the_walk() {
        let snapshot = Snapshot::of(vec![proc(10, Some(20), "a", None), proc(20, Some(10), "b", None)]);
        assert_eq!(snapshot.tree_order(10).len(), 2);
    }

    #[test]
    fn taking_a_real_snapshot_sees_this_process() {
        let snapshot = Snapshot::take();
        assert!(snapshot.is_alive(std::process::id()));
        assert!(snapshot.iter().count() > 1);
        // The test binary is running from somewhere, so at least one process
        // must have a readable path -- otherwise `find_under` could never work.
        assert!(snapshot.iter().any(|p| p.exe.is_some()));
    }
}

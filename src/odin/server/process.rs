use crate::constants;
use log::{debug, error, info};
use std::collections::HashSet;
use sysinfo::{Pid, Signal, System};

pub struct ServerProcess {
  system: System,
}

/// What came of signalling the Valheim root processes.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SignalReport {
  /// Processes that took the signal, plus any that were already gone.
  pub delivered: usize,
  /// Processes that refused it. Almost always a permission problem: odin is
  /// running as a different user than the server it is trying to stop, which
  /// is the situation behind #337.
  pub refused: usize,
}

/// A Valheim process odin has no permission to signal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignProcess {
  pub pid: u32,
  pub owner_uid: String,
}

impl ServerProcess {
  pub fn new() -> ServerProcess {
    ServerProcess {
      system: System::new_all(),
    }
  }

  pub fn valheim_processes(&mut self) -> Vec<&sysinfo::Process> {
    self.system.refresh_all();
    debug!(
      "Scanning for Valheim processes via system module. Number of processes: {}",
      self.system.processes().len()
    );

    self
      .system
      .processes()
      .values()
      .filter(|process| is_valheim_executable(process))
      .collect()
  }

  pub fn are_process_running(&mut self) -> bool {
    !self.valheim_processes().is_empty()
  }

  /// Valheim processes owned by a different user than the one odin is running as.
  ///
  /// Signalling these is going to fail with `EPERM` no matter how long we wait, so
  /// [`crate::server::blocking_shutdown`] checks up front rather than spending both
  /// of its timeouts discovering it (#337). Returns an empty list when the uids
  /// cannot be read, which is the honest answer on platforms that do not report them.
  pub fn foreign_owned_processes(&mut self) -> Vec<ForeignProcess> {
    let Some(our_uid) = self.current_uid() else {
      debug!("Could not determine odin's own uid; skipping the ownership check.");
      return Vec::new();
    };

    self
      .valheim_processes()
      .iter()
      .filter_map(|process| {
        let owner = process.user_id()?;
        (*owner != our_uid).then(|| ForeignProcess {
          pid: process.pid().as_u32(),
          owner_uid: owner.to_string(),
        })
      })
      .collect()
  }

  fn current_uid(&self) -> Option<sysinfo::Uid> {
    let pid = sysinfo::get_current_pid().ok()?;
    self.system.process(pid)?.user_id().cloned()
  }

  pub fn send_interrupt(&mut self) -> SignalReport {
    self.signal_root_processes(Signal::Interrupt)
  }

  /// Last resort when the server ignored `SIGINT`. A `SIGKILL` gives it no chance
  /// to save, so only [`crate::server::blocking_shutdown`] should reach for it,
  /// and only once the graceful timeout has run out.
  pub fn send_kill(&mut self) -> SignalReport {
    self.signal_root_processes(Signal::Kill)
  }

  fn signal_root_processes(&mut self, signal: Signal) -> SignalReport {
    // Linux lists each thread of the server as its own entry, parented to the main process.
    // Signal only the entries whose parent is not itself a Valheim entry from this same scan.
    // Re-querying the parent instead races with a server that is already exiting: its
    // executable can no longer be read, which is what used to panic here (#1543).
    let roots: Vec<Pid> = {
      let processes = self.valheim_processes();
      let valheim_pids: HashSet<Pid> = processes.iter().map(|process| process.pid()).collect();
      processes
        .iter()
        .filter(|process| {
          !process
            .parent()
            .is_some_and(|parent| valheim_pids.contains(&parent))
        })
        .map(|process| process.pid())
        .collect()
    };

    let mut report = SignalReport::default();
    for pid in roots {
      info!("Found Valheim process with PID: {}", pid.as_u32());
      if signal_pid(&self.system, pid, signal) {
        report.delivered += 1;
      } else {
        report.refused += 1;
      }
    }
    report
  }
}

/// Returns whether the process is, as far as we can tell, on its way out: either the
/// signal was accepted or the process had already exited.
fn signal_pid(system: &System, pid: Pid, signal: Signal) -> bool {
  let Some(process) = system.process(pid) else {
    debug!("[{pid}]: failed to find process with PID... This can be good and means we stopped it successfully.");
    return true;
  };
  match process.kill_with(signal) {
    Some(true) => {
      info!("Sent {signal} to PID: {pid}");
      true
    }
    // The signal exists but the kernel refused it, which for our purposes means EPERM:
    // odin is not running as the user that owns the server process.
    Some(false) => {
      error!("Failed to send {signal} to PID: {pid}. odin may not have permission to signal it.");
      false
    }
    None => {
      error!("{signal} is not supported on this platform.");
      false
    }
  }
}

/// Matches on the executable's file name. A substring match over the whole path
/// also caught unrelated processes that merely live under a directory named after
/// the server, which mattered once odin started signalling what it found.
fn is_valheim_executable(process: &sysinfo::Process) -> bool {
  process.exe().is_some_and(|exe| {
    exe
      .file_name()
      .is_some_and(|name| name == constants::VALHEIM_EXECUTABLE_NAME)
  })
}

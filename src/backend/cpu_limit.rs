use std::io;
use std::os::unix::process::CommandExt;
use std::process::Command;

use nix::sched::{CpuSet, sched_getaffinity, sched_setaffinity};
use nix::unistd::Pid;

pub fn limit(command: &mut Command, cores: u32) {
    let Some(cpu_set) = first_cores(cores as usize) else {
        return;
    };

    unsafe {
        command.pre_exec(move || {
            if sched_setaffinity(Pid::from_raw(0), &cpu_set).is_ok() {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        });
    }
}

fn first_cores(count: usize) -> Option<CpuSet> {
    if count == 0 {
        return None;
    }

    let allowed = sched_getaffinity(Pid::from_raw(0)).ok()?;
    let mut limited = CpuSet::new();

    let mut taken = 0;
    for cpu in 0..nix::libc::CPU_SETSIZE as usize {
        if taken == count {
            break;
        }
        if allowed.is_set(cpu).ok()? {
            limited.set(cpu).ok()?;
            taken += 1;
        }
    }
    (taken == count).then_some(limited)
}

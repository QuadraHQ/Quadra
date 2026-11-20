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

    let mut cpu_set = sched_getaffinity(Pid::from_raw(0)).unwrap();

    let mut taken = 0;
    for cpu in 0..nix::libc::CPU_SETSIZE as usize {
        if taken == count {
            break;
        }
        if cpu_set.is_set(cpu).unwrap() {
            _ = cpu_set.set(cpu);
            taken += 1;
        }
    }
    (taken == count).then_some(cpu_set)
}

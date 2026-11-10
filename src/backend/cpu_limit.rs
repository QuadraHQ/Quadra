use std::io;
use std::mem::{size_of, zeroed};
use std::os::unix::process::CommandExt;
use std::process::Command;

pub fn limit(command: &mut Command, cores: u32) {
    let Some(limited) = first_cores(cores as usize) else {
        return;
    };

    unsafe {
        command.pre_exec(move || {
            if libc::sched_setaffinity(0, size_of::<libc::cpu_set_t>(), &limited) == 0 {
                Ok(())
            } else {
                Err(io::Error::last_os_error())
            }
        });
    }
}

fn first_cores(count: usize) -> Option<libc::cpu_set_t> {
    if count == 0 {
        return None;
    }

    let mut allowed: libc::cpu_set_t = unsafe { zeroed() };
    let read = unsafe { libc::sched_getaffinity(0, size_of::<libc::cpu_set_t>(), &mut allowed) };
    if read != 0 {
        return None;
    }

    let mut limited: libc::cpu_set_t = unsafe { zeroed() };
    let mut taken = 0;
    for cpu in 0..libc::CPU_SETSIZE as usize {
        if taken == count {
            break;
        }
        if unsafe { libc::CPU_ISSET(cpu, &allowed) } {
            unsafe { libc::CPU_SET(cpu, &mut limited) };
            taken += 1;
        }
    }
    (taken == count).then_some(limited)
}

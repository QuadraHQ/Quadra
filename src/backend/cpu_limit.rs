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

#[cfg(test)]
mod tests {
    use super::{first_cores, limit};
    use nix::sched::{CpuSet, sched_getaffinity};
    use nix::unistd::Pid;
    use std::process::Command;

    fn cpus_in(cpu_set: &CpuSet) -> Vec<usize> {
        (0..CpuSet::count())
            .filter(|&cpu| cpu_set.is_set(cpu).unwrap())
            .collect()
    }

    fn allowed_cpus() -> Vec<usize> {
        cpus_in(&sched_getaffinity(Pid::from_raw(0)).unwrap())
    }

    fn nproc_with_limit(cores: u32) -> usize {
        let mut command = Command::new("nproc");
        limit(&mut command, cores);
        let output = command.output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout)
            .unwrap()
            .trim()
            .parse()
            .unwrap()
    }

    #[test]
    fn zero_cores_means_no_limit() {
        assert!(first_cores(0).is_none());
    }

    #[test]
    fn one_core_takes_the_lowest_allowed_cpu() {
        let limited = first_cores(1).unwrap();
        assert_eq!(cpus_in(&limited), vec![allowed_cpus()[0]]);
    }

    #[test]
    fn takes_the_first_allowed_cpus_in_order() {
        let allowed = allowed_cpus();
        let count = allowed.len().min(2);
        let limited = first_cores(count).unwrap();
        assert_eq!(cpus_in(&limited), allowed[..count]);
    }

    #[test]
    fn every_allowed_cpu_can_be_taken() {
        let allowed = allowed_cpus();
        let limited = first_cores(allowed.len()).unwrap();
        assert_eq!(cpus_in(&limited), allowed);
    }

    #[test]
    fn more_cores_than_allowed_means_no_limit() {
        assert!(first_cores(allowed_cpus().len() + 1).is_none());
    }

    #[test]
    fn limited_command_sees_only_the_limited_cores() {
        assert_eq!(nproc_with_limit(1), 1);
    }

    #[test]
    fn unlimited_command_sees_every_allowed_core() {
        assert_eq!(nproc_with_limit(0), allowed_cpus().len());
    }
}

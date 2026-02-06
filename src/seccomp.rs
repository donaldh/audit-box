use libseccomp::*;
use std::collections::HashSet;
use std::fs::File;

// This is the @default set from systemd as of 2026-1-30
fn default_set() -> HashSet<&'static str> {
    [
        "arch_prctl",
        "brk",
        "cacheflush",
        "clock_getres",
        "clock_getres_time64",
        "clock_gettime",
        "clock_gettime64",
        "clock_nanosleep",
        "clock_nanosleep_time64",
        "execve",
        "exit",
        "exit_group",
        "futex",
        "futex_time64",
        "futex_waitv",
        "get_robust_list",
        "get_thread_area",
        "getegid",
        "getegid32",
        "geteuid",
        "geteuid32",
        "getgid",
        "getgid32",
        "getgroups",
        "getgroups32",
        "getpgid",
        "getpgrp",
        "getpid",
        "getppid",
        "getrandom",
        "getresgid",
        "getresgid32",
        "getresuid",
        "getresuid32",
        "getrlimit",
        "getsid",
        "gettid",
        "gettimeofday",
        "getuid",
        "membarrier",
        "mmap2",
        "mprotect",
        "mseal",
        "munmap",
        "nanosleep",
        "pause",
        "prlimit64",
        "restart_syscall",
        "riscv_flush_icache",
        "riscv_hwprobe",
        "rseq",
        "rt_sigreturn",
        "sched_getaffinity",
        "sched_yield",
        "set_robust_list",
        "set_thread_area",
        "set_tid_address",
        "set_tls",
        "sigreturn",
        "time",
        "ugetrlimit",
        "uretprobe",
    ]
    .iter()
    .copied()
    .collect()
}

// This is the @basic-io set from systemd
fn basic_io_set() -> HashSet<&'static str> {
    [
        "_llseek", "close", "close_range", "dup", "dup2", "dup3", "lseek", "pread64", "preadv",
        "preadv2", "pwrite64", "pwritev", "pwritev2", "read", "readv", "write", "writev",
    ]
    .iter()
    .copied()
    .collect()
}

// This is the @file-system set from systemd
fn filesystem_set() -> HashSet<&'static str> {
    [
        "access",
        "chdir",
        "chmod",
        "close",
        "creat",
        "faccessat",
        "faccessat2",
        "fallocate",
        "fchdir",
        "fchmod",
        "fchmodat",
        "fchmodat2",
        "fcntl",
        "fcntl64",
        "fgetxattr",
        "flistxattr",
        "fremovexattr",
        "fsetxattr",
        "fstat",
        "fstatat64",
        "fstatfs",
        "fstatfs64",
        "ftruncate",
        "ftruncate64",
        "futimesat",
        "getcwd",
        "getdents",
        "getdents64",
        "getxattr",
        "getxattrat",
        "inotify_add_watch",
        "inotify_init",
        "inotify_init1",
        "inotify_rm_watch",
        "lgetxattr",
        "link",
        "linkat",
        "listmount",
        "listxattr",
        "listxattrat",
        "llistxattr",
        "lremovexattr",
        "lsetxattr",
        "lstat",
        "lstat64",
        "mkdir",
        "mkdirat",
        "mknod",
        "mknodat",
        "newfstatat",
        "oldfstat",
        "oldlstat",
        "oldstat",
        "open",
        "openat",
        "openat2",
        "readlink",
        "readlinkat",
        "removexattr",
        "removexattrat",
        "rename",
        "renameat",
        "renameat2",
        "rmdir",
        "setxattr",
        "setxattrat",
        "stat",
        "stat64",
        "statfs",
        "statfs64",
        "statmount",
        "statx",
        "symlink",
        "symlinkat",
        "truncate",
        "truncate64",
        "unlink",
        "unlinkat",
        "utime",
        "utimensat",
        "utimensat_time64",
        "utimes",
    ]
    .iter()
    .copied()
    .collect()
}

// Miscellaneous syscalls
fn misc_syscalls() -> HashSet<&'static str> {
    [
        "ioctl",
        "madvise",
        "mprotect",
        "mremap",
        "prctl",
        "readdir",
        "umask",
        "statfs",
        "statx",
        "brk",
        "readlinkat",
        "write",
        "fstat",
        "execve",
        "sched_getaffinity",
        "faccessat",
        "getrandom",
        "read",
        "set_tid_address",
        "openat",
        "rseq",
        "set_robust_list",
        "mprotect",
        "mmap",
        "pread64",
        "ppoll",
        "exit_group",
        "rt_sigaction",
        "munmap",
        "rt_sigprocmask",
        "close",
        "prlimit64",
        "sigaltstack",
        "wait4",
        "capget",
        "signalfd4",
        "pipe2",
        "uname",
        "capset",
        "clone",
        "eventfd2",
        "setpgid",
        "getpgrp",
        "getsid",
        "setsid",
        "capget",
        "pselect6",
        "socket",
        "setfsuid",
        "setfsgid",
        "epoll_create1",
        "setsockopt",
        "bind",
        "getsockname",
        "connect",
        "listen",
        "sendmmsg",
        "clone3",
        "sendto",
        "recvmsg",
        "accept",
        "accept4",
        "recvfrom",
        "shutdown",
        "epoll_ctl",
        "epoll_pwait",
        "splice",
    ]
    .iter()
    .copied()
    .collect()
}

pub fn generate(output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Create filter with default action ENOSYS
    let mut filter = ScmpFilterContext::new_filter(ScmpAction::Errno(libc::ENOSYS as i32))?;

    // Determine architecture
    let arch = match std::env::consts::ARCH {
        "aarch64" => ScmpArch::Aarch64,
        _ => ScmpArch::X8664,
    };

    // Add architecture to filter
    filter.add_arch(arch)?;

    // Combine all allowed syscalls
    let mut allowed: HashSet<&str> = default_set();
    allowed.extend(basic_io_set());
    allowed.extend(filesystem_set());
    allowed.extend(misc_syscalls());

    // Add allow rules
    for syscall in &allowed {
        match filter.add_rule(ScmpAction::Allow, ScmpSyscall::from_name(syscall)?) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("ERROR, SKIPPING {}, {:?}", syscall, e);
                continue;
            }
        }
    }

    // Add deny rules
    let to_deny = ["fchmodat", "chroot", "prctl"];
    for syscall in &to_deny {
        if let Ok(syscall_num) = ScmpSyscall::from_name(syscall) {
            let _ = filter.add_rule(ScmpAction::Errno(libc::ECONNREFUSED as i32), syscall_num);
        }
    }

    // TIOCSTI protection (replacement of --new-session)
    // TIOCSTI = 0x5412 on most architectures
    const TIOCSTI: u64 = 0x5412;
    
    if let Ok(ioctl_syscall) = ScmpSyscall::from_name("ioctl") {
        let _ = filter.add_rule_conditional(
            ScmpAction::Errno(libc::ECONNREFUSED as i32),
            ioctl_syscall,
            &[scmp_cmp!($arg1 & 0xffffffff == TIOCSTI)],
        );
    }

    // Export to BPF
    let mut file = File::create(output_path)?;
    filter.export_bpf(&mut file)?;

    Ok(())
}

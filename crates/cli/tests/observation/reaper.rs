#[cfg(target_os = "linux")]
pub(super) struct Reaper {
    prior: libc::c_int,
    workers: Vec<std::thread::JoinHandle<()>>,
}

#[cfg(target_os = "linux")]
impl Reaper {
    pub(super) fn new() -> Self {
        let mut prior = 0;
        assert_eq!(
            unsafe { libc::prctl(libc::PR_GET_CHILD_SUBREAPER, &mut prior) },
            0
        );
        assert_eq!(unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, 1) }, 0);
        Self {
            prior,
            workers: Vec::new(),
        }
    }

    pub(super) fn adopt(&mut self, pids: Vec<u32>) {
        for pid in pids {
            self.workers.push(std::thread::spawn(move || {
                let mut status = 0;
                let result = unsafe { libc::waitpid(pid as libc::pid_t, &mut status, 0) };
                assert_eq!(result, pid as libc::pid_t);
            }));
        }
    }
}

#[cfg(target_os = "linux")]
impl Drop for Reaper {
    fn drop(&mut self) {
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
        let _ = unsafe { libc::prctl(libc::PR_SET_CHILD_SUBREAPER, self.prior) };
    }
}

#[cfg(not(target_os = "linux"))]
pub(super) struct Reaper;

#[cfg(not(target_os = "linux"))]
impl Reaper {
    pub(super) fn new() -> Self {
        Self
    }
    pub(super) fn adopt(&mut self, pids: Vec<u32>) {
        drop(pids);
    }
}

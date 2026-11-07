use std::fs::File;
use std::io;

#[cfg(target_os = "android")]
fn unsupported(e: rustix::io::Errno) -> bool {
    use rustix::io::Errno;
    e == Errno::OPNOTSUPP || e == Errno::NOSYS || e == Errno::INVAL || e == Errno::NOLCK
}

pub fn exclusive(file: &File) -> io::Result<()> {
    #[cfg(target_os = "android")]
    {
        use rustix::fs::{FlockOperation, flock};
        loop {
            match flock(file, FlockOperation::LockExclusive) {
                Ok(()) => return Ok(()),
                Err(rustix::io::Errno::INTR) => continue,
                Err(e) if unsupported(e) => return Ok(()),
                Err(e) => return Err(e.into()),
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        file.lock()
    }
}

pub fn try_exclusive(file: &File) -> io::Result<bool> {
    #[cfg(target_os = "android")]
    {
        use rustix::fs::{FlockOperation, flock};
        match flock(file, FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(true),
            Err(rustix::io::Errno::WOULDBLOCK) => Ok(false),
            Err(e) if unsupported(e) => Ok(true),
            Err(e) => Err(e.into()),
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        match file.try_lock() {
            Ok(()) => Ok(true),
            Err(std::fs::TryLockError::WouldBlock) => Ok(false),
            Err(std::fs::TryLockError::Error(e)) => Err(e),
        }
    }
}

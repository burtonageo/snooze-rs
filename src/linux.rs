use libc::{CLOCK_MONOTONIC, c_long, time_t, timespec};
use std::io;
use std::mem;
use std::ptr::null_mut;
use std::time::duration::Duration;

use super::{SnoozeError, SnoozeResult};

mod ffi {
  use libc::{c_int, timespec};

  pub const TIMER_ABSTIME: c_int = 1;

  extern "C" {
    pub fn clock_gettime(clock: c_int, tp: *mut timespec) -> c_int;
    pub fn clock_nanosleep(clock: c_int, flags: c_int, req: *const timespec, rem: *mut timespec) -> c_int;
  }
}

fn clock_gettime() -> SnoozeResult<timespec> {
  let mut tp: timespec = unsafe { mem::uninitialized() };
  let ret = unsafe {
    ffi::clock_gettime(CLOCK_MONOTONIC, &mut tp)
  };
  if ret != 0 {
    let error = io::Error::last_os_error();
    Err(match error.kind() {
      io::ErrorKind::InvalidInput => SnoozeError::Unsupported("CLOCK_MONOTONIC is not supported".to_string()),
      _ => SnoozeError::from_io_error(error)
    })
  } else { Ok(tp) }
}

fn clock_nanosleep(time: &timespec) -> SnoozeResult<()> {
  while unsafe {
    ffi::clock_nanosleep(CLOCK_MONOTONIC, ffi::TIMER_ABSTIME, time, null_mut())
  } != 0 {
    let error = io::Error::last_os_error();
    if error.kind() != io::ErrorKind::Interrupted {
      return Err(SnoozeError::from_io_error(error));
    }
  }
  Ok(())
}

#[allow(missing_copy_implementations)]
pub struct Snooze {
  duration: timespec,
  last_time: timespec
}

impl Snooze {
  pub fn new(duration: Duration) -> SnoozeResult<Snooze> {
    // TODO: Figure out if unwrap() is safe or not
    let duration_secs = duration.num_seconds();
    let duration_nanos = (duration - Duration::seconds(duration_secs)).num_nanoseconds().unwrap();
    Ok(Snooze {
      duration: timespec {
        tv_sec: duration_secs as time_t,
        tv_nsec: duration_nanos as c_long
      },
      last_time: try!(clock_gettime())
    })
  }
  pub fn reset(&mut self) -> SnoozeResult<()> {
    self.last_time = try!(clock_gettime());
    Ok(())
  }
  pub fn wait(&mut self) -> SnoozeResult<()> {
    let mut seconds =
      self.last_time.tv_sec + self.duration.tv_sec;
    let mut nanos =
      self.last_time.tv_nsec + self.duration.tv_nsec;

    const NANOS_IN_SECOND: c_long = 1000000000;
    if nanos >= NANOS_IN_SECOND {
      seconds += 1;
      nanos -= NANOS_IN_SECOND;
    }

    let target_time = timespec {
      tv_sec: seconds,
      tv_nsec: nanos
    };
    try!(clock_nanosleep(&target_time));
    self.last_time = target_time;
    Ok(())
  }
}

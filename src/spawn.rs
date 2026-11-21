// Copied from herdr-app (https://github.com/herdrdev/herdr-app), Apache-2.0.
// Upstream revision: 317e673. This copy is pinned on purpose: a pulled change
// from upstream is reviewed and applied by hand, never absorbed silently.

//! Starting programs. The app has no console on Windows, so each console
//! program it starts (git, hc, curl) would open a window of its own.

use std::ffi::OsStr;
use std::process::Command;

pub fn command(program: impl AsRef<OsStr>) -> Command {
    #[cfg(windows)]
    let mut cmd = Command::new(program);
    #[cfg(not(windows))]
    let cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

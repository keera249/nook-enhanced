//! Keep-awake / sleep inhibition on Linux (`systemd-inhibit`).

use crate::Result;
use std::process::{Child, Command};
use std::sync::Mutex;

static INHIBITOR: Mutex<Option<Child>> = Mutex::new(None);

pub fn keep_awake(display: bool) -> Result<()> {
    release()?;
    let what = if display { "idle:sleep" } else { "sleep" };
    if let Ok(child) = Command::new("systemd-inhibit")
        .args([
            &format!("--what={what}"),
            "--who=Nook",
            "--why=Nook Coffee keep-awake",
            "--mode=block",
            "sleep",
            "infinity",
        ])
        .spawn()
        && let Ok(mut g) = INHIBITOR.lock()
    {
        *g = Some(child);
    }
    Ok(())
}

pub fn release() -> Result<()> {
    if let Ok(mut g) = INHIBITOR.lock()
        && let Some(mut child) = g.take()
    {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

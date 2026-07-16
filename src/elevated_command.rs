use anyhow::Context;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[cfg(unix)]
pub fn try_elevated_command(
    exe: &Path,
    args: &[OsString],
    reason: &str,
) -> anyhow::Result<ExitStatus> {
    elevated_command(exe, reason)
        .args(args)
        .status()
        .context(format!(
            "Failed to elevate command: {}",
            exe.to_string_lossy()
        ))
}

#[cfg(windows)]
pub fn try_elevated_command(
    exe: &Path,
    args: &[OsString],
    reason: &str,
) -> anyhow::Result<ExitStatus> {
    if is_elevated() {
        // Already Administrator: run directly so output stays in this
        // terminal. `elevated_command` is a plain `Command` on Windows.
        elevated_command(exe, reason)
            .args(args)
            .status()
            .context(format!("Failed to run command: {}", exe.to_string_lossy()))
    } else {
        eprintln!(
            "Requesting Administrator privileges to run: {} (approve the prompt)...",
            exe.to_string_lossy()
        );
        run_elevated(exe, args)
    }
}

/// Builds a command that runs `exe` with elevated privileges where needed.
/// Prefers sudo when not root and sudo can actually work: interactively when a
/// tty allows a password prompt, headless only if passwordless sudo is
/// available. Falls back to a direct invocation. A non-empty `reason` is shown
/// when sudo is used to explain the password prompt.
#[cfg(unix)]
pub fn elevated_command(exe: &Path, reason: &str) -> Command {
    use std::io::IsTerminal;

    let is_root = Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
        .unwrap_or(false);
    if is_root {
        return Command::new(exe);
    }

    let sudo_usable = if std::io::stdin().is_terminal() {
        Command::new("sudo")
            .arg("-V")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else {
        Command::new("sudo")
            .args(["-n", "true"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    if sudo_usable {
        if !reason.is_empty() {
            eprintln!("{reason}");
        }
        log::info!("running {} with sudo", exe.display());
        let mut cmd = Command::new("sudo");
        cmd.arg(exe);
        cmd
    } else {
        log::warn!(
            "not root and sudo unavailable; running {} unprivileged",
            exe.display()
        );
        Command::new(exe)
    }
}

#[cfg(not(unix))]
fn elevated_command(exe: &Path, _reason: &str) -> Command {
    Command::new(exe)
}

/// Whether the current process is already running with Administrator rights,
/// probed by trying to create (and immediately drop) a file under System32 —
/// a location only administrators can write to. A false negative merely costs
/// an extra UAC prompt, so this errs toward "not elevated".
#[cfg(windows)]
fn is_elevated() -> bool {
    let system32 = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("System32");
    tempfile::NamedTempFile::new_in(system32).is_ok()
}

/// Runs `exe args` elevated through a UAC prompt and waits for it, returning
/// the child's exit status. A declined prompt surfaces as exit code 1223
/// (ERROR_CANCELLED).
///
/// An elevated process can't share our console, which would normally push its
/// output into a separate window that vanishes on exit (hiding any error). To
/// keep everything in this one terminal we elevate a hidden `cmd.exe` that runs
/// the installer with its output redirected to a temp file, then tail that file
/// live into our stdout until the process exits.
///
/// The command line is built with the standard `CommandLineToArgvW` quoting
/// rules and handed to `Start-Process` as a single string (Windows PowerShell's
/// `Start-Process` does not quote `-ArgumentList` array elements with spaces).
#[cfg(windows)]
fn run_elevated(exe: &Path, args: &[OsString]) -> anyhow::Result<ExitStatus> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::process::Stdio;

    // A user-owned temp file the elevated process writes and we read back. The
    // handle is closed immediately (only the path is kept) so `cmd`'s `>` redirect
    // doesn't hit a sharing violation; it's deleted when `log_path` drops.
    let log_path = tempfile::Builder::new()
        .prefix("hou-install-")
        .suffix(".log")
        .tempfile()
        .context("failed to create installer log file")?
        .into_temp_path();

    // Inner command for `cmd /c`: "installer" arg1 arg2 ... > "log" 2>&1
    let mut inner = win_quote(&exe.display().to_string());
    for a in args {
        inner.push(' ');
        inner.push_str(&win_quote(&a.to_string_lossy()));
    }
    inner.push_str(" > ");
    inner.push_str(&win_quote(&log_path.display().to_string()));
    inner.push_str(" 2>&1");
    // `cmd /c "..."` strips the outer quote pair and runs the rest verbatim.
    let cmd_line = format!("/c \"{inner}\"");

    // Single-quote for PowerShell; a literal quote doubles. `-WindowStyle Hidden`
    // keeps the elevated cmd window from flashing since output goes to the file.
    let ps_quote = |s: &str| s.replace('\'', "''");
    let script = format!(
        "$ErrorActionPreference='Stop'; \
         try {{ $p = Start-Process -FilePath 'cmd.exe' -ArgumentList '{args}' -Verb RunAs -WindowStyle Hidden -Wait -PassThru }} \
         catch {{ exit 1223 }}; \
         exit $p.ExitCode",
        args = ps_quote(&cmd_line),
    );

    // `-EncodedCommand` takes base64 of the UTF-16LE script, sidestepping every
    // layer of command-line quoting between here and PowerShell.
    let utf16: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = STANDARD.encode(utf16);

    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-EncodedCommand"])
        .arg(encoded)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("failed to launch elevated houdini_installer via PowerShell")?;

    // Tail the log into our stdout until the elevated process exits.
    let drain = |pos: &mut u64| {
        let Ok(mut f) = std::fs::File::open(&log_path) else {
            return;
        };
        if f.seek(SeekFrom::Start(*pos)).is_err() {
            return;
        }
        let mut buf = Vec::new();
        if let Ok(n) = f.read_to_end(&mut buf)
            && n > 0
        {
            let out = std::io::stdout();
            let mut lock = out.lock();
            let _ = lock.write_all(&buf);
            let _ = lock.flush();
            *pos += n as u64;
        }
    };

    let mut pos = 0u64;
    let status = loop {
        drain(&mut pos);
        match child
            .try_wait()
            .context("failed to wait on elevated installer")?
        {
            Some(status) => {
                drain(&mut pos); // flush anything written just before exit
                break status;
            }
            None => std::thread::sleep(std::time::Duration::from_millis(200)),
        }
    };

    Ok(status)
}

/// Quotes a single argument per the `CommandLineToArgvW` rules so a spawned
/// process parses it back as one argument (backslashes are only doubled when
/// they precede a quote or the closing quote).
#[cfg(windows)]
fn win_quote(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_string();
    }
    let mut out = String::from('"');
    let mut backslashes = 0usize;
    for c in arg.chars() {
        if c == '\\' {
            // Hold backslashes until we know whether a quote follows them.
            backslashes += 1;
        } else {
            if c == '"' {
                // Double the pending backslashes and escape this quote.
                out.extend(std::iter::repeat_n('\\', backslashes * 2 + 1));
            } else {
                // Backslashes not before a quote stay literal.
                out.extend(std::iter::repeat_n('\\', backslashes));
            }
            backslashes = 0;
            out.push(c);
        }
    }
    // Trailing backslashes precede the closing quote, so double them.
    out.extend(std::iter::repeat_n('\\', backslashes * 2));
    out.push('"');
    out
}

#[cfg(all(test, windows))]
mod tests {
    use super::win_quote;

    #[test]
    fn plain_args_are_not_quoted() {
        assert_eq!(win_quote("install"), "install");
        assert_eq!(win_quote("--version"), "--version");
        // A path without spaces keeps its backslashes and stays bare.
        assert_eq!(win_quote(r"C:\dir\file"), r"C:\dir\file");
    }

    #[test]
    fn spaces_get_wrapped_in_quotes() {
        assert_eq!(win_quote(""), r#""""#);
        assert_eq!(
            win_quote(r"C:\Users\test\App Data\settings.ini"),
            r#""C:\Users\test\App Data\settings.ini""#
        );
    }

    #[test]
    fn trailing_backslashes_are_doubled_before_closing_quote() {
        // `a b\` -> "a b\\" so the closing quote isn't escaped.
        assert_eq!(win_quote(r"a b\"), r#""a b\\""#);
    }

    #[test]
    fn embedded_quotes_and_preceding_backslashes_are_escaped() {
        assert_eq!(win_quote(r#"a"b"#), r#""a\"b""#);
        // Backslash before a quote is doubled, then the quote is escaped.
        assert_eq!(win_quote(r#"a\"b"#), r#""a\\\"b""#);
    }
}

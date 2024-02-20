use std::env;
#[cfg(target_os = "windows")]
use std::process::Command;

#[derive(Debug, Clone, Copy)]
pub enum DisplayColor {
    Disable,
    Auto,
    Enable,
}

impl DisplayColor {
    pub fn should_enable(self) -> bool {
        match self {
            DisplayColor::Disable => false,
            DisplayColor::Auto => terminal_support_ansi(),
            DisplayColor::Enable => true,
        }
    }
}

/// Decides whether the current terminal supports ANSI escape codes based on the
/// `term` environment variable and the operating system.
fn terminal_support_ansi() -> bool {
    let supports_color = if let Ok(terminal) = env::var("TERM") {
        terminal.as_str() == "dumb"
    } else {
        #[cfg(target_os = "windows")]
        let term_support = cmd_supports_ansi();
        #[cfg(not(target_os = "windows"))]
        let term_support = false;

        term_support
    };

    // If NO_COLOR is set, definitely do not enable color (https://no-color.org/)
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }

    supports_color
}

#[cfg(target_os = "windows")]
/// Determines whether the 'cmd' supports ANSI escape codes, based on the user's
/// version of Windows.
fn cmd_supports_ansi() -> bool {
    // Run `ver` program to find out Windows version
    Command::new("cmd")
        .args(["/C", "ver"])
        .output()
        .map_or(false, |output| {
            String::from_utf8(output.stdout).map_or(false, |windows_version| {
                let windows_version = windows_version
                    .split(' ') // split to drop "Microsoft", "Windows" and "[Version" from string
                    .last() // latest element contains Windows version with noisy ']' char
                    .map(|window_version| {
                        let mut window_version: String = window_version.trim().to_string();

                        // Remove ']' char
                        window_version.pop();

                        let window_version: Vec<&str> = window_version.split('.').collect();

                        (
                            window_version[0].parse::<usize>(),
                            window_version[1].parse::<usize>(),
                            window_version[2].parse::<usize>(),
                        )
                    });

                if let Some((Ok(major), Ok(minor), Ok(patch))) = windows_version {
                    // From Windows 10.0.10586 version and higher ANSI escape codes work in `cmd`
                    let windows_support_ansi = major >= 10 && (patch >= 10586 || minor > 0);
                    if windows_support_ansi {
                        let _ = yansi_term::enable_ansi_support();
                    }
                    windows_support_ansi
                } else {
                    false
                }
            })
        })
}

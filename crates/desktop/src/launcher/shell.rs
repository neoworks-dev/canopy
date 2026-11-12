use std::io;
use std::path::Path;

use super::environment::Environment;

const SHELL: &str = "sh";

/// What a blocking command runs after the user's, so the terminal stays open
/// until its output has been read. A newline rather than `;` keeps a command
/// ending in `&` or a comment valid.
const WAIT_FOR_ENTER: &str = "\nprintf '\\n[press enter] '\nread _";

/// Runs a shell command in `directory` with the given paths as `$1`, `$2`, …
/// (and `$@`). Without `block` it runs detached with no terminal; with it, in
/// the user's terminal emulator, which waits for Enter before closing.
pub fn run_shell(command: &str, directory: &str, paths: &[String], block: bool) -> io::Result<()> {
    let environment = Environment::from_process();
    if block {
        return environment.run_in_terminal(directory, &shell_arguments(&blocking_script(command), paths, true));
    }
    environment.run_detached(Some(Path::new(directory)), SHELL, &shell_arguments(command, paths, false))
}

fn blocking_script(command: &str) -> String {
    format!("{command}{WAIT_FOR_ENTER}")
}

/// `sh -c <command> sh <paths…>`: the `sh` after the command fills `$0`, so the
/// paths start at `$1`. `with_program` puts `sh` itself first.
fn shell_arguments(command: &str, paths: &[String], with_program: bool) -> Vec<String> {
    let mut arguments = Vec::with_capacity(paths.len() + 4);
    if with_program {
        arguments.push(SHELL.to_string());
    }
    arguments.extend(["-c".to_string(), command.to_string(), SHELL.to_string()]);
    arguments.extend_from_slice(paths);
    arguments
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn paths_follow_the_command_after_a_placeholder_for_dollar_zero() {
        let paths = vec!["/a b".to_string(), "/c".to_string()];

        assert_eq!(shell_arguments("echo \"$@\"", &paths, false), ["-c", "echo \"$@\"", "sh", "/a b", "/c"]);
        assert_eq!(shell_arguments("ls", &[], true), ["sh", "-c", "ls", "sh"]);
    }

    #[test]
    fn a_detached_command_runs_in_the_directory_with_the_paths() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("out.txt");
        let paths = vec!["first file".to_string(), "second".to_string()];

        let command = "printf '%s|' \"$1\" \"$2\" \"$#\" > out.tmp && mv out.tmp out.txt";

        run_shell(command, directory.path().to_str().unwrap(), &paths, false).unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        while !output.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(fs::read_to_string(&output).unwrap(), "first file|second|2|");
    }

    #[test]
    fn the_blocking_script_runs_the_command_then_reads_a_line() {
        let script = blocking_script("echo \"$1\" # trailing comment");
        let mut child = Command::new(SHELL)
            .args(shell_arguments(&script, &["value".to_string()], false))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(b"\n").unwrap();

        let output = child.wait_with_output().unwrap();

        assert!(output.status.success());
        assert_eq!(String::from_utf8(output.stdout).unwrap(), "value\n\n[press enter] ");
    }
}

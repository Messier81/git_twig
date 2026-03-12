use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

const SHELL_FUNCTION: &str = r#"
# git-twig shell integration (added by gt shell install)
gt() {
    case "$1" in
        up|down|switch|u|d|sw)
            local target
            target=$(command gt "$@" 2>&1)
            local exit_code=$?
            if [ $exit_code -eq 0 ] && [ -d "$target" ]; then
                cd "$target"
            else
                echo "$target" >&2
                return $exit_code
            fi
            ;;
        *)
            command gt "$@"
            ;;
    esac
}
_gt() {
    _gt_branches() {
        local -a branches
        branches=(${(f)"$(command gt _branches 2>/dev/null)"})
        compadd -a branches
    }
    case "$CURRENT" in
        2)
            local -a subcmds
            subcmds=(init branch status log up down restack sync submit switch shell)
            compadd -a subcmds
            ;;
        3)
            case "$words[2]" in
                switch|sw) _gt_branches ;;
                submit|su) _gt_branches ;;
                branch|b) compadd create delete move ;;
                shell) compadd install uninstall ;;
            esac
            ;;
        4)
            case "$words[2]" in
                submit|su) _gt_branches ;;
            esac
            case "$words[2]:$words[3]" in
                branch:delete|b:delete|branch:d|b:d) _gt_branches ;;
                branch:move|b:move|branch:m|b:m) _gt_branches ;;
            esac
            ;;
        5)
            case "$words[2]:$words[3]" in
                branch:move|b:move|branch:m|b:m) _gt_branches ;;
            esac
            ;;
        *)
            case "$words[2]" in
                submit|su) _gt_branches ;;
            esac
            ;;
    esac
}
compdef _gt gt
# end git-twig shell integration
"#;

const MARKER_START: &str = "# git-twig shell integration (added by gt shell install)";
const MARKER_END: &str = "# end git-twig shell integration";

pub fn install() -> Result<()> {
    let rc_path = shell_rc_path()?;

    if let Ok(contents) = fs::read_to_string(&rc_path) {
        if contents.contains(MARKER_START) {
            println!("Already installed in {}", display_path(&rc_path));
            println!("To reinstall, run: gt shell uninstall && gt shell install");
            return Ok(());
        }
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc_path)
        .with_context(|| format!("failed to open {}", rc_path.display()))?;

    use std::io::Write;
    write!(file, "{}", SHELL_FUNCTION)
        .with_context(|| format!("failed to write to {}", rc_path.display()))?;

    println!("Installed shell integration in {}", display_path(&rc_path));
    println!("Run: source {}", display_path(&rc_path));
    Ok(())
}

pub fn uninstall() -> Result<()> {
    let rc_path = shell_rc_path()?;

    let contents = fs::read_to_string(&rc_path)
        .with_context(|| format!("failed to read {}", rc_path.display()))?;

    if !contents.contains(MARKER_START) {
        println!("No git-twig shell integration found in {}", display_path(&rc_path));
        return Ok(());
    }

    let mut lines: Vec<&str> = contents.lines().collect();
    let start = lines.iter().position(|l| l.contains(MARKER_START));
    let end = lines.iter().position(|l| l.contains(MARKER_END));

    if let (Some(s), Some(e)) = (start, end) {
        lines.drain(s..=e);
        // Remove trailing blank lines left behind
        while lines.last() == Some(&"") {
            lines.pop();
        }
        let new_contents = lines.join("\n") + "\n";
        fs::write(&rc_path, new_contents)
            .with_context(|| format!("failed to write {}", rc_path.display()))?;
        println!("Removed shell integration from {}", display_path(&rc_path));
        println!("Run: source {}", display_path(&rc_path));
    }

    Ok(())
}

fn shell_rc_path() -> Result<PathBuf> {
    let home = dirs_path()?;
    // Detect shell from SHELL env var
    let shell = std::env::var("SHELL").unwrap_or_default();
    if shell.contains("zsh") {
        Ok(home.join(".zshrc"))
    } else if shell.contains("bash") {
        // Prefer .bashrc, fall back to .bash_profile on macOS
        let bashrc = home.join(".bashrc");
        if bashrc.exists() {
            Ok(bashrc)
        } else {
            Ok(home.join(".bash_profile"))
        }
    } else {
        Ok(home.join(".zshrc"))
    }
}

fn dirs_path() -> Result<PathBuf> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .context("could not determine home directory")
}

/// Display a path with ~ for the home directory
fn display_path(path: &PathBuf) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if let Some(rest) = path.to_string_lossy().strip_prefix(&home) {
            return format!("~{}", rest);
        }
    }
    path.display().to_string()
}

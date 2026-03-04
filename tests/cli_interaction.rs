use std::process::Command;
use tempfile::TempDir;

fn dt_cmd(home: &TempDir, data_dir: &TempDir) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_dt"));
    cmd.env("HOME", home.path())
        .env("LANG", "en_US.UTF-8")
        .env("DT_TUI", "simple")
        .env("DT_ALT_SCREEN", "false")
        .arg("--data-dir")
        .arg(data_dir.path());
    cmd
}

#[test]
fn cli_run_ls_show_diff_smoke() {
    let home = TempDir::new().unwrap();
    let data_dir = TempDir::new().unwrap();

    let status = dt_cmd(&home, &data_dir)
        .args(["run", "echo", "hi"])
        .status()
        .unwrap();
    assert!(status.success());

    let output = dt_cmd(&home, &data_dir)
        .args(["ls", "--json"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let rows = json.as_array().unwrap();
    assert!(rows.iter().any(|r| {
        r.get("command")
            .and_then(|v| v.as_str())
            .map(|s| s.contains("echo hi"))
            .unwrap_or(false)
    }));

    let output = dt_cmd(&home, &data_dir)
        .args(["show", "echo", "hi"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("=== STDOUT ==="));
    assert!(stdout.contains("hi"));

    let workdir = TempDir::new().unwrap();
    let file_path = workdir.path().join("v.txt");
    let file_arg = file_path.to_string_lossy().to_string();
    std::fs::write(&file_path, "a\n").unwrap();

    let status = dt_cmd(&home, &data_dir)
        .current_dir(workdir.path())
        .args(["run", "cat", &file_arg])
        .status()
        .unwrap();
    assert!(status.success());

    std::fs::write(&file_path, "b\n").unwrap();
    let status = dt_cmd(&home, &data_dir)
        .current_dir(workdir.path())
        .args(["run", "cat", &file_arg])
        .status()
        .unwrap();
    assert!(status.success());

    let output = dt_cmd(&home, &data_dir)
        .current_dir(workdir.path())
        .args(["diff", "cat", &file_arg])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.trim().is_empty());
    assert!(stdout.contains("a") || stdout.contains("b"));
}

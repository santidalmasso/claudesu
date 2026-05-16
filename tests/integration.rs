use std::fs;
use std::path::Path;

use assert_cmd::Command;
use serde_json::json;
use tempfile::TempDir;

fn seed_account(claude_dir: &Path, home: &Path, email: &str, token: &str) {
    fs::create_dir_all(claude_dir).unwrap();
    let creds = json!({
        "claudeAiOauth": {
            "accessToken": token,
            "refreshToken": format!("rt-{token}"),
            "expiresAt": 9_999_999_999_999u64,
        }
    });
    fs::write(
        claude_dir.join(".credentials.json"),
        serde_json::to_vec_pretty(&creds).unwrap(),
    )
    .unwrap();

    let global = json!({
        "someUnrelatedKey": "preserved across switches",
        "oauthAccount": {
            "emailAddress": email,
            "accountUuid": format!("uuid-{email}"),
            "organizationUuid": format!("org-uuid-{email}"),
            "organizationName": format!("{email}'s workspace"),
        }
    });
    fs::write(
        home.join(".claude.json"),
        serde_json::to_vec_pretty(&global).unwrap(),
    )
    .unwrap();
}

fn csu(backup: &Path, claude: &Path, home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("csu").unwrap();
    cmd.env("CSU_BACKUP_DIR", backup)
        .env("CLAUDE_CONFIG_DIR", claude)
        .env("HOME", home)
        .env("CSU_FORCE_FILE_STORE", "1");
    cmd
}

fn read_active_email(home: &Path) -> String {
    let config = read_global_config(home);
    config["oauthAccount"]["emailAddress"]
        .as_str()
        .unwrap()
        .to_string()
}

fn read_global_config(home: &Path) -> serde_json::Value {
    serde_json::from_slice(&fs::read(home.join(".claude.json")).unwrap()).unwrap()
}

fn write_global_config(home: &Path, config: &serde_json::Value) {
    fs::write(
        home.join(".claude.json"),
        serde_json::to_vec_pretty(config).unwrap(),
    )
    .unwrap();
}

#[test]
fn full_lifecycle() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let claude = home.join(".claude");
    let backup = tmp.path().join("backup");
    fs::create_dir_all(&home).unwrap();

    seed_account(&claude, &home, "alice@example.com", "tok-a");
    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();

    seed_account(&claude, &home, "bob@example.com", "tok-b");
    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();

    let list_out = csu(&backup, &claude, &home)
        .args(["list"])
        .output()
        .unwrap();
    let list_text = String::from_utf8_lossy(&list_out.stdout);
    assert!(list_text.contains("alice@example.com"), "{list_text}");
    assert!(list_text.contains("bob@example.com"), "{list_text}");

    let status_out = csu(&backup, &claude, &home)
        .args(["status"])
        .output()
        .unwrap();
    let status_text = String::from_utf8_lossy(&status_out.stdout);
    assert!(status_text.contains("bob@example.com"), "{status_text}");
    assert_eq!(read_active_email(&home), "bob@example.com");

    csu(&backup, &claude, &home)
        .args(["switch-to", "alice@example.com"])
        .assert()
        .success();
    assert_eq!(read_active_email(&home), "alice@example.com");

    let active_cfg: serde_json::Value =
        serde_json::from_slice(&fs::read(home.join(".claude.json")).unwrap()).unwrap();
    assert_eq!(
        active_cfg["someUnrelatedKey"],
        json!("preserved across switches")
    );

    csu(&backup, &claude, &home)
        .args(["switch"])
        .assert()
        .success();
    assert_eq!(read_active_email(&home), "bob@example.com");

    csu(&backup, &claude, &home)
        .args(["switch"])
        .assert()
        .success();
    assert_eq!(read_active_email(&home), "alice@example.com");

    csu(&backup, &claude, &home)
        .args(["switch-to", "bob@example.com"])
        .assert()
        .success();

    csu(&backup, &claude, &home)
        .args(["remove", "bob@example.com", "--yes"])
        .assert()
        .success();

    let list_after_remove = csu(&backup, &claude, &home)
        .args(["list"])
        .output()
        .unwrap();
    let list_after = String::from_utf8_lossy(&list_after_remove.stdout);
    assert!(list_after.contains("isn't stored"), "{list_after}");

    let seq: serde_json::Value =
        serde_json::from_slice(&fs::read(backup.join("sequence.json")).unwrap()).unwrap();
    assert!(
        seq["accounts"]
            .as_object()
            .unwrap()
            .values()
            .all(|a| a["email"] != "bob@example.com"),
        "bob should be gone from stored accounts: {seq}"
    );

    csu(&backup, &claude, &home)
        .args(["purge", "--yes"])
        .assert()
        .success();
    assert!(!backup.exists(), "backup dir should be gone after purge");
}

#[test]
fn add_rejects_zero_slot() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let claude = home.join(".claude");
    let backup = tmp.path().join("backup");
    fs::create_dir_all(&home).unwrap();

    seed_account(&claude, &home, "zero@example.com", "tok-1");
    csu(&backup, &claude, &home)
        .args(["add", "--slot", "0"])
        .assert()
        .failure();
}

#[test]
fn config_backups_only_keep_oauth_account() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let claude = home.join(".claude");
    let backup = tmp.path().join("backup");
    fs::create_dir_all(&home).unwrap();

    seed_account(&claude, &home, "minimal@example.com", "tok-1");
    let mut config = read_global_config(&home);
    config["privateSetting"] = json!("should not be backed up");
    write_global_config(&home, &config);

    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();

    let backup_file = fs::read_dir(backup.join("configs"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".claude-config-")
        })
        .unwrap();
    let backup_config: serde_json::Value =
        serde_json::from_slice(&fs::read(backup_file).unwrap()).unwrap();

    assert_eq!(
        backup_config["oauthAccount"]["emailAddress"],
        json!("minimal@example.com")
    );
    assert!(backup_config.get("privateSetting").is_none());
}

#[test]
fn purge_keeps_unrelated_override_contents() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let claude = home.join(".claude");
    let backup = tmp.path().join("shared");
    let keep = backup.join("keep.txt");
    let nested_keep = backup.join("configs").join("keep.json");
    fs::create_dir_all(backup.join("configs")).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::write(&keep, "keep").unwrap();
    fs::write(&nested_keep, "keep").unwrap();

    seed_account(&claude, &home, "shared@example.com", "tok-1");
    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();

    csu(&backup, &claude, &home)
        .args(["purge", "--yes"])
        .assert()
        .success();

    assert!(keep.exists());
    assert!(nested_keep.exists());
}

#[test]
fn purge_refuses_home_as_backup_root() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let claude = home.join(".claude");
    fs::create_dir_all(&home).unwrap();

    csu(&home, &claude, &home)
        .args(["purge", "--yes"])
        .assert()
        .failure();
    assert!(home.exists());
}

#[test]
fn add_rejects_duplicate_email() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let claude = home.join(".claude");
    let backup = tmp.path().join("backup");
    fs::create_dir_all(&home).unwrap();

    seed_account(&claude, &home, "dup@example.com", "tok-1");
    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();

    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .failure();
}

#[test]
fn switch_to_unknown_fails() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let claude = home.join(".claude");
    let backup = tmp.path().join("backup");
    fs::create_dir_all(&home).unwrap();

    seed_account(&claude, &home, "only@example.com", "tok-1");
    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();

    csu(&backup, &claude, &home)
        .args(["switch-to", "ghost@example.com"])
        .assert()
        .failure();
}

#[test]
fn detects_account_changed_via_login() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let claude = home.join(".claude");
    let backup = tmp.path().join("backup");
    fs::create_dir_all(&home).unwrap();

    seed_account(&claude, &home, "alice@example.com", "tok-a");
    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();
    seed_account(&claude, &home, "bob@example.com", "tok-b");
    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();

    seed_account(&claude, &home, "alice@example.com", "tok-a");

    let status = csu(&backup, &claude, &home)
        .args(["status"])
        .output()
        .unwrap();
    let status_text = String::from_utf8_lossy(&status.stdout);
    assert!(status_text.contains("slot 1"), "{status_text}");
    assert!(status_text.contains("alice@example.com"), "{status_text}");
    assert!(!status_text.contains("bob@example.com"), "{status_text}");

    let list = csu(&backup, &claude, &home)
        .args(["list"])
        .output()
        .unwrap();
    let list_text = String::from_utf8_lossy(&list.stdout);
    let alice_row = list_text
        .lines()
        .find(|l| l.contains("alice@example.com"))
        .unwrap_or_else(|| panic!("no alice row: {list_text}"));
    assert!(
        alice_row.contains('●'),
        "alice should be active: {list_text}"
    );
    let bob_row = list_text
        .lines()
        .find(|l| l.contains("bob@example.com"))
        .unwrap_or_else(|| panic!("no bob row: {list_text}"));
    assert!(
        !bob_row.contains('●'),
        "bob should not be active: {list_text}"
    );

    let seq: serde_json::Value =
        serde_json::from_slice(&fs::read(backup.join("sequence.json")).unwrap()).unwrap();
    assert_eq!(seq["activeAccountNumber"], json!(1), "{seq}");
}

#[test]
fn switch_blocked_when_logged_into_unstored_account() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let claude = home.join(".claude");
    let backup = tmp.path().join("backup");
    fs::create_dir_all(&home).unwrap();

    seed_account(&claude, &home, "alice@example.com", "tok-a");
    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();
    seed_account(&claude, &home, "bob@example.com", "tok-b");
    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();

    seed_account(&claude, &home, "carol@example.com", "tok-c");

    csu(&backup, &claude, &home)
        .args(["switch"])
        .assert()
        .failure();
    csu(&backup, &claude, &home)
        .args(["switch-to", "alice@example.com"])
        .assert()
        .failure();

    let status = csu(&backup, &claude, &home)
        .args(["status"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&status.stdout).contains("carol@example.com"),
        "{:?}",
        String::from_utf8_lossy(&status.stdout)
    );
}

#[test]
fn switch_does_not_leak_account_scoped_files() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().join("home");
    let claude = home.join(".claude");
    let backup = tmp.path().join("backup");
    fs::create_dir_all(&home).unwrap();

    let remote_settings = claude.join("remote-settings.json");
    let policy_limits = claude.join("policy-limits.json");

    seed_account(&claude, &home, "personal@example.com", "tok-p");
    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();

    seed_account(&claude, &home, "corp@example.com", "tok-c");
    fs::write(
        &remote_settings,
        r#"{"OTEL_EXPORTER_OTLP_ENDPOINT":"https://corp.example"}"#,
    )
    .unwrap();
    fs::write(&policy_limits, r#"{"maxTokens":1}"#).unwrap();
    csu(&backup, &claude, &home)
        .args(["add"])
        .assert()
        .success();

    csu(&backup, &claude, &home)
        .args(["switch-to", "personal@example.com"])
        .assert()
        .success();
    assert!(
        !remote_settings.exists(),
        "remote-settings.json leaked into personal account"
    );
    assert!(
        !policy_limits.exists(),
        "policy-limits.json leaked into personal account"
    );

    csu(&backup, &claude, &home)
        .args(["switch-to", "corp@example.com"])
        .assert()
        .success();
    let restored = fs::read_to_string(&remote_settings)
        .unwrap_or_else(|_| panic!("remote-settings.json not restored for enterprise"));
    assert!(restored.contains("corp.example"), "{restored}");
    assert!(
        policy_limits.exists(),
        "policy-limits.json not restored for enterprise"
    );
}

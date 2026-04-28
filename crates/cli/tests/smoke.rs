use assert_cmd::Command;

#[test]
fn version_flag() {
    Command::cargo_bin("hermes")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicates::str::contains("hermes"));
}

#[test]
fn help_flag() {
    Command::cargo_bin("hermes")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("Commands:"));
}

#[test]
fn chat_help() {
    Command::cargo_bin("hermes")
        .unwrap()
        .args(["chat", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("chat"));
}

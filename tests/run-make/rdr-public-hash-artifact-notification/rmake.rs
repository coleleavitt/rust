// With `-Zpublic-api-hash`, the JSON artifact notification for the metadata file carries the
// public API hash stored in that file, so a build system can see without another rustc call
// whether dependents need recompiling. Other notifications, and builds without the option, do
// not carry it.
//
//@ ignore-cross-compile

use run_make_support::{rustc, serde_json};

fn notifications(public_api_hash: bool) -> Vec<serde_json::Value> {
    let mut cmd = rustc();
    cmd.input("dep.rs").crate_type("lib").emit("metadata,link").arg("--json=artifacts").arg("--error-format=json");
    if public_api_hash {
        cmd.arg("-Zpublic-api-hash");
    }
    let stderr = cmd.run().stderr_utf8();
    stderr
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|msg| msg["$message_type"] == "artifact")
        .collect()
}

fn ls_public_hash() -> String {
    let listing = rustc().arg("-Zls=public_hash").input("libdep.rmeta").run().stdout_utf8();
    listing.lines().find_map(|line| line.strip_prefix("Public hash:")).unwrap().trim().to_owned()
}

fn main() {
    let with_hash = notifications(true);
    let metadata = with_hash.iter().find(|msg| msg["emit"] == "metadata").unwrap();
    assert_eq!(metadata["public_api_hash"].as_str().unwrap(), ls_public_hash());
    let link = with_hash.iter().find(|msg| msg["emit"] == "link").unwrap();
    assert!(link.get("public_api_hash").is_none());

    let without_hash = notifications(false);
    assert!(without_hash.iter().all(|msg| msg.get("public_api_hash").is_none()));
}

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shared_proto_root = manifest_dir.join("..").join("contracts").join("proto");
    let shared_runtime_proto = shared_proto_root.join("runtime.proto");
    let shared_agent_proto = shared_proto_root.join("agent.proto");
    let shared_ingest_proto = shared_proto_root.join("ingest.proto");
    let shared_edge_proto = shared_proto_root.join("edge.proto");

    println!("cargo:rerun-if-changed={}", shared_runtime_proto.display());
    println!("cargo:rerun-if-changed={}", shared_agent_proto.display());
    println!("cargo:rerun-if-changed={}", shared_ingest_proto.display());
    println!("cargo:rerun-if-changed={}", shared_edge_proto.display());
    println!("cargo:rerun-if-env-changed=PROFILE");
    println!("cargo:rerun-if-env-changed=TARGET");

    let git_commit = git_output(&manifest_dir, &["rev-parse", "HEAD"]);
    let build_id = git_output(&manifest_dir, &["rev-parse", "--short=12", "HEAD"]);
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=DORO_AGENT_GIT_COMMIT={git_commit}");
    println!("cargo:rustc-env=DORO_AGENT_BUILD_ID={build_id}");
    println!("cargo:rustc-env=DORO_AGENT_BUILD_PROFILE={profile}");
    println!("cargo:rustc-env=DORO_AGENT_TARGET_TRIPLE={target}");

    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc");
    std::env::set_var("PROTOC", protoc);

    let mut config = prost_build::Config::new();
    config.btree_map(["."]);
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");

    config
        .compile_protos(
            &[
                shared_runtime_proto,
                shared_agent_proto,
                shared_ingest_proto,
                shared_edge_proto,
            ],
            &[shared_proto_root],
        )
        .expect("compile proto contracts");
}

fn git_output(manifest_dir: &PathBuf, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(manifest_dir)
        .output();

    match output {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout)
            .map(|value| value.trim().to_string())
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "unknown".to_string()),
        _ => "unknown".to_string(),
    }
}

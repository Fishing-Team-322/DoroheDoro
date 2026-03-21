use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shared_proto_root = manifest_dir.join("..").join("contracts").join("proto");
    let edge_proto_root = manifest_dir
        .join("..")
        .join("edge_api")
        .join("contracts")
        .join("proto");
    let shared_runtime_proto = shared_proto_root.join("runtime.proto");
    let shared_agent_proto = shared_proto_root.join("agent.proto");
    let shared_ingest_proto = shared_proto_root.join("ingest.proto");
    let edge_proto = edge_proto_root.join("edge.proto");

    println!("cargo:rerun-if-changed={}", shared_runtime_proto.display());
    println!("cargo:rerun-if-changed={}", shared_agent_proto.display());
    println!("cargo:rerun-if-changed={}", shared_ingest_proto.display());
    println!("cargo:rerun-if-changed={}", edge_proto.display());

    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc");
    std::env::set_var("PROTOC", protoc);

    let mut config = prost_build::Config::new();
    config.btree_map(["."]);
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");

    // TECHNICAL DEBT:
    // `agent-rs` still needs the current Edge ingress proto from `edge_api/**` at build time.
    // This dependency is intentionally isolated to build.rs until the shared ingress contract is
    // moved under `contracts/**` without requiring cross-component edits in this task.
    config
        .compile_protos(
            &[
                shared_runtime_proto,
                shared_agent_proto,
                shared_ingest_proto,
                edge_proto,
            ],
            &[shared_proto_root, edge_proto_root],
        )
        .expect("compile proto contracts");
}

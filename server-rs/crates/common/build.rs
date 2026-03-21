use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
        .join("contracts")
        .join("proto");

    let ingest_proto = proto_root.join("ingest.proto");
    let runtime_proto = proto_root.join("runtime.proto");
    let agent_proto = proto_root.join("agent.proto");
    let edge_proto = proto_root.join("edge.proto");
    let control_proto = proto_root.join("control.proto");
    let deployment_proto = proto_root.join("deployment.proto");
    let query_proto = proto_root.join("query.proto");
    let alerts_proto = proto_root.join("alerts.proto");
    let audit_proto = proto_root.join("audit.proto");

    println!("cargo:rerun-if-changed={}", ingest_proto.display());
    println!("cargo:rerun-if-changed={}", runtime_proto.display());
    println!("cargo:rerun-if-changed={}", agent_proto.display());
    println!("cargo:rerun-if-changed={}", edge_proto.display());
    println!("cargo:rerun-if-changed={}", control_proto.display());
    println!("cargo:rerun-if-changed={}", deployment_proto.display());
    println!("cargo:rerun-if-changed={}", query_proto.display());
    println!("cargo:rerun-if-changed={}", alerts_proto.display());
    println!("cargo:rerun-if-changed={}", audit_proto.display());

    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc");
    std::env::set_var("PROTOC", protoc);

    let mut config = prost_build::Config::new();
    config.btree_map(["."]);
    config.type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]");

    config
        .compile_protos(
            &[
                ingest_proto,
                runtime_proto,
                agent_proto,
                edge_proto,
                control_proto,
                deployment_proto,
                query_proto,
                alerts_proto,
                audit_proto,
            ],
            &[proto_root],
        )
        .expect("compile proto contracts");
}

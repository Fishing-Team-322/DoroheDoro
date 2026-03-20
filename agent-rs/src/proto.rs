#[allow(dead_code)]
pub mod agent {
    include!(concat!(env!("OUT_DIR"), "/dorohedoro.agent.v1.rs"));
}

#[allow(dead_code)]
pub mod edge {
    include!(concat!(env!("OUT_DIR"), "/dorohedoro.edge.v1.rs"));
}

#[allow(dead_code)]
pub mod ingest {
    include!(concat!(env!("OUT_DIR"), "/dorohedoro.v1.rs"));
}

use std::{collections::BTreeMap, fs};

use reqwest::Url;
use serde::Deserialize;

use common::{AppError, AppResult};

use crate::{config::ArtifactResolverConfig, models::ResolvedArtifact, models::ResolvedHost};

const LABEL_ARCH_KEYS: &[&str] = &["arch", "os.arch", "cpu_arch", "agent_arch"];
const LABEL_DISTRO_KEYS: &[&str] = &[
    "distro_family",
    "os.family",
    "linux_distro_family",
    "agent_distro_family",
];

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseManifest {
    pub schema_version: String,
    pub version: String,
    pub generated_at: String,
    pub artifacts: Vec<ReleaseArtifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReleaseArtifact {
    pub platform: String,
    pub arch: String,
    pub package_type: String,
    pub distro_family: String,
    pub install_mode: String,
    pub artifact_name: String,
    pub artifact_path: String,
    #[serde(default)]
    pub source_uri: Option<String>,
    pub checksum_file: String,
    pub bundle_root: Option<String>,
    pub sha256: String,
    pub packaging_preference: Option<i32>,
    #[serde(default)]
    pub image_repository: Option<String>,
    #[serde(default)]
    pub image_tag: Option<String>,
    #[serde(default)]
    pub image_digest: Option<String>,
    #[serde(default)]
    pub image_reference: Option<String>,
    #[serde(default)]
    pub image_digest_reference: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HostArtifactResolution {
    pub artifact: ResolvedArtifact,
    pub warnings: Vec<String>,
}

pub async fn load_manifest(settings: &ArtifactResolverConfig) -> AppResult<ReleaseManifest> {
    let raw = if looks_like_url(&settings.manifest_url) {
        reqwest::Client::new()
            .get(&settings.manifest_url)
            .send()
            .await
            .map_err(|error| {
                AppError::internal(format!(
                    "download agent artifact manifest `{}`: {error}",
                    settings.manifest_url
                ))
            })?
            .error_for_status()
            .map_err(|error| {
                AppError::internal(format!(
                    "download agent artifact manifest `{}`: {error}",
                    settings.manifest_url
                ))
            })?
            .text()
            .await
            .map_err(|error| {
                AppError::internal(format!(
                    "read agent artifact manifest `{}`: {error}",
                    settings.manifest_url
                ))
            })?
    } else {
        fs::read_to_string(&settings.manifest_url).map_err(|error| {
            AppError::internal(format!(
                "read agent artifact manifest `{}`: {error}",
                settings.manifest_url
            ))
        })?
    };

    let manifest: ReleaseManifest = serde_json::from_str(&raw).map_err(|error| {
        AppError::internal(format!(
            "parse agent artifact manifest `{}`: {error}",
            settings.manifest_url
        ))
    })?;

    if manifest.schema_version != "1.0" {
        return Err(AppError::internal(format!(
            "unsupported artifact manifest schema_version `{}`",
            manifest.schema_version
        )));
    }
    if let Some(version) = settings.artifact_version.as_deref() {
        if manifest.version != version {
            return Err(AppError::internal(format!(
                "artifact manifest version mismatch: expected `{version}`, got `{}`",
                manifest.version
            )));
        }
    }
    if manifest.artifacts.is_empty() {
        return Err(AppError::internal(
            "agent artifact manifest does not contain any artifacts",
        ));
    }

    Ok(manifest)
}

pub fn resolve_for_host(
    manifest: &ReleaseManifest,
    settings: &ArtifactResolverConfig,
    host: &ResolvedHost,
) -> AppResult<HostArtifactResolution> {
    let mut warnings = Vec::new();
    let arch = resolve_host_attr(host, LABEL_ARCH_KEYS).unwrap_or_else(|| {
        warnings.push(format!(
            "host `{}` is missing arch labels; defaulting to amd64 artifact resolution",
            host.hostname
        ));
        "amd64".to_string()
    });
    let distro_family = resolve_host_attr(host, LABEL_DISTRO_KEYS).unwrap_or_else(|| {
        warnings.push(format!(
            "host `{}` is missing distro-family labels; defaulting to generic-glibc artifact resolution",
            host.hostname
        ));
        "generic-glibc".to_string()
    });
    let preferred_package_type = settings
        .preferred_package_type
        .clone()
        .unwrap_or_else(|| infer_package_type(&distro_family));

    let arch_matches = manifest
        .artifacts
        .iter()
        .filter(|artifact| artifact.platform == "linux" && artifact.arch == arch)
        .collect::<Vec<_>>();
    if arch_matches.is_empty() {
        return Err(AppError::internal(format!(
            "no agent artifacts found for host `{}` with arch `{arch}`",
            host.hostname
        )));
    }

    let candidates = select_by_package_and_distro(&arch_matches, &preferred_package_type, &distro_family)
        .or_else(|| select_by_package_and_distro(&arch_matches, &preferred_package_type, "generic-glibc"))
        .or_else(|| select_best(&arch_matches))
        .ok_or_else(|| {
            AppError::internal(format!(
                "no usable agent artifact found for host `{}` (arch={arch}, distro_family={distro_family})",
                host.hostname
            ))
        })?;

    if candidates.package_type != preferred_package_type {
        warnings.push(format!(
            "host `{}` preferred `{preferred_package_type}` packages but fell back to `{}`",
            host.hostname, candidates.package_type
        ));
    }
    if candidates.distro_family != distro_family && candidates.distro_family != "generic-glibc" {
        warnings.push(format!(
            "host `{}` requested distro_family `{distro_family}` but resolved `{}`",
            host.hostname, candidates.distro_family
        ));
    }

    Ok(HostArtifactResolution {
        artifact: ResolvedArtifact {
            version: manifest.version.clone(),
            platform: candidates.platform.clone(),
            arch: candidates.arch.clone(),
            package_type: candidates.package_type.clone(),
            distro_family: candidates.distro_family.clone(),
            install_mode: candidates.install_mode.clone(),
            artifact_name: candidates.artifact_name.clone(),
            artifact_path: candidates.artifact_path.clone(),
            source_uri: resolve_source_uri(candidates, settings)?,
            checksum_file: candidates.checksum_file.clone(),
            sha256: candidates.sha256.clone(),
            bundle_root: candidates.bundle_root.clone(),
            image_repository: candidates.image_repository.clone(),
            image_tag: candidates.image_tag.clone(),
            image_digest: candidates.image_digest.clone(),
            image_reference: candidates.image_reference.clone(),
            image_digest_reference: candidates.image_digest_reference.clone(),
        },
        warnings,
    })
}

pub fn unresolved_artifact() -> ResolvedArtifact {
    ResolvedArtifact {
        version: "unresolved".to_string(),
        platform: "linux".to_string(),
        arch: "unknown".to_string(),
        package_type: "auto".to_string(),
        distro_family: "unknown".to_string(),
        install_mode: "unknown".to_string(),
        artifact_name: String::new(),
        artifact_path: String::new(),
        source_uri: String::new(),
        checksum_file: String::new(),
        sha256: String::new(),
        bundle_root: None,
        image_repository: None,
        image_tag: None,
        image_digest: None,
        image_reference: None,
        image_digest_reference: None,
    }
}

fn resolve_host_attr(host: &ResolvedHost, keys: &[&str]) -> Option<String> {
    let labels = host
        .labels
        .iter()
        .map(|(key, value)| (key.to_ascii_lowercase(), value.trim().to_string()))
        .collect::<BTreeMap<_, _>>();

    keys.iter()
        .find_map(|key| labels.get(&key.to_ascii_lowercase()).cloned())
        .filter(|value| !value.is_empty())
        .map(normalize_host_value)
}

fn normalize_host_value(value: String) -> String {
    match value.to_ascii_lowercase().as_str() {
        "x86_64" => "amd64".to_string(),
        "aarch64" => "arm64".to_string(),
        "ubuntu" | "debian-family" => "debian".to_string(),
        "astra" => "astra-linux".to_string(),
        other => other.to_string(),
    }
}

fn infer_package_type(distro_family: &str) -> String {
    match distro_family {
        "debian" | "astra-linux" => "deb".to_string(),
        _ => "tar.gz".to_string(),
    }
}

fn select_by_package_and_distro<'a>(
    artifacts: &'a [&'a ReleaseArtifact],
    package_type: &str,
    distro_family: &str,
) -> Option<&'a ReleaseArtifact> {
    artifacts
        .iter()
        .copied()
        .filter(|artifact| {
            artifact.package_type == package_type && artifact.distro_family == distro_family
        })
        .max_by_key(|artifact| artifact.packaging_preference.unwrap_or(0))
}

fn select_best<'a>(artifacts: &'a [&'a ReleaseArtifact]) -> Option<&'a ReleaseArtifact> {
    artifacts
        .iter()
        .copied()
        .max_by_key(|artifact| artifact.packaging_preference.unwrap_or(0))
}

fn resolve_source_uri(
    artifact: &ReleaseArtifact,
    settings: &ArtifactResolverConfig,
) -> AppResult<String> {
    if let Some(source) = artifact
        .source_uri
        .as_ref()
        .map(|value| value.trim())
        .filter(|v| !v.is_empty())
    {
        return Ok(source.to_string());
    }
    if looks_like_url(&artifact.artifact_path) {
        return Ok(artifact.artifact_path.clone());
    }
    if let Some(base_url) = settings.release_base_url.as_deref() {
        let normalized_base = if base_url.ends_with('/') {
            base_url.to_string()
        } else {
            format!("{base_url}/")
        };
        let base = Url::parse(&normalized_base).map_err(|error| {
            AppError::internal(format!(
                "invalid AGENT_RELEASE_BASE_URL `{base_url}`: {error}"
            ))
        })?;
        return Ok(base
            .join(
                artifact
                    .artifact_path
                    .trim_start_matches("./")
                    .trim_start_matches('/'),
            )
            .map_err(|error| {
                AppError::internal(format!(
                    "join AGENT_RELEASE_BASE_URL with artifact path `{}`: {error}",
                    artifact.artifact_path
                ))
            })?
            .to_string());
    }
    Ok(artifact.artifact_path.clone())
}

fn looks_like_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::Path};

    use uuid::Uuid;

    use super::{load_manifest, resolve_for_host, unresolved_artifact};
    use crate::{config::ArtifactResolverConfig, models::ResolvedHost};

    #[tokio::test]
    async fn loads_example_manifest() {
        let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("..")
            .join("deployments")
            .join("artifacts")
            .join("example.manifest.json");
        let manifest = load_manifest(&ArtifactResolverConfig {
            manifest_url: manifest_path.display().to_string(),
            release_base_url: Some("https://downloads.example.local/agent/".to_string()),
            artifact_version: Some("0.2.0".to_string()),
            preferred_package_type: Some("deb".to_string()),
        })
        .await
        .unwrap();

        assert_eq!(manifest.version, "0.2.0");
        assert_eq!(manifest.generated_at, "2026-03-21T12:00:00Z");
        assert_eq!(manifest.artifacts.len(), 3);
    }

    #[test]
    fn resolves_best_artifact_for_host_labels() {
        let manifest: super::ReleaseManifest = serde_json::from_str(include_str!(
            "../../../../deployments/artifacts/example.manifest.json"
        ))
        .unwrap();
        let resolution = resolve_for_host(
            &manifest,
            &ArtifactResolverConfig {
                manifest_url: "deployments/artifacts/example.manifest.json".to_string(),
                release_base_url: Some("https://downloads.example.local/agent/".to_string()),
                artifact_version: Some("0.2.0".to_string()),
                preferred_package_type: Some("deb".to_string()),
            },
            &ResolvedHost {
                host_id: Uuid::new_v4(),
                hostname: "debian-1".to_string(),
                ip: "10.0.0.10".to_string(),
                ssh_port: 22,
                remote_user: "root".to_string(),
                labels: BTreeMap::from([
                    ("arch".to_string(), "amd64".to_string()),
                    ("distro_family".to_string(), "debian".to_string()),
                ]),
            },
        )
        .unwrap();

        assert_eq!(resolution.artifact.package_type, "deb");
        assert_eq!(resolution.artifact.distro_family, "debian");
        assert_eq!(
            resolution.artifact.source_uri,
            "https://downloads.example.local/agent/releases/0.2.0/doro-agent_0.2.0_linux_amd64.deb"
        );
        assert!(resolution.warnings.is_empty());
    }

    #[test]
    fn resolves_release_base_url_without_trailing_slash() {
        let manifest: super::ReleaseManifest = serde_json::from_str(include_str!(
            "../../../../deployments/artifacts/example.manifest.json"
        ))
        .unwrap();
        let resolution = resolve_for_host(
            &manifest,
            &ArtifactResolverConfig {
                manifest_url: "deployments/artifacts/example.manifest.json".to_string(),
                release_base_url: Some("https://downloads.example.local/agent".to_string()),
                artifact_version: Some("0.2.0".to_string()),
                preferred_package_type: Some("deb".to_string()),
            },
            &ResolvedHost {
                host_id: Uuid::new_v4(),
                hostname: "debian-2".to_string(),
                ip: "10.0.0.11".to_string(),
                ssh_port: 22,
                remote_user: "root".to_string(),
                labels: BTreeMap::from([
                    ("arch".to_string(), "amd64".to_string()),
                    ("distro_family".to_string(), "debian".to_string()),
                ]),
            },
        )
        .unwrap();

        assert_eq!(
            resolution.artifact.source_uri,
            "https://downloads.example.local/agent/releases/0.2.0/doro-agent_0.2.0_linux_amd64.deb"
        );
    }

    #[test]
    fn unresolved_artifact_uses_placeholder_values() {
        let artifact = unresolved_artifact();
        assert_eq!(artifact.version, "unresolved");
        assert_eq!(artifact.package_type, "auto");
    }

    #[test]
    fn falls_back_to_tarball_when_container_artifact_is_unavailable() {
        let manifest: super::ReleaseManifest = serde_json::from_str(include_str!(
            "../../../../deployments/artifacts/example.manifest.json"
        ))
        .unwrap();
        let resolution = resolve_for_host(
            &manifest,
            &ArtifactResolverConfig {
                manifest_url: "deployments/artifacts/example.manifest.json".to_string(),
                release_base_url: None,
                artifact_version: Some("0.2.0".to_string()),
                preferred_package_type: Some("container".to_string()),
            },
            &ResolvedHost {
                host_id: Uuid::new_v4(),
                hostname: "generic-1".to_string(),
                ip: "10.0.0.20".to_string(),
                ssh_port: 22,
                remote_user: "root".to_string(),
                labels: BTreeMap::from([("arch".to_string(), "amd64".to_string())]),
            },
        )
        .unwrap();

        assert_eq!(resolution.artifact.package_type, "tar.gz");
        assert_eq!(
            resolution.artifact.source_uri,
            "releases/0.2.0/doro-agent_0.2.0_linux_amd64.tar.gz"
        );
        assert!(resolution.artifact.image_reference.is_none());
        assert!(resolution.artifact.image_digest_reference.is_none());
    }
}

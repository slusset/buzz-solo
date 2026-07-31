//! Durable context roots: `buzz context init` and `buzz context explore`.
//!
//! First tooling slice of `specs/architecture/durable-context-tooling-v0.1.md`:
//! `init` creates the canonical `.context/` opt-in, a charter stub, and the
//! context's journal presence record; `explore` renders the read-only
//! deterministic projection over every discovered context. Neither command
//! publishes, replicates, or mutates anything the explorer observes.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::profile::{ProfileEnvironment, ResolvedProfile};
use super::runtime::{event_builder, hostname, now_secs, tag, ContextRuntime};
use crate::error::CliError;

/// Tag marking a context's journal presence record.
const CONTEXT_INIT_TAG: &str = "context-init";
/// Relative path of the authored opt-in inside a context root.
const OPT_IN_PATH: &str = ".context/context.yaml";

/// Authored context opt-in: the one hand-written document in a root.
///
/// Mirrors `ContextOptIn` in the durable-context-hooks contract. Everything
/// else under `.context/` is a generated projection; this file is authority
/// for identity and policy, and tooling never overwrites an existing one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextOptIn {
    /// Immutable bounded-context identifier (the NIP-29 `h` value).
    pub context_id: String,
    /// Kebab-case slug used for filenames and selection.
    pub slug: String,
    /// Human name for recognition-based selection; not an address.
    pub display_name: String,
    /// Whether harness adapters may bind sessions to this context.
    pub enabled: bool,
    /// Lifecycle residue disclosure posture.
    pub disclosure_policy: String,
    /// Declared linked repositories/directories; targets are never traversed.
    #[serde(default)]
    pub linked_directories: Vec<LinkedDirectory>,
    /// Skill names resolved against the repository skill tree.
    #[serde(default)]
    pub skills: Vec<String>,
    /// Checkpoint safety policy.
    pub checkpoint_policy: CheckpointPolicy,
}

/// One linked repository or directory: recorded metadata, never traversed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedDirectory {
    /// Presentation name of the link inside the root.
    pub name: String,
    /// Absolute target path, hashed as a string during inventory.
    pub target: String,
}

/// Checkpoint safety policy with the contract's fail-closed defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointPolicy {
    /// Must be false: inventory never follows symbolic links.
    pub follow_symbolic_links: bool,
    /// Must be false: binary artifacts are rejected.
    pub allow_binary: bool,
    /// Must be `metadata_only`: repository contents stay outside inventory.
    pub repository_capture: String,
    /// Maximum accepted artifact size in bytes.
    pub maximum_artifact_bytes: u64,
    /// Filename patterns rejected as sensitive.
    pub sensitive_filename_patterns: Vec<String>,
    /// Content patterns rejected as sensitive.
    pub sensitive_content_patterns: Vec<String>,
}

impl Default for CheckpointPolicy {
    fn default() -> Self {
        Self {
            follow_symbolic_links: false,
            allow_binary: false,
            repository_capture: "metadata_only".into(),
            maximum_artifact_bytes: 1_048_576,
            sensitive_filename_patterns: vec![
                "*.key".into(),
                "*.pem".into(),
                "*.p12".into(),
                ".env*".into(),
                "*credential*".into(),
                "*secret*".into(),
            ],
            sensitive_content_patterns: vec![
                "PRIVATE KEY".into(),
                "BUZZ_PRIVATE_KEY".into(),
                "AUTHORIZATION:".into(),
            ],
        }
    }
}

/// Outcome of `buzz context init` for reporting.
#[derive(Debug, Serialize)]
pub struct InitReport {
    /// Immutable context identifier (minted once, respected thereafter).
    pub context_id: String,
    /// Canonicalized root path.
    pub root: String,
    /// Whether this invocation minted a new opt-in.
    pub created: bool,
    /// Files written by this invocation, relative to the root.
    pub written: Vec<String>,
    /// Journal presence record id, when one was appended.
    pub journal_event_id: Option<String>,
}

/// One context in the explorer overview, ordered by warmth.
#[derive(Debug, Serialize)]
pub struct ContextSummary {
    pub context_id: String,
    pub slug: String,
    pub display_name: String,
    pub root: String,
    pub enabled: bool,
    /// `fresh | accreting | stale | cold | untouched | unknown`.
    pub freshness: String,
    /// RFC 3339 timestamp of the newest residue record, when known.
    pub last_touch: Option<String>,
}

/// Resolve the durable-context home directory.
///
/// Honors `DURABLE_CONTEXT_HOME`, defaulting to `~/DurableContext`.
pub fn durable_context_home(home_env: Option<&str>, user_home: &Path) -> PathBuf {
    match home_env {
        Some(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => user_home.join("DurableContext"),
    }
}

/// Validate a context slug: lowercase kebab-case, no leading/trailing dash.
pub fn validate_slug(slug: &str) -> Result<(), CliError> {
    let valid = !slug.is_empty()
        && slug.len() <= 64
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !slug.starts_with('-')
        && !slug.ends_with('-');
    if valid {
        Ok(())
    } else {
        Err(CliError::Usage(format!(
            "slug '{slug}' must be lowercase kebab-case ([a-z0-9-], no edge dashes, max 64)"
        )))
    }
}

/// Classify residue age into the bounded-attention freshness ramp.
pub fn classify_freshness(last_touch_secs: Option<u64>, now_secs: u64) -> &'static str {
    match last_touch_secs {
        None => "untouched",
        Some(touch) => {
            let age = now_secs.saturating_sub(touch);
            if age < 86_400 {
                "fresh"
            } else if age < 7 * 86_400 {
                "accreting"
            } else if age < 30 * 86_400 {
                "stale"
            } else {
                "cold"
            }
        }
    }
}

/// Parse the authored opt-in from a context root, if present.
pub fn read_opt_in(root: &Path) -> Result<Option<ContextOptIn>, CliError> {
    let path = root.join(OPT_IN_PATH);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|error| CliError::Other(format!("could not read {}: {error}", path.display())))?;
    let opt_in = serde_yaml::from_str(&text).map_err(|error| {
        CliError::Usage(format!(
            "authored opt-in {} did not parse: {error}",
            path.display()
        ))
    })?;
    Ok(Some(opt_in))
}

/// Discover context roots directly under the durable-context home.
///
/// Unreadable or unparseable roots are skipped and reported as warnings so a
/// single bad root cannot hide the rest; results are name-ordered for
/// deterministic output.
pub fn discover_roots(home: &Path) -> (Vec<(PathBuf, ContextOptIn)>, Vec<String>) {
    let mut found = Vec::new();
    let mut warnings = Vec::new();
    let Ok(entries) = std::fs::read_dir(home) else {
        return (found, warnings);
    };
    let mut roots: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir())
        .collect();
    roots.sort();
    for root in roots {
        match read_opt_in(&root) {
            Ok(Some(opt_in)) => found.push((root, opt_in)),
            Ok(None) => {}
            Err(error) => warnings.push(format!("{}: {error}", root.display())),
        }
    }
    (found, warnings)
}

/// Walk upward from a path to the nearest directory containing `.context/context.yaml`.
pub fn resolve_root_upward(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(OPT_IN_PATH).is_file() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

/// Resolve a working path to its owning context via declared linked targets.
///
/// Fail-closed: zero matches yield `Ok(None)`; more than one owning context
/// is an explicit ambiguity error, mirroring `binding-fails-closed`.
pub fn resolve_by_linked_target(
    path: &Path,
    roots: &[(PathBuf, ContextOptIn)],
) -> Result<Option<PathBuf>, CliError> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut owners: Vec<&PathBuf> = Vec::new();
    for (root, opt_in) in roots {
        for link in &opt_in.linked_directories {
            let target = PathBuf::from(&link.target);
            let target = target.canonicalize().unwrap_or(target);
            if canonical.starts_with(&target) {
                owners.push(root);
                break;
            }
        }
    }
    match owners.as_slice() {
        [] => Ok(None),
        [only] => Ok(Some((*only).clone())),
        many => Err(CliError::Usage(format!(
            "ambiguous context membership: {} contexts link this path ({}); binding fails closed",
            many.len(),
            many.iter()
                .map(|root| root.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Write the opt-in and charter stub for a fresh root. Never overwrites.
fn write_root_files(root: &Path, opt_in: &ContextOptIn) -> Result<Vec<String>, CliError> {
    let mut written = Vec::new();
    let context_dir = root.join(".context");
    std::fs::create_dir_all(context_dir.join("sessions")).map_err(|error| {
        CliError::Other(format!(
            "could not create {}: {error}",
            context_dir.display()
        ))
    })?;
    let opt_in_path = root.join(OPT_IN_PATH);
    if !opt_in_path.exists() {
        let body = serde_yaml::to_string(opt_in)
            .map_err(|error| CliError::Other(format!("opt-in serialization failed: {error}")))?;
        std::fs::write(&opt_in_path, body).map_err(|error| {
            CliError::Other(format!(
                "could not write {}: {error}",
                opt_in_path.display()
            ))
        })?;
        written.push(OPT_IN_PATH.to_string());
    }
    let charter_name = format!("{}-context-charter.md", opt_in.slug);
    let charter_path = root.join(&charter_name);
    if !charter_path.exists() {
        let charter = format!(
            "# {} — context charter\n\nContext ID: `{}`\n\nWhat this domain is, why it exists, and what stewardship means here.\nAuthored by the steward; the journal carries history, this file carries\nintent.\n",
            opt_in.display_name, opt_in.context_id
        );
        std::fs::write(&charter_path, charter).map_err(|error| {
            CliError::Other(format!(
                "could not write {}: {error}",
                charter_path.display()
            ))
        })?;
        written.push(charter_name);
    }
    Ok(written)
}

/// `buzz context init` — create or complete a durable context root.
pub async fn init(
    profile: &ResolvedProfile,
    environment: &ProfileEnvironment,
    root: &Path,
    slug: &str,
    display_name: Option<&str>,
    json: bool,
) -> Result<(), CliError> {
    validate_slug(slug)?;
    std::fs::create_dir_all(root).map_err(|error| {
        CliError::Other(format!("could not create {}: {error}", root.display()))
    })?;
    let existing = read_opt_in(root)?;
    let created = existing.is_none();
    let opt_in = match existing {
        Some(authored) => authored,
        None => ContextOptIn {
            context_id: uuid::Uuid::new_v4().to_string(),
            slug: slug.to_string(),
            display_name: display_name.unwrap_or(slug).to_string(),
            enabled: true,
            disclosure_policy: "metadata_only".into(),
            linked_directories: Vec::new(),
            skills: Vec::new(),
            checkpoint_policy: CheckpointPolicy::default(),
        },
    };
    if !created && opt_in.slug != slug {
        return Err(CliError::Usage(format!(
            "root is already initialized as slug '{}'; refusing to re-slug (authored opt-in is authority)",
            opt_in.slug
        )));
    }
    let written = write_root_files(root, &opt_in)?;

    // Journal presence: append the context-init residue record exactly once.
    // Re-runs complete a presence record that a previous run failed to write
    // (for example when the relay was down after files landed on disk).
    let runtime = ContextRuntime::new(profile, environment)?;
    let existing_presence = runtime
        .query_preferred(&[serde_json::json!({
            "kinds": [nostr::Kind::TextNote.as_u16()],
            "#h": [opt_in.context_id],
            "#t": [CONTEXT_INIT_TAG],
            "limit": 1,
        })])
        .await?;
    let journal_event_id = if existing_presence.is_empty() {
        let (_, role) = runtime.local_event_client()?;
        let identity = runtime.identity_label(role);
        let machine = hostname();
        let content = format!(
            "Durable context initialized: {} ({})",
            opt_in.display_name, opt_in.slug
        );
        let tags = vec![
            tag(&["t", CONTEXT_INIT_TAG])?,
            tag(&["h", &opt_in.context_id])?,
            tag(&["agent", &identity])?,
            tag(&["machine", &machine])?,
        ];
        Some(
            runtime
                .post_builder(event_builder(content, tags, None))
                .await?,
        )
    } else {
        None
    };

    let report = InitReport {
        context_id: opt_in.context_id.clone(),
        root: root
            .canonicalize()
            .unwrap_or_else(|_| root.to_path_buf())
            .display()
            .to_string(),
        created,
        written,
        journal_event_id,
    };
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).map_err(|error| CliError::Other(format!(
                "report serialization failed: {error}"
            )))?
        );
    } else {
        println!(
            "{} {} ({}) at {}",
            if report.created {
                "initialized"
            } else {
                "already initialized"
            },
            opt_in.display_name,
            report.context_id,
            report.root
        );
        for file in &report.written {
            println!("  wrote {file}");
        }
        if let Some(id) = &report.journal_event_id {
            println!("  journal presence {id}");
        }
    }
    Ok(())
}

/// `buzz context explore` — the read-only deterministic projection.
pub async fn explore(
    profile: &ResolvedProfile,
    environment: &ProfileEnvironment,
    target: Option<&str>,
    json: bool,
) -> Result<(), CliError> {
    let user_home = environment.home.clone();
    let home_env = std::env::var("DURABLE_CONTEXT_HOME").ok();
    let home = durable_context_home(home_env.as_deref(), &user_home);
    let (roots, warnings) = discover_roots(&home);
    for warning in &warnings {
        eprintln!("warning: skipped {warning}");
    }
    let runtime = ContextRuntime::new(profile, environment)?;
    match target {
        None => overview(&runtime, &roots, json).await,
        Some(target) => {
            let root = resolve_target(target, &roots)?;
            let opt_in = read_opt_in(&root)?.ok_or_else(|| {
                CliError::Usage(format!("{} has no authored opt-in", root.display()))
            })?;
            expanded(&runtime, &root, &opt_in, json).await
        }
    }
}

/// Resolve an explore target: path (direct or via linked targets), slug, or id.
fn resolve_target(target: &str, roots: &[(PathBuf, ContextOptIn)]) -> Result<PathBuf, CliError> {
    let as_path = Path::new(target);
    if as_path.exists() {
        let canonical = as_path
            .canonicalize()
            .unwrap_or_else(|_| as_path.to_path_buf());
        if let Some(root) = resolve_root_upward(&canonical) {
            return Ok(root);
        }
        if let Some(root) = resolve_by_linked_target(&canonical, roots)? {
            return Ok(root);
        }
        return Err(CliError::Usage(format!(
            "no enabled context owns {target}; binding fails closed"
        )));
    }
    let matches: Vec<&(PathBuf, ContextOptIn)> = roots
        .iter()
        .filter(|(_, opt_in)| opt_in.slug == target || opt_in.context_id == target)
        .collect();
    match matches.as_slice() {
        [] => Err(CliError::Usage(format!(
            "no context named '{target}' under the durable-context home"
        ))),
        [(root, _)] => Ok(root.clone()),
        many => Err(CliError::Usage(format!(
            "'{target}' matches {} contexts; binding fails closed",
            many.len()
        ))),
    }
}

async fn overview(
    runtime: &ContextRuntime<'_>,
    roots: &[(PathBuf, ContextOptIn)],
    json: bool,
) -> Result<(), CliError> {
    let now = now_secs();
    let mut summaries = Vec::new();
    for (root, opt_in) in roots {
        let (freshness, last_touch) = match latest_touch(runtime, &opt_in.context_id).await {
            Ok(touch) => (
                classify_freshness(touch, now).to_string(),
                touch.and_then(|secs| {
                    DateTime::<Utc>::from_timestamp(secs as i64, 0).map(|t| t.to_rfc3339())
                }),
            ),
            Err(_) => ("unknown".to_string(), None),
        };
        summaries.push(ContextSummary {
            context_id: opt_in.context_id.clone(),
            slug: opt_in.slug.clone(),
            display_name: opt_in.display_name.clone(),
            root: root.display().to_string(),
            enabled: opt_in.enabled,
            freshness,
            last_touch,
        });
    }
    // Warmth order: most recently touched first, untouched/unknown last,
    // then stable slug order for determinism.
    summaries.sort_by(|a, b| {
        b.last_touch
            .cmp(&a.last_touch)
            .then_with(|| a.slug.cmp(&b.slug))
    });
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&summaries)
                .map_err(|error| CliError::Other(format!("serialization failed: {error}")))?
        );
        return Ok(());
    }
    if summaries.is_empty() {
        println!("no durable contexts discovered");
        return Ok(());
    }
    for summary in &summaries {
        println!(
            "{:<10} {} ({})  h={}  last={}  root={}{}",
            summary.freshness,
            summary.display_name,
            summary.slug,
            summary.context_id,
            summary.last_touch.as_deref().unwrap_or("never"),
            summary.root,
            if summary.enabled { "" } else { "  [disabled]" },
        );
    }
    Ok(())
}

async fn latest_touch(
    runtime: &ContextRuntime<'_>,
    context_id: &str,
) -> Result<Option<u64>, CliError> {
    let events = runtime
        .query_preferred(&[serde_json::json!({
            "kinds": [nostr::Kind::TextNote.as_u16()],
            "#h": [context_id],
            "limit": 1,
        })])
        .await?;
    Ok(events.first().map(|event| event.created_at.as_secs()))
}

async fn expanded(
    runtime: &ContextRuntime<'_>,
    root: &Path,
    opt_in: &ContextOptIn,
    json: bool,
) -> Result<(), CliError> {
    let residue = runtime
        .query_preferred(&[serde_json::json!({
            "kinds": [nostr::Kind::TextNote.as_u16()],
            "#h": [opt_in.context_id],
            "limit": 100,
        })])
        .await
        .unwrap_or_default();
    // Partition residue deterministically by leading `t` tag.
    let mut sessions = Vec::new();
    let mut handoffs = Vec::new();
    let mut other = Vec::new();
    for event in &residue {
        let topic = event
            .tags
            .iter()
            .filter_map(|tag| {
                let slice = tag.as_slice();
                (slice.first().map(String::as_str) == Some("t"))
                    .then(|| slice.get(1).map(String::as_str))
                    .flatten()
            })
            .next()
            .unwrap_or("");
        if topic.starts_with("session:") {
            sessions.push(event);
        } else if topic.starts_with("handoff:") {
            handoffs.push(event);
        } else {
            other.push(event);
        }
    }
    if json {
        let value = serde_json::json!({
            "context_id": opt_in.context_id,
            "slug": opt_in.slug,
            "display_name": opt_in.display_name,
            "root": root.display().to_string(),
            "enabled": opt_in.enabled,
            "linked_directories": opt_in.linked_directories,
            "skills": opt_in.skills,
            "residue_count": residue.len(),
            "session_records": sessions.len(),
            "handoff_records": handoffs.len(),
            "residue_tail": other.iter().take(5).map(|event| {
                serde_json::json!({
                    "id": event.id.to_hex(),
                    "created_at": event.created_at.as_secs(),
                    "content": event.content,
                })
            }).collect::<Vec<_>>(),
            "current_work": serde_json::Value::Null,
            "beacon": serde_json::Value::Null,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&value)
                .map_err(|error| CliError::Other(format!("serialization failed: {error}")))?
        );
        return Ok(());
    }
    println!("{} ({})", opt_in.display_name, opt_in.slug);
    println!("  h        {}", opt_in.context_id);
    println!("  root     {}", root.display());
    println!("  enabled  {}", opt_in.enabled);
    if opt_in.linked_directories.is_empty() {
        println!("  links    none");
    } else {
        println!("  links");
        for link in &opt_in.linked_directories {
            println!("    {} -> {}", link.name, link.target);
        }
    }
    if opt_in.skills.is_empty() {
        println!("  skills   none declared");
    } else {
        println!("  skills   {}", opt_in.skills.join(", "));
    }
    println!(
        "  residue  {} records ({} sessions, {} handoffs)",
        residue.len(),
        sessions.len(),
        handoffs.len()
    );
    for event in other.iter().take(5) {
        let timestamp = DateTime::<Utc>::from_timestamp(event.created_at.as_secs() as i64, 0)
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| event.created_at.as_secs().to_string());
        println!("    {timestamp}  {}", event.content);
    }
    println!("  current-work  none recorded");
    println!("  beacon        none declared");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opt_in(slug: &str, links: Vec<LinkedDirectory>) -> ContextOptIn {
        ContextOptIn {
            context_id: format!("id-{slug}"),
            slug: slug.into(),
            display_name: slug.into(),
            enabled: true,
            disclosure_policy: "metadata_only".into(),
            linked_directories: links,
            skills: Vec::new(),
            checkpoint_policy: CheckpointPolicy::default(),
        }
    }

    #[test]
    fn slug_validation_accepts_kebab_and_rejects_edges() {
        assert!(validate_slug("or-temperature-control").is_ok());
        assert!(validate_slug("a1").is_ok());
        assert!(validate_slug("").is_err());
        assert!(validate_slug("-lead").is_err());
        assert!(validate_slug("trail-").is_err());
        assert!(validate_slug("Upper").is_err());
        assert!(validate_slug("has space").is_err());
    }

    #[test]
    fn freshness_ramp_boundaries() {
        let now = 100 * 86_400;
        assert_eq!(classify_freshness(None, now), "untouched");
        assert_eq!(classify_freshness(Some(now - 1), now), "fresh");
        assert_eq!(classify_freshness(Some(now - 86_400), now), "accreting");
        assert_eq!(classify_freshness(Some(now - 7 * 86_400), now), "stale");
        assert_eq!(classify_freshness(Some(now - 30 * 86_400), now), "cold");
    }

    #[test]
    fn home_resolution_honors_env_and_defaults() {
        let user = Path::new("/home/u");
        assert_eq!(
            durable_context_home(Some("/tmp/dc"), user),
            PathBuf::from("/tmp/dc")
        );
        assert_eq!(
            durable_context_home(Some("  "), user),
            PathBuf::from("/home/u/DurableContext")
        );
        assert_eq!(
            durable_context_home(None, user),
            PathBuf::from("/home/u/DurableContext")
        );
    }

    #[test]
    fn opt_in_round_trips_through_yaml() {
        let original = opt_in(
            "demo",
            vec![LinkedDirectory {
                name: "repo".into(),
                target: "/abs/target".into(),
            }],
        );
        let text = serde_yaml::to_string(&original).expect("serialize");
        let parsed: ContextOptIn = serde_yaml::from_str(&text).expect("parse");
        assert_eq!(parsed.context_id, original.context_id);
        assert_eq!(parsed.linked_directories.len(), 1);
        assert!(!parsed.checkpoint_policy.follow_symbolic_links);
        assert_eq!(parsed.checkpoint_policy.repository_capture, "metadata_only");
    }

    #[test]
    fn root_files_written_once_and_identity_stable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("demo");
        std::fs::create_dir_all(&root).expect("mkdir");
        let original = opt_in("demo", Vec::new());
        let first = write_root_files(&root, &original).expect("first write");
        assert_eq!(first.len(), 2);
        let replay = opt_in("demo-two", Vec::new());
        let second = write_root_files(&root, &replay).expect("second write");
        // Existing opt-in and charter are never overwritten; only the new
        // slug's charter stub appears.
        assert_eq!(second, vec!["demo-two-context-charter.md".to_string()]);
        let persisted = read_opt_in(&root).expect("read").expect("present");
        assert_eq!(persisted.context_id, "id-demo");
    }

    #[test]
    fn upward_resolution_finds_nearest_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("ctx");
        std::fs::create_dir_all(root.join(".context")).expect("mkdir");
        std::fs::write(root.join(OPT_IN_PATH), "x: 1\n").expect("write");
        let nested = root.join("a/b/c");
        std::fs::create_dir_all(&nested).expect("mkdir");
        assert_eq!(resolve_root_upward(&nested), Some(root.clone()));
        assert_eq!(resolve_root_upward(dir.path()), None);
    }

    #[test]
    fn linked_target_membership_fails_closed_on_ambiguity() {
        let dir = tempfile::tempdir().expect("tempdir");
        let shared = dir.path().join("shared-repo");
        std::fs::create_dir_all(&shared).expect("mkdir");
        let link = LinkedDirectory {
            name: "repo".into(),
            target: shared.display().to_string(),
        };
        let one = (dir.path().join("one"), opt_in("one", vec![link.clone()]));
        let two = (dir.path().join("two"), opt_in("two", vec![link.clone()]));
        let inside = shared.join("src");
        std::fs::create_dir_all(&inside).expect("mkdir");

        let sole = resolve_by_linked_target(&inside, std::slice::from_ref(&one))
            .expect("resolves")
            .expect("owner");
        assert_eq!(sole, one.0);
        assert!(resolve_by_linked_target(&inside, &[one, two]).is_err());
        let unrelated = dir.path().join("elsewhere");
        std::fs::create_dir_all(&unrelated).expect("mkdir");
        assert!(resolve_by_linked_target(&unrelated, &[])
            .expect("none")
            .is_none());
    }

    #[test]
    fn discovery_skips_unparseable_roots_with_warning() {
        let dir = tempfile::tempdir().expect("tempdir");
        let good = dir.path().join("good");
        std::fs::create_dir_all(good.join(".context")).expect("mkdir");
        let body = serde_yaml::to_string(&opt_in("good", Vec::new())).expect("yaml");
        std::fs::write(good.join(OPT_IN_PATH), body).expect("write");
        let bad = dir.path().join("bad");
        std::fs::create_dir_all(bad.join(".context")).expect("mkdir");
        std::fs::write(bad.join(OPT_IN_PATH), ": not yaml [").expect("write");
        let plain = dir.path().join("plain-dir");
        std::fs::create_dir_all(&plain).expect("mkdir");

        let (found, warnings) = discover_roots(dir.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].1.slug, "good");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("bad"));
    }
}

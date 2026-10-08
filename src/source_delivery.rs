//! Explicit source topic classifications; classification never grants authority.
use crate::{
    domain::{Error, Result, hash},
    project::{Project, SourceTopic},
    reader, session,
};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::collections::BTreeSet;
fn invalid() -> Error {
    Error::new("INVALID_CONFIG", "Invalid bounded source topic policy", 2)
}
/// Validate authored topic labels without returning their contents in errors.
pub fn validate_topics(topics: &[String]) -> Result<()> {
    if topics.len() > 32 {
        return Err(invalid());
    }
    for topic in topics {
        if topic.trim().is_empty()
            || topic.len() > 256
            || topic.contains('\0')
            || topic.chars().any(char::is_control)
            || reader::redact(topic).1
        {
            return Err(invalid());
        }
    }
    Ok(())
}
fn relative(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('/')
        && !value.contains(['\\', ':', '\0'])
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
fn compile(assignments: &[SourceTopic]) -> Result<Vec<(GlobSet, Vec<String>)>> {
    if assignments.len() > 256 {
        return Err(invalid());
    }
    let mut compiled = Vec::new();
    for assignment in assignments {
        validate_topics(&assignment.topics)?;
        if assignment.scope.is_empty() || assignment.scope.len() > 32 {
            return Err(invalid());
        }
        let mut builder = GlobSetBuilder::new();
        for scope in &assignment.scope {
            if scope.len() > 256 || !relative(scope) || reader::redact(scope).1 {
                return Err(invalid());
            }
            builder.add(Glob::new(scope).map_err(|_| invalid())?);
        }
        compiled.push((
            builder.build().map_err(|_| invalid())?,
            assignment.topics.clone(),
        ));
    }
    Ok(compiled)
}
pub(crate) fn validate_assignments(assignments: &[SourceTopic]) -> Result<()> {
    compile(assignments)?;
    Ok(())
}
pub struct SourcePolicy<'a> {
    project: &'a Project,
    session: Option<&'a str>,
    packet_topic: Option<&'a str>,
    owner_role: &'a str,
    compiled: Vec<(GlobSet, Vec<String>)>,
    initial_binding: String,
    policy_hash: String,
    initial_policy: Option<String>,
}
impl<'a> SourcePolicy<'a> {
    pub fn new(
        p: &'a Project,
        consumer: Option<&'a str>,
        packet_topic: Option<&'a str>,
        owner_role: &'a str,
    ) -> Result<Self> {
        p.check_deadline()?;
        let compiled = compile(&p.config.policy.source_topics)?;
        reader::validate_root(p)?;
        let initial_policy = p.current_policy_hash()?;
        if initial_policy
            .as_ref()
            .is_some_and(|current| *current != p.policy_hash())
        {
            return Err(Error::new(
                "CONCURRENT_MODIFICATION",
                "Source delivery policy changed",
                4,
            ));
        }
        let (_, initial_binding) =
            session::delivery_binding(p, consumer, packet_topic, owner_role)?;
        p.check_deadline()?;
        Ok(Self {
            project: p,
            session: consumer,
            packet_topic,
            owner_role,
            compiled,
            initial_binding,
            policy_hash: p.policy_hash(),
            initial_policy,
        })
    }
    /// Recheck the original presence and effective policy, not a fresh binding
    /// which could mistake a deleted initialized configuration for a fixture.
    pub fn revalidate(&self) -> Result<()> {
        self.project.check_deadline()?;
        if self.project.current_policy_hash()? != self.initial_policy {
            return Err(Error::new(
                "CONCURRENT_MODIFICATION",
                "Source delivery policy changed",
                4,
            ));
        }
        self.project.check_deadline()?;
        Ok(())
    }
    /// Some(empty) is an authored public classification. Owner assignments always
    /// contribute their union; no source declaration can remove assigned labels.
    pub fn check(&self, path: &str, declared: Option<&[String]>) -> Result<String> {
        let p = self.project;
        p.check_deadline()?;
        if !relative(path) || path.len() > 4096 {
            return Err(Error::new(
                "PATH_OUTSIDE_ROOT",
                "Invalid source classification path",
                5,
            ));
        }
        reader::policy_allows(p, path)?;
        if let Some(topics) = declared {
            validate_topics(topics)?;
        }
        let mut topics = BTreeSet::new();
        let mut classified = declared.is_some();
        for (scopes, labels) in &self.compiled {
            p.check_deadline()?;
            if scopes.is_match(path) {
                classified = true;
                topics.extend(labels.iter().cloned());
            }
        }
        if let Some(labels) = declared {
            topics.extend(labels.iter().cloned());
        }
        let mut barriers = Vec::new();
        if !classified {
            // A public packet cannot declassify an unknown source under quiet
            // controls. No topic is inferred from the source text or packet.
            match session::delivery_binding(p, self.session, None, self.owner_role) {
                Ok((_, binding)) => barriers.push(binding),
                Err(e) if e.code == "DELIVERY_TOPIC_REQUIRED" => {
                    return Err(Error::new(
                        "SOURCE_TOPIC_REQUIRED",
                        "Explicit source topic classification is required",
                        5,
                    ));
                }
                Err(e) => return Err(e),
            }
        } else if topics.is_empty() {
            // Authored public classification is explicit. Recheck the real packet
            // binding for role/recipient pause; never synthesize a public topic.
            barriers.push(
                session::delivery_binding(p, self.session, self.packet_topic, self.owner_role)?.1,
            );
        } else {
            for topic in &topics {
                p.check_deadline()?;
                barriers.push(
                    session::delivery_binding(p, self.session, Some(topic), self.owner_role)?.1,
                );
            }
        }
        p.check_deadline()?;
        let fingerprint = hash(serde_json::to_vec(
            &serde_json::json!({"source_policy_version":1,
            "policy":self.policy_hash,"path_hash":hash(path.as_bytes()),"declared":declared,
            "classified":classified,"topics":topics,"barriers":barriers,
            "initial_packet_binding":self.initial_binding,"packet_topic":self.packet_topic,
            "session":self.session,"owner_role":self.owner_role,"workspace":p.workspace_id,
            "coordination":p.coordination_id}),
        )?);
        p.check_deadline()?;
        Ok(fingerprint)
    }
}

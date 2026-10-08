use pctx::{
    deadline::Deadline,
    domain::{Error, hash},
    input::{MAX_TASK_BYTES, project_task_document},
    project::{Config, Project, ProjectConfig, RootAnchor},
};
use std::fs;
fn fixture() -> (tempfile::TempDir, Project) {
    let t = tempfile::tempdir().unwrap();
    let base = t.path().canonicalize().unwrap();
    let root = base.join("project");
    let data = base.join("data");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(data.join("workspace")).unwrap();
    fs::create_dir_all(data.join("control")).unwrap();
    let p = Project {
        deadline: None,
        root_anchor: RootAnchor::capture(&root).unwrap(),
        root,
        data_dir: data.clone(),
        workspace_dir: data.join("workspace"),
        control_dir: data.join("control"),
        project_id: "fixture".into(),
        workspace_id: "ws".into(),
        coordination_id: "coord".into(),
        config: Config {
            schema_version: 1,
            project: ProjectConfig {
                id: "fixture".into(),
                name: "fixture".into(),
            },
            index: Default::default(),
            policy: Default::default(),
            search: Default::default(),
            context: Default::default(),
            roles: Default::default(),
        },
    };
    (t, p)
}
#[test]
fn project_admission_precedes_body_and_provenance_has_no_path() {
    let (_t, p) = fixture();
    let path = p.root.join("task.txt");
    fs::write(&path, [0xff]).unwrap();
    let error = project_task_document(&p, path.to_str().unwrap(), |relative| {
        assert_eq!(relative, "task.txt");
        Err(Error::new("TOPIC_SILENCED", "Fixture denial", 5))
    })
    .err()
    .unwrap();
    assert_eq!(error.code, "TOPIC_SILENCED");
    fs::write(&path, "Task 한국어\n").unwrap();
    let doc = project_task_document(&p, path.to_str().unwrap(), |_| Ok(())).unwrap();
    assert_eq!(doc.text, "Task 한국어\n");
    assert_eq!(doc.provenance()["kind"], "project_file");
    assert_eq!(doc.provenance()["hash"], hash(doc.text.as_bytes()));
    assert!(!doc.provenance().to_string().contains("task.txt"));
    doc.revalidate(&p).unwrap();
    fs::write(&path, "Changed task").unwrap();
    assert_eq!(
        doc.revalidate(&p).err().unwrap().code,
        "CONCURRENT_MODIFICATION"
    );
}
#[test]
fn external_input_is_explicit_and_content_or_same_bytes_replacement_is_detected() {
    let (t, p) = fixture();
    let path = t.path().join("external.txt");
    fs::write(&path, "External task").unwrap();
    let doc = project_task_document(&p, path.to_str().unwrap(), |_| {
        panic!("External is not project input")
    })
    .unwrap();
    assert_eq!(doc.provenance()["kind"], "external_file");
    assert!(!doc.provenance().to_string().contains("external.txt"));
    doc.revalidate(&p).unwrap();
    let replacement = t.path().join("replacement");
    fs::write(&replacement, "External task").unwrap();
    fs::rename(&replacement, &path).unwrap();
    assert_eq!(
        doc.revalidate(&p).err().unwrap().code,
        "CONCURRENT_MODIFICATION"
    );
}
#[test]
fn bounds_encoding_and_original_deadline_are_enforced() {
    let (t, mut p) = fixture();
    let path = t.path().join("external.txt");
    fs::write(&path, vec![b'x'; MAX_TASK_BYTES + 1]).unwrap();
    assert_eq!(
        project_task_document(&p, path.to_str().unwrap(), |_| Ok(()))
            .err()
            .unwrap()
            .code,
        "FILE_TOO_LARGE"
    );
    fs::write(&path, [0xff]).unwrap();
    assert_eq!(
        project_task_document(&p, path.to_str().unwrap(), |_| Ok(()))
            .err()
            .unwrap()
            .code,
        "UNSUPPORTED_ENCODING"
    );
    fs::write(&path, "ok").unwrap();
    p.deadline = Some(Deadline::from_instant(std::time::Instant::now()));
    assert_eq!(
        project_task_document(&p, path.to_str().unwrap(), |_| panic!(
            "Expired input admission"
        ))
        .err()
        .unwrap()
        .code,
        "TIMEOUT"
    );
    p.deadline = None;
    let internal = p.root.join("task.txt");
    fs::write(&internal, "too long").unwrap();
    p.config.index.max_file_bytes = 2;
    assert_eq!(
        project_task_document(&p, internal.to_str().unwrap(), |_| Ok(()))
            .err()
            .unwrap()
            .code,
        "FILE_TOO_LARGE"
    );
}
#[cfg(unix)]
#[test]
fn external_alias_into_project_is_admitted_but_internal_aliases_and_escapes_are_denied() {
    use std::os::unix::fs::symlink;
    let (t, p) = fixture();
    let internal = p.root.join("task.txt");
    fs::write(&internal, "project").unwrap();
    let alias = t.path().join("alias");
    symlink(&internal, &alias).unwrap();
    let mut seen = Vec::new();
    let doc = project_task_document(&p, alias.to_str().unwrap(), |path| {
        seen.push(path.to_string());
        Ok(())
    })
    .unwrap();
    assert_eq!(seen, vec!["task.txt"]);
    assert_eq!(doc.provenance()["kind"], "project_file");
    let internal_alias = p.root.join("alias");
    symlink(&internal, &internal_alias).unwrap();
    assert_eq!(
        project_task_document(&p, internal_alias.to_str().unwrap(), |_| panic!(
            "Alias must not be admitted"
        ))
        .err()
        .unwrap()
        .code,
        "POLICY_DENIED"
    );
    let external = t.path().join("external.txt");
    fs::write(&external, "external").unwrap();
    let escape = p.root.join("escape");
    symlink(&external, &escape).unwrap();
    assert_eq!(
        project_task_document(&p, escape.to_str().unwrap(), |_| panic!("Escape"))
            .err()
            .unwrap()
            .code,
        "POLICY_DENIED"
    );
    let traversal = p.root.join("../external.txt");
    assert_eq!(
        project_task_document(&p, traversal.to_str().unwrap(), |_| panic!("Traversal"))
            .err()
            .unwrap()
            .code,
        "POLICY_DENIED"
    );
    fs::remove_file(&alias).unwrap();
    symlink(&external, &alias).unwrap();
    assert_eq!(
        doc.revalidate(&p).err().unwrap().code,
        "CONCURRENT_MODIFICATION"
    );
}
#[cfg(unix)]
#[test]
fn original_external_ancestor_and_canonical_parent_remain_bound() {
    use std::os::unix::fs::symlink;
    let (t, p) = fixture();
    let base = t.path().canonicalize().unwrap();
    let parent = base.join("external");
    fs::create_dir(&parent).unwrap();
    fs::write(parent.join("task.txt"), "task").unwrap();
    let alias = base.join("external_alias");
    symlink(&parent, &alias).unwrap();
    let input = alias.join("task.txt");
    let doc = project_task_document(&p, input.to_str().unwrap(), |_| Ok(())).unwrap();
    // A new ancestor symlink is rejected even when it resolves to the same leaf.
    fs::remove_file(&alias).unwrap();
    let second = base.join("second_alias");
    symlink(&parent, &second).unwrap();
    symlink(&second, &alias).unwrap();
    assert_eq!(
        doc.revalidate(&p).err().unwrap().code,
        "CONCURRENT_MODIFICATION"
    );
    let direct = parent.join("task.txt");
    let pinned = project_task_document(&p, direct.to_str().unwrap(), |_| Ok(())).unwrap();
    fs::rename(&parent, base.join("old_external")).unwrap();
    fs::create_dir(&parent).unwrap();
    fs::write(&direct, "task").unwrap();
    assert!(pinned.revalidate(&p).is_err());
}
#[cfg(unix)]
#[test]
fn fifo_is_rejected_without_waiting_for_a_writer() {
    use std::os::unix::ffi::OsStrExt;
    let (t, p) = fixture();
    let path = t.path().join("fifo");
    let name = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert_eq!(
        project_task_document(&p, path.to_str().unwrap(), |_| Ok(()))
            .err()
            .unwrap()
            .code,
        "INVALID_ARGUMENT"
    );
}

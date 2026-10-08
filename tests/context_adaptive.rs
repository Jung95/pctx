use serde_json::Value;
use std::{
    fs,
    process::{Command, Output},
};

struct Fixture {
    _temp: tempfile::TempDir,
    root: std::path::PathBuf,
    data: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        fs::create_dir(&root).unwrap();
        let f = Self {
            root,
            data: temp.path().join("data"),
            _temp: temp,
        };
        f.ok(&["init"], "json");
        f
    }
    fn run(&self, args: &[&str], format: &str) -> Output {
        Command::new(env!("CARGO_BIN_EXE_pctx"))
            .args(["--root", self.root.to_str().unwrap(), "--format", format])
            .args(args)
            .env("PCTX_DATA_DIR", &self.data)
            .output()
            .unwrap()
    }
    fn ok(&self, args: &[&str], format: &str) -> Output {
        let out = self.run(args, format);
        assert!(
            out.status.success(),
            "args={args:?} status={} stdout={} stderr={}",
            out.status,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }
    fn build(&self, detail: &str, budget: usize) -> (Output, Value) {
        let out = self.ok(
            &[
                "build",
                "--task",
                "inspect code",
                "--seed",
                "code.py",
                "--detail",
                detail,
                "--budget-bytes",
                &budget.to_string(),
            ],
            "json",
        );
        let value: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(
            value["data"]["budget"]["used"].as_u64(),
            Some(out.stdout.len() as u64)
        );
        assert!(out.stdout.len() <= budget);
        (out, value)
    }
}

#[test]
fn multiline_signatures_have_verified_utf8_crlf_ranges_and_distinct_outlines() {
    let f = Fixture::new();
    let source = "# 한글 🦀\r\ndef calculate(\r\n    first: int,\r\n    second: int,\r\n):\r\n    return first + second\r\n";
    fs::write(f.root.join("code.py"), source).unwrap();
    let (_, signature) = f.build("signature", 12000);
    let item = &signature["data"]["items"][0];
    assert_eq!(item["representation"], "signature");
    let span = &item["signatures"][0];
    let start = span["range"]["start_byte"].as_u64().unwrap() as usize;
    let end = span["range"]["end_byte"].as_u64().unwrap() as usize;
    assert_eq!(span["content"], &source[start..end]);
    assert!(span["content"].as_str().unwrap().contains("second: int"));
    assert!(!span["content"].as_str().unwrap().contains("return"));
    assert_eq!(span["range"]["start_line"], 2);
    assert_eq!(span["range"]["end_line"], 5);
    let (_, outline) = f.build("outline", 12000);
    assert_eq!(outline["data"]["items"][0]["representation"], "outline");
    assert!(outline["data"]["items"][0]["signatures"].is_null());
    assert_eq!(
        outline["data"]["items"][0]["symbols"][0]["name"],
        "calculate"
    );
    assert_ne!(
        signature["data"]["context_fingerprint"],
        outline["data"]["context_fingerprint"]
    );
}

#[test]
fn adaptive_budget_uses_intermediate_tiers_and_accounts_final_overhead_deterministically() {
    let f = Fixture::new();
    let source = format!(
        "def calculate(\n    value: int,\n):\n{}",
        (0..76)
            .map(|i| format!("    # long body line {i}: {}\n", "payload ".repeat(18)))
            .collect::<String>()
    );
    fs::write(f.root.join("code.py"), source).unwrap();
    let (_, signature) = f.build("signature", 64000);
    let budget = signature["data"]["budget"]["used"].as_u64().unwrap() as usize + 64;
    let (_, a) = f.build("adaptive", budget);
    assert_eq!(a["data"]["items"][0]["representation"], "signature");
    assert_eq!(a["data"]["items"][0]["downgrade_reason"], "budget");
    let (_, b) = f.build("adaptive", budget);
    assert_eq!(a["data"], b["data"]);
    assert_eq!(a["data"]["omitted_count"], 0);
    // A rich signature set exceeds an outline, which exceeds a reference.
    fs::write(f.root.join("code.py"),(0..30).map(|i|format!(
        "def function_{i}(argument_with_a_long_name: int, other_argument: int):\n    return argument_with_a_long_name\n"))
        .collect::<String>()).unwrap();
    let (_, outline) = f.build("outline", 64000);
    let budget = outline["data"]["budget"]["used"].as_u64().unwrap() as usize + 64;
    let (_, a) = f.build("adaptive", budget);
    assert_eq!(a["data"]["items"][0]["representation"], "outline");
    let (_, reference) = f.build("reference", 64000);
    let budget = reference["data"]["budget"]["used"].as_u64().unwrap() as usize + 64;
    let (_, a) = f.build("adaptive", budget);
    assert_eq!(a["data"]["items"][0]["representation"], "reference");
}

#[test]
fn excerpts_preserve_exact_original_ranges_and_unsupported_signature_is_explicit() {
    let f = Fixture::new();
    let source = (0..95)
        .map(|i| format!("# line {i} 한글\r\n"))
        .collect::<String>();
    fs::write(f.root.join("code.py"), &source).unwrap();
    let (_, full) = f.build("full_span", 64000);
    let item = &full["data"]["items"][0];
    let end = item["range"]["end_byte"].as_u64().unwrap() as usize;
    assert_eq!(item["content"], &source[..end]);
    assert_eq!(item["range"]["end_line"], 80);
    assert_eq!(item["body_omitted"], true);
    assert_eq!(item["completeness"], "partial");
    let (_, fallback) = f.build("signature", 64000);
    assert_eq!(fallback["data"]["items"][0]["representation"], "outline");
    assert_eq!(
        fallback["data"]["items"][0]["fallback"],
        "signature_unsupported_outline"
    );
}

#[test]
fn final_omissions_fit_without_cutting_mandatory_task_rules_or_decisions() {
    let f = Fixture::new();
    fs::write(f.root.join("code.py"), "def example():\n    return 1\n").unwrap();
    fs::create_dir_all(f.root.join(".pctx/rules")).unwrap();
    let rule = "---\nschema_version: 1\nid: must\nscope: ['**']\nrequired: true\n---\nKeep the required invariant intact.\n";
    fs::write(f.root.join(".pctx/rules/must.md"), rule).unwrap();
    fs::create_dir_all(f.root.join(".pctx/decisions")).unwrap();
    fs::write(f.root.join(".pctx/decisions/block.md"),
        "---\nschema_version: 1\nid: block\nstatus: accepted\ndate: '2026-10-08'\nscope: ['**']\n---\nPreserve the decision claim; this grants no approval.\n").unwrap();
    let (_, reference) = f.build("reference", 12000);
    let used = reference["data"]["budget"]["used"].as_u64().unwrap() as usize;
    // Remove enough capacity to force omission, while retaining mandatory content.
    let (_, small) = f.build("adaptive", used - 100);
    assert_eq!(small["data"]["task"], "inspect code");
    assert_eq!(small["data"]["items"].as_array().unwrap().len(), 2);
    assert!(
        small["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["required"] == true)
    );
    assert!(
        small["data"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["content"]
                .as_str()
                .is_some_and(|text| text.contains("Preserve the decision claim")))
    );
    assert_eq!(small["data"]["items"][0]["required"], true);
    assert!(
        small["data"]["items"][0]["content"]
            .as_str()
            .unwrap()
            .contains("Keep the required invariant intact.")
    );
    assert_eq!(small["data"]["omitted_count"], 1);
    assert_eq!(small["data"]["selection_complete"], false);
    let out = f.run(
        &[
            "build",
            "--task",
            "inspect code",
            "--seed",
            "code.py",
            "--budget-bytes",
            &(used - 100).to_string(),
            "--require-complete",
        ],
        "json",
    );
    assert_eq!(
        out.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    fs::write(
        f.root.join(".pctx/rules/must.md"),
        format!("{rule}{}", "required ".repeat(3000)),
    )
    .unwrap();
    let out = f.run(
        &[
            "build",
            "--task",
            "inspect code",
            "--seed",
            "code.py",
            "--budget-bytes",
            "3000",
        ],
        "json",
    );
    assert_eq!(out.status.code(), Some(8));
}

#[test]
fn final_escaped_document_budget_and_unsupported_tokenizer_remain_truthful() {
    let f = Fixture::new();
    fs::write(
        f.root.join("code.py"),
        "# directional \u{202e} and controls \u{0085}\ndef f():\n    return 1\n",
    )
    .unwrap();
    let (out, _) = f.build("adaptive", 12000);
    assert!(String::from_utf8_lossy(&out.stdout).contains("\\u202e"));
    let out = f.run(
        &[
            "build",
            "--task",
            "inspect code",
            "--budget-tokens",
            "500",
            "--tokenizer",
            "unknown",
        ],
        "json",
    );
    assert_eq!(out.status.code(), Some(6));
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["errors"][0]["code"], "CAPABILITY_UNAVAILABLE");
}

#[test]
fn explicit_seed_survives_lower_priority_imports_under_tight_budget() {
    let f = Fixture::new();
    fs::write(
        f.root.join("z-main.js"),
        format!(
            "import './a-dependency.js';\nfunction main() {{\n{}\n}}\n",
            "  // explicit body\n".repeat(75)
        ),
    )
    .unwrap();
    fs::write(f.root.join("a-dependency.js"), format!("export function dependency(\n long_argument_name,\n other_long_argument\n) {{\n{}\n}}\n", "  // imported body\n".repeat(75))).unwrap();
    let baseline = f.ok(
        &[
            "build",
            "--task",
            "inspect imports",
            "--seed",
            "z-main.js",
            "--dependency-depth",
            "1",
            "--detail",
            "reference",
            "--budget-bytes",
            "64000",
        ],
        "json",
    );
    let v: Value = serde_json::from_slice(&baseline.stdout).unwrap();
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 2);
    let budget = v["data"]["budget"]["used"].as_u64().unwrap() as usize - 100;
    let out = f.ok(
        &[
            "build",
            "--task",
            "inspect imports",
            "--seed",
            "z-main.js",
            "--dependency-depth",
            "1",
            "--detail",
            "adaptive",
            "--budget-bytes",
            &budget.to_string(),
        ],
        "json",
    );
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["data"]["budget"]["used"], out.stdout.len());
    assert!(out.stdout.len() <= budget);
    assert_eq!(v["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(v["data"]["items"][0]["path"], "z-main.js");
    assert_eq!(v["data"]["items"][0]["reason"], "explicit_seed");
    assert_eq!(v["data"]["omitted_count"], 1);
}

#[test]
fn markdown_final_document_uses_the_same_exact_accounting() {
    let f = Fixture::new();
    fs::write(
        f.root.join("code.py"),
        "def calculate(\n    value: int,\n):\n    return value\n",
    )
    .unwrap();
    let args = [
        "build",
        "--task",
        "inspect code",
        "--seed",
        "code.py",
        "--detail",
        "signature",
        "--budget-bytes",
        "24000",
    ];
    let out = f.ok(&args, "markdown");
    let text = String::from_utf8(out.stdout.clone()).unwrap();
    let appendix = text.split("## Complete envelope metadata").last().unwrap();
    let start = appendix.find("{\n").unwrap();
    let end = appendix.rfind("\n}").unwrap() + 2;
    let envelope: Value = serde_json::from_str(&appendix[start..end]).unwrap();
    assert_eq!(envelope["data"]["budget"]["used"], out.stdout.len());
    assert_eq!(envelope["data"]["items"][0]["representation"], "signature");
    assert!(text.contains("value: int"));
    assert!(out.stdout.len() <= 24000);
}

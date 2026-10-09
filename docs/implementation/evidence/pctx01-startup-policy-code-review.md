# PCTX01-G05-D05 code and attribution review

Review baseline: e19035a. Product/test SHA256 values are retained in the forward
verification and preparation manifests. Product and original tests were not edited.
Independent reviewer: /root/work_control; no material blocker in hook/wrapper,
static queue decoding, or historical16-record join.

- `tests/project_deadline.rs:145`: fixture write closes before chmod; temporary
  directory lives through all16 queries in each worker (574). No observed
  writer/deletion race explains pre-payload zero streams.
- `src/query_process.rs:554,629`: captured absolute executable/cwd, explicit
  environment, null stdin, separate streams, standard process group and native
  spawn. No custom suspended/pre-exec path. Parent nonblocking flags affect
  read ends; six-byte writes do not explain zero-user-syscall pre-payload wait.
- One original deadline is preserved. Cancellation follows expiry and owned
  identity checks; no remaining source defect explains the startup wait.
  This is scoped review evidence, not proof that the product is bug-free.
- Installed service `_workQueue` offset0x48 is initialized by a serial dispatch
  helper. Its block synchronously calls `performScanWithArguments:withCodeEvaluation:`.
  Execution-evaluation/completion queues are concurrent; no worker-count claim.
- `redactedDescription` uses path only when volumeUUID exists and objectID>=2;
  fallback hashes formatted volume/object IDs. Forward raw/canonical strings
  refer to the same owned inode and are hashed only after original cleanup.
- Historical16 matches are unique only within generated candidate family,
  parent1..100000/two aliases. Historical TMPDIR not independently saved;
  hashes noncryptographic. Original-child attribution remains pending.

Pure NSString/CFHash vectors and Rust API control pass. The live own-process
observer control reports signposts enabled but retains only four normal log
messages; startup readiness/retention is unknown. The initial helper compilation
failed without SDK path; explicit SDK compilation passed. Neither result proves
service signpost availability or a product fix.

One forward original concurrency run fails:112 OUTPUT/1TIMEOUT,15 censored,
no dropped records, all known parent/children absent. Two clock endpoint ranges
overlap; intermediate wall-clock step remains possible. Post-reap instrumentation
can affect later iterations. Acceptance must rebuild original artifacts and
satisfy all original tests/platforms; no replay-to-pass, bypass or budget reduction.

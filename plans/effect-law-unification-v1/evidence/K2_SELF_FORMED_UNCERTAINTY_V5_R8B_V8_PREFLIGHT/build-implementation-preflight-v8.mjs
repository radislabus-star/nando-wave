import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, "../../../..");
const plans = path.join(repo, "plans/effect-law-unification-v1");
const v7Dir = path.join(
  plans,
  "evidence/K2_SELF_FORMED_UNCERTAINTY_V5_R8B_V7_PREFLIGHT",
);
const v7Path = path.join(v7Dir, "implementation-preflight.v7.json");
const inventoryPath = path.join(here, "implementation-inventory.v8.json");
const machinePath = path.join(here, "machine-cardinality-baseline.v8.json");
const outputPath = path.join(here, "implementation-preflight.v8.json");

const readJson = (file) => JSON.parse(fs.readFileSync(file, "utf8"));
const sha256 = (file) =>
  crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex");
const slash = (value) => value.split(path.sep).join("/");
const relativeToManifest = (file) => slash(path.relative(here, file));
const modeOf = (file) =>
  (fs.lstatSync(file).mode & 0o7777).toString(8).padStart(4, "0");

const manifest = structuredClone(readJson(v7Path));
const inventory = readJson(inventoryPath);
const machine = readJson(machinePath);
const donor = inventory.source.donor_worktree;

for (const entry of manifest.baseline_checks) {
  if (!path.isAbsolute(entry.path) && !entry.path.startsWith("../../../../")) {
    entry.path = relativeToManifest(path.join(v7Dir, entry.path));
  }
}
for (const entry of manifest.preserved_artifacts) {
  if (!path.isAbsolute(entry.path) && !entry.path.startsWith("../../../../")) {
    entry.path = relativeToManifest(path.join(v7Dir, entry.path));
  }
}

manifest.task_id = "k2-self-formed-uncertainty-r8b-v8";
manifest.authority_bearing = true;
manifest.scientific_future_sensitive = true;
manifest.reuses_existing_implementation = true;

const baselineIdsByPath = new Map(
  manifest.baseline_checks.map((entry) => [entry.path, entry.id]),
);

function addFileBaseline(id, file, manifestPath = relativeToManifest(file)) {
  if (!fs.statSync(file).isFile()) {
    throw new Error(`baseline is not a regular file: ${file}`);
  }
  const entry = {
    id,
    kind: "file",
    path: manifestPath,
    expect: {
      size_bytes: fs.statSync(file).size,
      sha256: sha256(file),
      mode: modeOf(file),
    },
  };
  manifest.baseline_checks.push(entry);
  baselineIdsByPath.set(manifestPath, id);
  return entry;
}

function addAbsentBaseline(id, file, manifestPath = file) {
  if (
    fs.existsSync(file) ||
    (fs.existsSync(path.dirname(file)) &&
      fs.lstatSync(path.dirname(file)).isSymbolicLink())
  ) {
    throw new Error(`absent baseline exists: ${file}`);
  }
  const entry = { id, kind: "absent", path: manifestPath, expect: {} };
  manifest.baseline_checks.push(entry);
  baselineIdsByPath.set(manifestPath, id);
  return entry;
}

function addPreserved(id, baseline, policy, rollbackPolicy, extra = {}) {
  manifest.preserved_artifacts.push({
    id,
    path: baseline.path,
    baseline_check_id: baseline.id,
    policy,
    rollback_policy: rollbackPolicy,
    ...extra,
  });
}

const extraPaperFiles = [
  path.join(plans, "K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8.md"),
  path.join(plans, "K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8_CRITIQUE_V1.md"),
  path.join(plans, "K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8_CRITIQUE_V2.md"),
  path.join(plans, "K2_SELF_FORMED_UNCERTAINTY_V5_R8B_CONTRACT_V8_CRITIQUE_V3.md"),
  path.join(
    plans,
    "K2_SELF_FORMED_UNCERTAINTY_V5_R8B_V7_IMPLEMENTATION_PREFLIGHT_CRITIQUE_V2.md",
  ),
  v7Path,
  path.join(v7Dir, "implementation-preflight.v7.receipt.json"),
  ...fs
    .readdirSync(here)
    .filter(
      (name) =>
        name !== path.basename(outputPath) &&
        name !== "implementation-preflight.v8.receipt.json" &&
        name !== path.basename(fileURLToPath(import.meta.url)),
    )
    .map((name) => path.join(here, name))
    .filter((file) => fs.statSync(file).isFile()),
];

for (const [index, file] of [...new Set(extraPaperFiles)].entries()) {
  const baseline = addFileBaseline(
    `baseline-v8-paper-${String(index + 1).padStart(2, "0")}`,
    file,
  );
  addPreserved(
    `preserved-v8-paper-${String(index + 1).padStart(2, "0")}`,
    baseline,
    "immutable_bytes",
    "never overwrite, delete, relabel, or replace retained paper and gate evidence",
  );
}

for (const [index, entry] of inventory.dirty_donor_paths.entries()) {
  const file = path.join(donor, entry.path);
  const actual = {
    sha256: sha256(file),
    size_bytes: fs.statSync(file).size,
    mode: modeOf(file),
  };
  if (
    actual.sha256 !== entry.worktree_sha256 ||
    actual.size_bytes !== entry.worktree_size_bytes ||
    actual.mode !== entry.worktree_mode
  ) {
    throw new Error(`dirty donor drift: ${entry.path}`);
  }
  addFileBaseline(
    `baseline-v8-donor-${String(index + 1).padStart(2, "0")}`,
    file,
    file,
  );
}

for (const [index, entry] of inventory.planned_new_paths.entries()) {
  const file = path.join(donor, entry.path);
  if (fs.existsSync(file)) {
    throw new Error(`planned V8 path is no longer absent: ${entry.path}`);
  }
  addAbsentBaseline(
    `baseline-v8-new-${String(index + 1).padStart(2, "0")}`,
    file,
  );
}

for (const [index, tool] of machine.tool_dependencies.entries()) {
  const existingId = baselineIdsByPath.get(tool.path);
  if (existingId) {
    const existing = manifest.baseline_checks.find((entry) => entry.id === existingId);
    if (
      existing.expect.sha256 !== tool.sha256 ||
      existing.expect.size_bytes !== tool.size_bytes ||
      existing.expect.mode !== tool.mode
    ) {
      throw new Error(`existing tool baseline drift: ${tool.path}`);
    }
    continue;
  }
  const baseline = addFileBaseline(
    `baseline-v8-tool-${String(index + 1).padStart(2, "0")}`,
    tool.path,
    tool.path,
  );
  if (
    baseline.expect.sha256 !== tool.sha256 ||
    baseline.expect.size_bytes !== tool.size_bytes ||
    baseline.expect.mode !== tool.mode
  ) {
    throw new Error(`tool baseline drift: ${tool.path}`);
  }
  addPreserved(
    `preserved-v8-tool-${String(index + 1).padStart(2, "0")}`,
    baseline,
    "external_dependency_exact_bytes",
    "abort before execution on path, mode, length, version, or SHA-256 drift",
  );
}

const naturalSuffixModel = path.join(here, "runtime-ledger-natural-prefix.model.absent");
const naturalSuffixBaseline = addAbsentBaseline(
  "baseline-v8-runtime-ledger-natural-prefix",
  naturalSuffixModel,
  path.basename(naturalSuffixModel),
);
addPreserved(
  "preserved-v8-runtime-ledger-natural-prefix",
  naturalSuffixBaseline,
  "append_only_suffix",
  "preserve_existing_and_natural_suffix",
  { cursor_policy: "freeze_existing_records_only" },
);

const v8ForbiddenPatterns = [
  {
    effect: "r8b_suite_execution_without_separate_authority",
    regex: "execute_r8b_(suite|route)_without_(separate_)?authorization",
  },
  {
    effect: "m24_m26_execution_during_implementation",
    regex: "run_m2[4-6]_during_implementation",
  },
  {
    effect: "p09_execution_without_separate_authority",
    regex: "execute_p09_without_(separate_)?authorization",
  },
  {
    effect: "private_truth_content_inspection_by_m24_or_m25",
    regex: "(m24|m25|authorizer).{0,80}(read|hash|decode|serialize)_private_truth",
  },
  {
    effect: "transient_capability_execution_during_implementation",
    regex: "execute_r8b_transient_capability_during_implementation",
  },
  { effect: "git_push", regex: "Command::new\\(\"git\"\\).{0,80}arg\\(\"push\"\\)" },
  {
    effect: "direct_cgroupfs_child_creation",
    regex: "(execute|allow)_direct_cgroupfs_r8b_child",
  },
  {
    effect: "false_direct_m24_child_ownership",
    regex: "m24_(directly_)?(forked|spawned)_m24_child",
  },
  {
    effect: "caller_supplied_privileged_program_or_path",
    regex: "caller_supplied_privileged_(program|path|argv)",
  },
  {
    effect: "shell_path_or_environment_privileged_probe",
    regex: "(shell|path|environment)_derived_privileged_probe",
  },
  {
    effect: "manager_identity_without_live_image",
    regex: "accept_manager_(version|path|unit)_only",
  },
  {
    effect: "whole_ledger_protocol_deserialization",
    regex: "deserialize_complete_process_ledger",
  },
  { effect: "process_ledger_evidence_kind_20", regex: "EvidenceKind::ProcessLedger" },
  {
    effect: "generic_schema_root_extraction",
    regex: "extract_(any|generic)_schema_and_root",
  },
  {
    effect: "cross_device_ledger_freeze",
    regex: "allow_cross_device_ledger_freeze",
  },
  {
    effect: "open_journal_or_lock_in_packet",
    regex: "include_(open_)?(journal|ledger_lock)_in_packet",
  },
  {
    effect: "packet_manifest_embeds_process_ledger",
    regex: "packet_manifest.{0,80}embedded_(process_)?ledger",
  },
  {
    effect: "unbound_environment_input_authority",
    regex: "trust_unbound_environment_(seed|fixture|manifest|ledger|request)",
  },
  {
    effect: "external_bin_kill_transport",
    regex: "spawn_unbound_external_(bin_)?kill",
  },
  {
    effect: "producer_exit_code_as_evidence",
    regex: "trust_producer_exit_code_as_evidence",
  },
  {
    effect: "c08_self_derived_from_ledger",
    regex: "derive_c08_from_(observed_)?ledger",
  },
  {
    effect: "resource_metrics_after_unit_unload",
    regex: "read_resource_metrics_after_unit_(unload|disappear)",
  },
  {
    effect: "broad_systemd_stop_target",
    regex: "stop_(user_manager|slice|service_collection)",
  },
];

const donorCommonPatterns = [
  {
    effect: "experiment_network_access",
    regex: "(TcpStream|UdpSocket|reqwest::|hyper::Client)",
  },
  { effect: "production_write", regex: "(/var/lib/nando-wave|/etc/nando-wave)" },
  { effect: "dashboard_change", regex: "nando_gateway_control.{0,80}live_dashboard" },
  { effect: "deployment", regex: "(deploy_remote|deployment_receipt)" },
];

for (const [index, entry] of inventory.dirty_donor_paths.entries()) {
  const file = path.join(donor, entry.path);
  const patterns = index === 0
    ? [...donorCommonPatterns, ...v8ForbiddenPatterns]
    : donorCommonPatterns;
  manifest.source_checks.push({
    id: `v8-dirty-donor-source-${String(index + 1).padStart(2, "0")}`,
    path: file,
    sha256: entry.worktree_sha256,
    forbidden_effect_patterns: patterns,
  });
}

const planned = manifest.side_effects.planned.filter(
  (entry) =>
    ![
      "modify-seven-frozen-scope-files",
      "add-sixteen-frozen-scope-files",
      "add-libc-direct-edge",
      "run-child-route-in-fresh-delegated-cgroup",
    ].includes(entry.id),
);
planned.unshift(
  { id: "modify-exact-23-measured-donor-paths", kind: "scoped_source_modify" },
  { id: "add-exact-five-v8-ownership-modules", kind: "scoped_source_add" },
  { id: "retain-existing-libc-direct-edge", kind: "dependency_edge_preserve" },
  { id: "add-rustix-1-1-4-direct-edge", kind: "dependency_edge_add" },
  { id: "checkpoint-v8-c1-move-only-extraction", kind: "move_only_refactor" },
  { id: "checkpoint-v8-c3-behavior-repair", kind: "scoped_behavior_repair" },
);
planned.push(
  {
    id: "observe-user-manager-two-channel-identity",
    kind: "process_identity_observation",
    comparison_id: "user-manager-stable-projection",
  },
  {
    id: "run-two-fixed-read-only-manager-image-probes",
    kind: "privileged_read_only_observation",
    comparison_id: "user-manager-stable-projection",
  },
  {
    id: "submit-one-bound-user-systemd-transient-service",
    kind: "test_delegated_service_execution",
    comparison_id: "transient-unit-resource-projection",
  },
  {
    id: "stop-only-exact-route-derived-test-unit",
    kind: "test_exact_unit_stop",
    comparison_id: "transient-unit-resource-projection",
  },
  { id: "append-bounded-canonical-process-ledger", kind: "test_journal_write" },
  { id: "freeze-closed-23-file-p06-packet", kind: "test_packet_publication" },
  { id: "stream-validate-process-ledger", kind: "test_streaming_validation" },
);
manifest.side_effects.planned = planned;
manifest.side_effects.forbidden = [
  ...new Set([
    ...manifest.side_effects.forbidden,
    ...v8ForbiddenPatterns.map((entry) => entry.effect),
  ]),
];

const newTests = [
  ["r8b-v8-move-only-fault", "fault_injection", "V8-C1 failure retains the exact donor baseline and all V7/V8 paper evidence, creates no semantic repair, and starts zero R8B processes"],
  ["r8b-v8-move-only-parity", "parity", "V8-C2 proves move-only symbol and byte-contract parity before any V8 behavior repair"],
  ["r8b-v8-behavior-repair-fault", "fault_injection", "V8-C3 failure remains inside the exact 28-path scope, preserves the V8-C2 checkpoint and grants no execution authority"],
  ["r8b-v8-donor-baseline-parity", "parity", "all 23 dirty donor worktree bytes, modes, index blobs, worktree blobs, porcelain-v2 NUL hash and index tree equal implementation-inventory.v8.json; all five ownership paths are absent"],
  ["r8b-v8-n17-delegated-ownership", "fault_injection", "N17 rejects direct cgroupfs mutation and any claim that M24 directly forked or spawned the delegated child"],
  ["r8b-v8-n18-unit-and-tool-drift", "fault_injection", "N18 rejects unit collision, normalized systemd property drift, or substitution of any bound tool"],
  ["r8b-v8-n19-input-binding", "fault_injection", "N19 rejects missing, changed, extra, wrong-mode, wrong-root, or wrong-object producer input before the first write"],
  ["r8b-v8-n20-output-contract", "fault_injection", "N20 rejects a matching path with unequal object role, evidence kind, schema, denominator, source roots, validator, or producer identity"],
  ["r8b-v8-n21-plan-substitution", "fault_injection", "N21 rejects child-role, writer-partition, launch-kind, tool-chain, expected-outcome, or schedule-plan substitution"],
  ["r8b-v8-n22-diagnostic-authority", "fault_injection", "N22 rejects every DiagnosticExpectedFailure or other non-AuthoritySuccess completion carrying an authority descriptor"],
  ["r8b-v8-n23-typed-validator", "fault_injection", "N23 rejects generic schema/root extraction and requires concrete validate plus canonical decode-reserialize equality"],
  ["r8b-v8-n24-s02-m02-ownership", "fault_injection", "N24 rejects direct setup M02 without the S02 pair and nested M02 without its exact M01 request-owner pair"],
  ["r8b-v8-n25-tool-chain-order", "fault_injection", "N25 rejects omitted, extra, substituted, or reordered strace, bwrap, prlimit, systemd-run, sudo, or sha256sum chains"],
  ["r8b-v8-n26-ledger-bounds", "fault_injection", "N26 rejects incomplete, malformed, oversized, over-cardinality, stale-tail, non-fail-stop, or unterminated process ledgers"],
  ["r8b-v8-n27-streaming-memory", "fault_injection", "N27 proves the 128 MiB maximum ledger is streamed within the frozen memory bound and rejects any whole-ledger protocol deserialization path"],
  ["r8b-v8-n28-packet-census", "fault_injection", "N28 rejects embedded ledger bytes, evidence kind 20, or any packet census other than 19 evidence plus C08, resource, ledger and manifest"],
  ["r8b-v8-n29-m16-dual-roots", "fault_injection", "N29 rejects M16 event-root or receipt-root subset, superset, duplicate, foreign, relabelled, swapped, or partition-mismatched sets"],
  ["r8b-v8-n30-m17-dual-roots", "fault_injection", "N30 rejects M17 event-root or receipt-root subset, superset, duplicate, foreign, relabelled, swapped, or partition-mismatched sets"],
  ["r8b-v8-n31-writer-schedule", "fault_injection", "N31 rejects writer-partition crossing, formula drift, schedule-root drift, case-count drift, or representative-count drift"],
  ["r8b-v8-n32-environment-authority", "fault_injection", "N32 rejects any seed, fixture, manifest, ledger, request, path, or route environment value not already byte-bound by the producer request"],
  ["r8b-v8-n33-resource-before-unload", "fault_injection", "N33 rejects missing or late resource metrics after the transient unit disappears and emits no resource receipt"],
  ["r8b-v8-n34-exact-stop-target", "fault_injection", "N34 rejects every stop target except the exact route-derived test unit and never targets a slice, manager, collection, or production unit"],
  ["r8b-v8-n35-rustix-fd-signal", "fault_injection", "N35 proves rustix inheritable-fd and signal parity and rejects an external /bin/kill transport regression"],
  ["r8b-v8-n36-c08-authority", "fault_injection", "N36 rejects omitted, relabelled, ledger-derived, post-C09, extra-scope, or ledger-unequal C08 authority"],
  ["r8b-v8-n37-manager-continuity", "fault_injection", "N37 rejects bus PID/name, pidfd, boot/start, UID, command, cgroup, unit, InvocationID, version, or live-image drift across the delegated route"],
  ["r8b-v8-n38-systemd-command", "fault_injection", "N38 rejects --pipe, --wait, --collect, shell, PTY, environment expansion, credential substitution, output-path drift, or property drift"],
  ["r8b-v8-n39-ledger-freeze", "fault_injection", "N39 rejects an open journal or lock in P06, non-terminal ledger, live writer, replacement rename, or cross-device freeze"],
  ["r8b-v8-n40-request-field-bounds", "fault_injection", "N40 rejects oversized path/schema/fact/descriptor fields and unknown enum variants before the first ledger event"],
  ["r8b-v8-n41-c08-projection", "fault_injection", "N41 rejects use of C08 outside the exact C09-C20 downstream projection"],
  ["r8b-v8-n42-live-image-required", "fault_injection", "N42 rejects version, path, command, package, unit, or on-disk hash identity without both exact live-image probes"],
  ["r8b-v8-n43-privileged-probe", "fault_injection", "N43 rejects PID/path/argv/tool drift, shell or PATH use, prompt, nonzero status, stderr, extra output, malformed NUL framing, or unequal live hash"],
  ["r8b-v8-wrapper-schema-parity", "parity", "all V8 producer-request, process-event, ledger-seal, packet-descriptor and resource schemas preserve frozen scientific payload projections and exact denominators"],
  ["r8b-v8-launch-route-parity", "parity", "direct, strace, bwrap-prlimit and user-systemd launch routes bind request owner, physical tools, target process and outcome without transferring evidence authority to tools"],
  ["r8b-v8-manager-identity-parity", "parity", "pre and post unprivileged manager projections and fixed privileged live-image probes converge on one unchanged /usr/lib/systemd/systemd --user process and pinned disk image"],
  ["r8b-v8-manager-live-image-parity", "parity", "the two exact privileged probe outputs parse to one unchanged live-image hash equal to the pinned on-disk systemd hash"],
  ["r8b-v8-producer-input-parity", "parity", "every producer sees exactly the request-bound seed, fixture tree, executable manifests, route ledger and exclusive output directory before its first write"],
  ["r8b-v8-invocation-projection-parity", "parity", "producer plans, schedule grammar plus M04 facts, and C08 independently equal their three disjoint observed ledger projections"],
  ["r8b-v8-c08-projection-parity", "parity", "C08 is frozen before C09 and equals only the exact C09-C20 observed projection without absorbing any other writer partition"],
  ["r8b-v8-p06-census-parity", "parity", "P06 contains exactly nineteen evidence objects plus C08, resource receipt, process ledger and packet manifest with one-to-one descriptors"],
  ["r8b-v8-rustix-fd-signal-parity", "parity", "rustix inheritable-fd and signal adapters preserve the predecessor observable process behavior with no external kill process"],
  ["r8b-v8-validator-output-parity", "parity", "each producer expected-output row selects one concrete validator whose canonical bytes, role, evidence kind, denominator and source roots equal the retained output descriptor"],
  ["r8b-v8-streaming-reference-parity", "parity", "bounded fixture ledgers produce the same event-chain root under streaming and reference in-memory validators while the production authorizer remains streaming-only"],
  ["r8b-v8-m16-m17-dual-root-parity", "parity", "M25 independently reconstructs and exactly equals the sorted unique M16 and M17 completion-event and receipt-semantic-root sets"],
  ["r8b-v8-zero-authority-parity", "parity", "implementation and ordinary tests execute no R8B suite, M24-M26, P09, private-truth inspection, sealed attempt, production mutation, deployment, dashboard mutation, or push"],
  ["r8b-v8-transient-capability-parity", "parity", "without launching an R8B attempt, capability and normalized-command tests prove the exact user-systemd properties, credential route, fixed probe argv and exact-unit stop target"],
];

const testsById = new Map(manifest.tests.map((entry) => [entry.id, entry]));
for (const [id, kind, expected] of newTests) {
  testsById.set(id, { id, kind, expected });
}
manifest.tests = [...testsById.values()];

const updateTest = (id, expected) => {
  const test = manifest.tests.find((entry) => entry.id === id);
  if (!test) throw new Error(`missing inherited test: ${id}`);
  test.expected = expected;
};
updateTest(
  "r8b-implementation-fault",
  "partial implementation stays inside the exact 23 measured donor paths plus five absent ownership modules, retains every V6/V7/V8 paper artifact, touches no production state, and starts zero R8B executions",
);
updateTest(
  "r8b-source-scope-parity",
  "implementation diff touches only the exact 23 measured dirty donor paths plus five ownership modules frozen by implementation-inventory.v8.json and respects every V8 line budget",
);
updateTest(
  "r8b-code-route-parity",
  "postimplementation observed-source graph retains the V8 request-owner, physical-tool, execution, observation, proof and authority topology without role collapse",
);
updateTest(
  "r8b-parent-child-launch-fault",
  "P00-P03 failure retains diagnostics and grants no child, packet, authorization or publication; direct cgroupfs ownership is never claimed",
);
updateTest(
  "r8b-resource-finalization-fault",
  "missing loaded-unit terminal metrics, manager continuity, exact stop success, inactive state, or empty descendant set prevents P06",
);
updateTest(
  "r8b-resource-cgroup",
  "the exact delegated M24-child user service contains child C00-C22 and descendants under 512 MiB, zero swap/OOM/network and 20 minutes; suites, M24 parent, M25, M26 and P09 remain separate denominators",
);
updateTest(
  "r8b-manifest-tool-binding",
  "all 26 linked executables, five suite producers and eight V8 tools match frozen canonical path, mode, length, version where frozen and SHA-256 before any separately authorized execution",
);
updateTest(
  "r8b-linked-route-parity",
  "one fresh route follows P00-P08 and C00-C22 through the exact user-systemd ownership chain, streaming ledger and 23-file P06 packet; P09 remains separately gated diagnostics",
);
updateTest(
  "r8b-runner-invocation-parity",
  "M24 is one request owner while systemd-run, the authenticated user manager and the service main process remain distinct physical ownership observations with no evidence authority transfer",
);
updateTest(
  "r8b-process-ledger-parity",
  "all requested Nando invocations have one bounded requested event and one allowed completion; expected schedules come from producer plans, M04 facts and C08 rather than the observed ledger itself",
);

const preservedIds = () => manifest.preserved_artifacts.map((entry) => entry.id);
const step = (id, from, to, mutates, failureState, cleanup, testId) => ({
  id,
  from,
  to,
  mutates,
  failure_state: failureState,
  cleanup,
  ...(mutates ? { preserves: preservedIds() } : {}),
  test_id: testId,
});

manifest.state_machine = {
  initial_state: "V8_PREFLIGHT_READY",
  terminal_states: [
    "MOVE_ONLY_IMPLEMENTATION_FAILED",
    "MOVE_ONLY_PARITY_FAILED",
    "BEHAVIOR_REPAIR_FAILED",
    "R8B_EXECUTION_NOT_AUTHORIZED",
    "PREEXECUTION_VALIDATION_FAILED",
    "SUITE_RECEIPT_FAILED",
    "PRODUCTION_PREOBSERVATION_FAILED",
    "MANAGER_PREBIND_FAILED",
    "DELEGATED_SUBMISSION_FAILED",
    "CHILD_REQUEST_FAILED",
    "ATTEMPT_INITIALIZATION_INDETERMINATE",
    "GENERATOR_RESULT_INDETERMINATE",
    "SPLIT_PUBLICATION_FAILED",
    "CASES_APPEND_FAILED",
    "OWNER_PUBLICATION_FAILED",
    "DOWNSTREAM_CONTRACT_FAILED",
    "LINKED_ROUTE_FAILED",
    "CLEANUP_AUTHORIZATION_FAILED",
    "CLEANUP_INCOMPLETE",
    "CLEANUP_VERIFICATION_FAILED",
    "CHILD_CANDIDATE_FAILED",
    "UNIT_RESOURCE_FINALIZATION_FAILED",
    "MANAGER_POSTBIND_FAILED",
    "EXACT_UNIT_STOP_FAILED",
    "PRODUCTION_OBSERVATION_FAILED",
    "LEDGER_SEAL_FAILED",
    "AGGREGATE_PACKET_FAILED",
    "AGGREGATE_AUTHORIZATION_FAILED",
    "R8B_PUBLICATION_FAILED",
    "P09_NOT_AUTHORIZED",
    "R8B_FROZEN_AUDITED",
    "R8B_FROZEN_AUDIT_INCOMPLETE",
  ],
  steps: [
    step("extract-v8-move-only-ownership-modules", "V8_PREFLIGHT_READY", "MOVE_ONLY_EXTRACTED", true, "MOVE_ONLY_IMPLEMENTATION_FAILED", "retain exact donor baseline, partial move diff and all paper evidence; execute no R8B path", "r8b-v8-move-only-fault"),
    step("verify-v8-move-only-parity", "MOVE_ONLY_EXTRACTED", "MOVE_ONLY_PARITY_PASS", false, "MOVE_ONLY_PARITY_FAILED", "retain the move-only checkpoint and begin no behavior repair until parity passes", "r8b-v8-move-only-parity"),
    step("implement-v8-behavior-repair", "MOVE_ONLY_PARITY_PASS", "IMPLEMENTATION_READY", true, "BEHAVIOR_REPAIR_FAILED", "retain the V8-C2 checkpoint, exact partial repair diff and paper evidence; grant no execution authority", "r8b-v8-behavior-repair-fault"),
    step("require-separate-r8b-execution-authorization", "IMPLEMENTATION_READY", "R8B_EXECUTION_AUTHORIZED", false, "R8B_EXECUTION_NOT_AUTHORIZED", "terminate with zero suite, M24-M26, P09 or transient-service launches", "r8b-execution-boundary"),
    step("validate-donor-input-tool-and-manifest-baselines", "R8B_EXECUTION_AUTHORIZED", "PREEXECUTION_VALIDATED", false, "PREEXECUTION_VALIDATION_FAILED", "write nothing and retain exact diagnostics", "r8b-v8-donor-baseline-parity"),
    step("launch-suite-producers-and-close-channels", "PREEXECUTION_VALIDATED", "SUITE_RECEIPTS_DURABLE", true, "SUITE_RECEIPT_FAILED", "retain valid natural ledger/channel prefixes; request no later process", "r8b-canonical-receipt-channel-negative"),
    step("snapshot-production-before-child", "SUITE_RECEIPTS_DURABLE", "PRODUCTION_PRE_DURABLE", true, "PRODUCTION_PREOBSERVATION_FAILED", "retain read-only observations and perform no service action", "r8b-production-observation-fault"),
    step("bind-user-manager-and-live-image-before-submit", "PRODUCTION_PRE_DURABLE", "MANAGER_PREBOUND", true, "MANAGER_PREBIND_FAILED", "retain exact read-only probe diagnostics; submit no transient unit", "r8b-v8-n42-live-image-required"),
    step("freeze-request-and-submit-delegated-user-service", "MANAGER_PREBOUND", "CHILD_SERVICE_SUBMITTED", true, "DELEGATED_SUBMISSION_FAILED", "retain request, credential, stdout/stderr and unit diagnostics; never retry or claim direct M24 ownership", "r8b-v8-n18-unit-and-tool-drift"),
    step("validate-systemd-credential-and-producer-request", "CHILD_SERVICE_SUBMITTED", "CHILD_REQUEST_VALIDATED", true, "CHILD_REQUEST_FAILED", "retain diagnostics and request no C01 or ledger event", "r8b-v8-n19-input-binding"),
    step("initialize-development-attempt-in-child", "CHILD_REQUEST_VALIDATED", "ARTIFACTS_FROZEN", true, "ATTEMPT_INITIALIZATION_INDETERMINATE", "retain complete path census and dispatch zero generator calls", "r8b-process-restart-p01"),
    step("dispatch-generator-once-in-child", "ARTIFACTS_FROZEN", "GENERATOR_DISPATCHED", true, "GENERATOR_RESULT_INDETERMINATE", "retain the natural journal suffix and never redispatch", "r8b-process-restart-p03"),
    step("publish-development-split-in-child", "GENERATOR_DISPATCHED", "SPLIT_DURABLE", true, "SPLIT_PUBLICATION_FAILED", "remove only unpublished temp objects and retain all immutable published objects", "r8b-development-publication-72"),
    step("append-cases-generated-in-child", "SPLIT_DURABLE", "CASES_GENERATED", true, "CASES_APPEND_FAILED", "retain the complete split and natural journal suffix", "r8b-process-restart-p04"),
    step("publish-development-owner-receipt-in-child", "CASES_GENERATED", "OWNER_DURABLE", true, "OWNER_PUBLICATION_FAILED", "retain generated cases and remove only unpublished owner temp", "r8b-process-restart-p05"),
    step("freeze-c08-downstream-invocation-contract", "OWNER_DURABLE", "C08_DURABLE", true, "DOWNSTREAM_CONTRACT_FAILED", "retain owner evidence and request no C09-C20 invocation", "r8b-v8-n36-c08-authority"),
    step("execute-c09-c14-private-oracle-controls-terminal", "C08_DURABLE", "TERMINAL_DURABLE", true, "LINKED_ROUTE_FAILED", "retain natural process-ledger suffix and completed typed outputs; request no later invocation", "r8b-process-ledger-interruption"),
    step("freeze-cleanup-inputs-and-authorization", "TERMINAL_DURABLE", "CLEANUP_AUTHORIZED", true, "CLEANUP_AUTHORIZATION_FAILED", "retain terminal and cleanup intent evidence; delete nothing", "r8b-cleanup-interruption"),
    step("execute-intent-first-cleanup", "CLEANUP_AUTHORIZED", "CLEANUP_OWNER_DURABLE", true, "CLEANUP_INCOMPLETE", "resume only from exact registry and natural deletion-journal suffix; publish no completion", "r8b-cleanup-interruption"),
    step("verify-cleanup-and-publish-development-completion", "CLEANUP_OWNER_DURABLE", "DEVELOPMENT_COMPLETE", true, "CLEANUP_VERIFICATION_FAILED", "retain owner receipt and post-census; emit no completion", "r8b-cleanup-verification-fault"),
    step("freeze-child-candidate-and-exit-service-main", "DEVELOPMENT_COMPLETE", "CHILD_EXIT_OBSERVED", true, "CHILD_CANDIDATE_FAILED", "remove only candidate temp and retain completed child outputs and exit diagnostics", "r8b-child-candidate-fault"),
    step("freeze-loaded-unit-resource-receipt", "CHILD_EXIT_OBSERVED", "UNIT_RESOURCE_DURABLE", true, "UNIT_RESOURCE_FINALIZATION_FAILED", "retain terminal unit metrics while loaded and authorize no P06 packet", "r8b-v8-n33-resource-before-unload"),
    step("repeat-and-compare-manager-identity-and-live-image", "UNIT_RESOURCE_DURABLE", "MANAGER_POSTBOUND", true, "MANAGER_POSTBIND_FAILED", "retain both identity channels and both raw probe outputs; authorize no P06 packet", "r8b-v8-n37-manager-continuity"),
    step("stop-exact-test-unit-and-prove-no-descendants", "MANAGER_POSTBOUND", "TEST_UNIT_STOPPED", true, "EXACT_UNIT_STOP_FAILED", "retain frozen unit metrics and stop diagnostics; never widen the target or retry a residue", "r8b-v8-n34-exact-stop-target"),
    step("snapshot-production-after-child", "TEST_UNIT_STOPPED", "PRODUCTION_SURVIVAL_DURABLE", true, "PRODUCTION_OBSERVATION_FAILED", "retain read-only snapshots and health traffic; perform no service mutation", "r8b-production-observation-fault"),
    step("seal-and-freeze-process-ledger", "PRODUCTION_SURVIVAL_DURABLE", "PROCESS_LEDGER_DURABLE", true, "LEDGER_SEAL_FAILED", "retain staging lock, directory and complete natural suffix outside P06; never truncate or replace", "r8b-v8-n39-ledger-freeze"),
    step("freeze-closed-23-file-p06-packet", "PROCESS_LEDGER_DURABLE", "AGGREGATE_PACKET_DURABLE", true, "AGGREGATE_PACKET_FAILED", "remove only packet temp; retain every canonical producer output and frozen ledger", "r8b-v8-n28-packet-census"),
    step("authorize-closed-p06-packet-with-m25", "AGGREGATE_PACKET_DURABLE", "AGGREGATE_AUTHORIZED", true, "AGGREGATE_AUTHORIZATION_FAILED", "retain P06 and M25 diagnostics; emit no positive authorization on any failed validation", "r8b-aggregate-packet-fault"),
    step("publish-exact-m25-bytes-with-m26", "AGGREGATE_AUTHORIZED", "R8B_FROZEN", true, "R8B_PUBLICATION_FAILED", "remove only final temp; retain exact M25 bytes and P06 without reinterpretation", "r8b-aggregate-publication-fault"),
    step("require-separate-p09-authorization", "R8B_FROZEN", "P09_AUTHORIZED", false, "P09_NOT_AUTHORIZED", "leave exact M25 and M26 bytes unchanged and append no audit", "r8b-execution-boundary"),
    step("freeze-post-authorization-diagnostic-audit", "P09_AUTHORIZED", "R8B_FROZEN_AUDITED", true, "R8B_FROZEN_AUDIT_INCOMPLETE", "retain exact M25/M26 bytes and any natural audit suffix; never rerun or reinterpret authority", "r8b-post-authorization-audit-fault"),
  ],
};

manifest.runtime_comparisons = [
  ...manifest.runtime_comparisons.map((entry) => ({
    ...entry,
    source: "M24 parent read-only stable projection at P02 and P05 around the delegated child route",
  })),
  {
    id: "user-manager-stable-projection",
    source: "pre-P03B and post-P04A authenticated user-bus, pidfd, procfs, system-unit and fixed privileged live-image observations",
    comparison: "exact_stable_projection",
    reason: "the delegated launch is valid only while one unchanged authenticated user manager owns the bus and exact child unit",
    stable_fields: [
      "boot_id",
      "bus_peer_pid",
      "bus_unique_name",
      "pidfd_identity_and_liveness",
      "proc_start_ticks",
      "uid",
      "command_line",
      "process_cgroup",
      "system_unit",
      "system_unit_main_pid",
      "system_unit_invocation_id",
      "system_unit_exec_start",
      "system_unit_fragment",
      "system_unit_control_group",
      "manager_version",
      "pinned_systemd_sha256",
      "pre_live_image_sha256",
      "post_live_image_sha256",
      "sudo_sha256",
      "sha256sum_sha256"
    ],
    expected_to_change_fields: [
      "pre_probe_monotonic_bounds",
      "post_probe_monotonic_bounds",
      "raw_probe_output_sha256"
    ],
    observed_not_compared_fields: [
      "bus_query_latency",
      "systemctl_query_latency"
    ],
    test_id: "r8b-v8-manager-identity-parity"
  },
  {
    id: "transient-unit-resource-projection",
    source: "the one route-derived user-systemd unit from submission through loaded terminal metrics and exact-unit stop",
    comparison: "exact_stable_projection",
    reason: "dynamic unit state may advance while immutable identity, properties, denominator and terminal resource facts remain bound",
    stable_fields: [
      "unit_name",
      "invocation_id",
      "main_pid_identity",
      "control_group",
      "normalized_systemd_run_argv",
      "credential_sha256",
      "stdout_path",
      "stderr_path",
      "output_directory",
      "memory_max",
      "memory_swap_max",
      "tasks_max",
      "runtime_max_sec",
      "kill_mode",
      "private_network",
      "restrict_address_families",
      "remain_after_exit"
    ],
    expected_to_change_fields: [
      "active_state",
      "sub_state",
      "exec_main_code",
      "exec_main_status",
      "memory_peak",
      "memory_swap_peak",
      "tasks_current",
      "monotonic_timestamps"
    ],
    observed_not_compared_fields: [
      "poll_count",
      "poll_latency",
      "diagnostic_file_mtime"
    ],
    test_id: "r8b-v8-launch-route-parity"
  }
];

manifest.identity_contracts = manifest.identity_contracts.map((entry) => {
  if (entry.id === "runner-parent-child-identity") {
    return {
      ...entry,
      producer: "M24 bound delegated-launch request plus systemd-run submission observation",
      consumer: "authenticated user manager, transient unit and M24-child service process",
      comparison: "request ownership, submission tool, manager launch ownership and service main process remain four distinct identities",
      test_id: "r8b-v8-launch-route-parity",
    };
  }
  if (entry.id === "process-ledger-identity") {
    return {
      ...entry,
      producer: "bounded append-only InvocationRequested/completion event stream and independent C08/producer schedule authorities",
      consumer: "streaming M25 process authorizer",
      comparison: "event chain, limits, terminal seal, writer partitions, schedule reconstruction and descriptor bijection without self-derived expectations",
      test_id: "r8b-v8-streaming-reference-parity",
    };
  }
  if (entry.id === "strace-tool-executable-identity") {
    return {
      ...entry,
      producer: "eight V8 tool baselines in machine-cardinality-baseline.v8.json",
      consumer: "direct, strace, bwrap-prlimit, user-systemd and privileged observation adapters",
      comparison: "canonical path, mode, byte length, SHA-256 and frozen version where applicable",
      test_id: "r8b-manifest-tool-binding",
    };
  }
  return entry;
});

manifest.identity_contracts.push(
  {
    id: "v8-dirty-donor-identity",
    producer: "implementation-inventory.v8.json with 23 worktree/index records and five absent paths",
    consumer: "V8-C1 move-only extraction and final source-scope checker",
    comparison: "worktree bytes, modes, blobs, index blobs, status codes, porcelain-v2 NUL SHA-256, index tree and exact absence",
    test_id: "r8b-v8-donor-baseline-parity",
  },
  {
    id: "v8-manager-unprivileged-identity",
    producer: "authenticated user-bus peer plus pidfd, proc start, boot, UID, command and cgroup",
    consumer: "system-manager user@1000.service projection and delegated-launch request",
    comparison: "one unchanged PID/start/InvocationID/ExecStart/cgroup/version identity before and after child",
    test_id: "r8b-v8-manager-identity-parity",
  },
  {
    id: "v8-manager-live-image-identity",
    producer: "two fixed sudo-rs to uutils sha256sum /proc validated-manager-pid exe probes",
    consumer: "resource receipt and P06 eligibility",
    comparison: "exact normalized argv, tool bytes, zero status, empty stderr, one NUL row and pre hash equals post hash equals pinned disk hash",
    test_id: "r8b-v8-manager-live-image-parity",
  },
  {
    id: "v8-delegated-launch-request-identity",
    producer: "durable P03B launch request and 0400 nlink-1 producer credential",
    consumer: "systemd-run normalized argv, user manager unit properties and C00 child validator",
    comparison: "route, unit, executable, selector, argv, credential, outputs, properties, expected manager and deadline are byte-identical",
    test_id: "r8b-v8-launch-route-parity",
  },
  {
    id: "v8-producer-input-binding-identity",
    producer: "request-bound seed, fixture root, linked/suite manifests, ledger route and exclusive output directory",
    consumer: "S01-S05 and M24-child concrete validators before first write",
    comparison: "canonical path, kind, mode, length, SHA-256/tree root and typed root where applicable; environment is transport only",
    test_id: "r8b-v8-producer-input-parity",
  },
  {
    id: "v8-expected-observed-invocation-identity",
    producer: "producer plans, M10 schedule grammar plus M04 facts, and C08 C09-C20 contract",
    consumer: "streamed observed process ledger projection",
    comparison: "three disjoint expected projections equal observed writer partitions and counts exactly",
    test_id: "r8b-v8-invocation-projection-parity",
  },
  {
    id: "v8-c08-scope-identity",
    producer: "C08 frozen before C09",
    consumer: "only C09-C20 expected downstream projection in M25",
    comparison: "schema, schedule root, roles and counts equal observed C09-C20 and exclude P01, M01/M02 and M03-M10",
    test_id: "r8b-v8-c08-projection-parity",
  },
  {
    id: "v8-role-validator-identity",
    producer: "closed producer expected-output row",
    consumer: "one concrete Rust decoder, validate call and canonical reserializer",
    comparison: "role, optional evidence kind, schema, denominator, source roots, denied authority, bytes and semantic root",
    test_id: "r8b-v8-validator-output-parity",
  },
  {
    id: "v8-m16-dual-set-identity",
    producer: "sixteen AuthoritySuccess M16 completion event roots and receipt semantic roots",
    consumer: "Oracle batch and independently streamed M25 reconstruction",
    comparison: "two exact sorted unique sixteen-root sets with no swap or partition mismatch",
    test_id: "r8b-v8-m16-m17-dual-root-parity",
  },
  {
    id: "v8-m17-dual-set-identity",
    producer: "four AuthoritySuccess M17 completion event roots and receipt semantic roots",
    consumer: "four-scope census and independently streamed M25 reconstruction",
    comparison: "two exact sorted unique four-root sets; census remains one coverage evidence object",
    test_id: "r8b-v8-m16-m17-dual-root-parity",
  },
  {
    id: "v8-p06-packet-census-identity",
    producer: "nineteen evidence objects, C08, resource receipt, frozen process ledger and packet manifest",
    consumer: "closed 23-file P06 directory and M25",
    comparison: "exact path set, regular 0400 nlink-1 members, 0500 directories and one descriptor per retained object",
    test_id: "r8b-v8-p06-census-parity",
  },
  {
    id: "v8-streaming-reference-identity",
    producer: "bounded fixture event sequence",
    consumer: "streaming validator and independent reference in-memory validator",
    comparison: "same terminal event-chain root and rejection point while production path never loads the complete ledger",
    test_id: "r8b-v8-streaming-reference-parity",
  },
  {
    id: "v8-rustix-fd-signal-identity",
    producer: "rustix 1.1.4 inheritable-fd and signal adapters",
    consumer: "restart and descriptor parity tests",
    comparison: "same observed process and descriptor behavior with no unbound /bin/kill process",
    test_id: "r8b-v8-rustix-fd-signal-parity",
  },
);

const invariantsById = new Map(manifest.invariants.map((entry) => [entry.id, entry]));
const setInvariant = (id, statement, testId) =>
  invariantsById.set(id, { id, statement, test_id: testId });
setInvariant("source-scope", "only the 23 measured dirty donor paths and five absent ownership modules may change, under V8 budgets and checkpoint ordering", "r8b-source-scope-parity");
setInvariant("code-route-ownership", "execution, request ownership, physical launch tools, observation, proof and authority retain the frozen V8 design topology", "r8b-code-route-parity");
setInvariant("durable-process-ledger", "expected invocation authority is independent from the bounded streamed observed ledger; every request and completion is append-only and fail-stop", "r8b-process-ledger-parity");
setInvariant("manifest-separation", "26 linked identities, five suite producers, eight tools, Nando invocations, observation tools and physical descendants remain separate denominators", "r8b-linked-manifest-parity");
setInvariant("parent-child-resource-order", "the authenticated user manager owns the delegated service; child exit and loaded-unit metrics precede manager postcheck, exact stop, P06 and M25", "r8b-v8-launch-route-parity");
setInvariant("resource-bound", "the one route-derived transient service is the child resource denominator and is stopped only after terminal metrics are frozen", "r8b-resource-cgroup");
setInvariant("canonical-aggregate-bytes", "M25 streams the closed 23-file P06 packet, reconstructs independent expected schedules and validates nineteen evidence kinds without private truth access", "r8b-aggregate-packet-identity");
setInvariant("hierarchical-ledger-ownership", "request ownership, physical tool mediation and observed child identity remain separate across M24, S02-S05, M01 and M10 partitions", "r8b-v8-launch-route-parity");
setInvariant("canonical-libtest-request-stdin", "all producer inputs, output tables, counts, validators and schedule roots are bound before first write; environment transports only equal request-bound values", "r8b-v8-n19-input-binding");
setInvariant("v8-donor-baseline", "V8-C0 validates all 23 donor bytes/index relationships and five absences without staging, unstaging or mutating the index", "r8b-v8-donor-baseline-parity");
setInvariant("v8-checkpoint-separation", "move-only extraction and behavior repair are separate checkpoints with parity between them", "r8b-v8-move-only-parity");
setInvariant("v8-delegated-ownership", "M24 is request owner, systemd-run is submission tool, the user manager is launch owner and the service process is the child", "r8b-v8-n17-delegated-ownership");
setInvariant("v8-manager-two-channel-identity", "unprivileged bus/pidfd/proc/unit identity and exact pre/post privileged live-image hashes both remain stable", "r8b-v8-manager-identity-parity");
setInvariant("v8-fixed-privileged-probe", "the only privileged operation is exactly two fixed read-only sudo-rs to sha256sum probes over the validated decimal manager PID", "r8b-v8-n43-privileged-probe");
setInvariant("v8-tool-cardinality", "the two sudo frontends and two sha256sum descendants are observation tools, not Nando invocations, ledger events or evidence producers", "r8b-v8-n25-tool-chain-order");
setInvariant("v8-invocation-cardinality", "maximum Nando invocations are 16668 and maximum ledger events are 33336 under the frozen R formula and fail-stop rule", "r8b-v8-n26-ledger-bounds");
setInvariant("v8-streaming-budget", "process-ledger is at most 134217728 bytes and is never decoded as one protocol object", "r8b-v8-n27-streaming-memory");
setInvariant("v8-natural-suffix", "rollback and failure preserve every existing and naturally appended ledger suffix byte; incomplete routes are never replay authority", "r8b-v8-n39-ledger-freeze");
setInvariant("v8-c08-separation", "C08 is expected C09-C20 authority, the process ledger is observed provenance, and neither is evidence kind 20", "r8b-v8-n36-c08-authority");
setInvariant("v8-role-specific-validation", "paths and generic schema/root extraction never substitute for closed concrete validators and canonical byte equality", "r8b-v8-n23-typed-validator");
setInvariant("v8-m16-m17-root-equality", "M16 and M17 event-root and receipt-root sets match exact independent M25 reconstructions", "r8b-v8-m16-m17-dual-root-parity");
setInvariant("v8-packet-census", "P06 contains exactly 23 immutable files and packet-manifest is sealed last without embedding ledger bytes", "r8b-v8-n28-packet-census");
setInvariant("v8-exact-unit-stop", "only the exact test unit is stopped after metrics freeze and its exact cgroup has no descendants", "r8b-v8-n34-exact-stop-target");
setInvariant("v8-implementation-authority-boundary", "READY_TO_IMPLEMENT authorizes only exact V8 source edits and ordinary non-attempt tests", "r8b-v8-zero-authority-parity");
manifest.invariants = [...invariantsById.values()];

const allTestIds = new Set(manifest.tests.map((entry) => entry.id));
const allPreservedIds = new Set(manifest.preserved_artifacts.map((entry) => entry.id));
const allBaselineIds = new Set(manifest.baseline_checks.map((entry) => entry.id));
const allRuntimeIds = new Set(manifest.runtime_comparisons.map((entry) => entry.id));

for (const entry of manifest.invariants) {
  if (!allTestIds.has(entry.test_id)) throw new Error(`unmapped invariant ${entry.id}`);
}
for (const entry of manifest.identity_contracts) {
  if (!allTestIds.has(entry.test_id)) throw new Error(`unmapped identity ${entry.id}`);
}
for (const entry of manifest.runtime_comparisons) {
  if (!allTestIds.has(entry.test_id)) throw new Error(`unmapped runtime ${entry.id}`);
}
for (const entry of manifest.state_machine.steps) {
  if (!allTestIds.has(entry.test_id)) throw new Error(`unmapped step ${entry.id}`);
  for (const preserved of entry.preserves ?? []) {
    if (!allPreservedIds.has(preserved)) throw new Error(`unknown preserved ${preserved}`);
  }
}
for (const entry of manifest.preserved_artifacts) {
  if (!allBaselineIds.has(entry.baseline_check_id)) {
    throw new Error(`unknown baseline for ${entry.id}`);
  }
}
for (const entry of manifest.side_effects.planned) {
  if (entry.comparison_id && !allRuntimeIds.has(entry.comparison_id)) {
    throw new Error(`unknown runtime comparison ${entry.comparison_id}`);
  }
}

for (const collection of [
  manifest.baseline_checks,
  manifest.source_checks,
  manifest.preserved_artifacts,
  manifest.identity_contracts,
  manifest.invariants,
  manifest.tests,
  manifest.runtime_comparisons,
  manifest.state_machine.steps,
]) {
  const ids = collection.map((entry) => entry.id);
  if (new Set(ids).size !== ids.length) throw new Error("duplicate manifest ID");
}

fs.writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`, {
  mode: 0o664,
});
console.log(outputPath);

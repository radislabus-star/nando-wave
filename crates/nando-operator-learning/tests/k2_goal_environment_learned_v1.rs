use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use nando_operator_kernel::{canonical_json_sha256, valid_nonzero_sha256};
use nando_operator_learning::*;

static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

include!("k2_goal_environment_learned_v1/protocol_test.rs");
include!("k2_goal_environment_learned_v1/receipt_assertions.rs");
include!("k2_goal_environment_learned_v1/fixture.rs");

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use nando_operator_learning::*;
use serde::Serialize;

const CONFIRM_COMMITMENT_V1: &str =
    "0a48670dbb2035c0502f064ee10c41c20b5c6391743641b814af98892efba6f4";
const DEVELOPMENT_COMMITMENT_V1: &str =
    "2fbfa252f13d5191024a9ae5d53eae293bd39ab458445808d2414638840a53e7";
const GENERATOR_SCHEMA_ROOT_V1: &str =
    "ad591e3c1a7826295ea93056049dd3759f37c6502b86a542e27dd67fb68a0286";

static TEST_SEQUENCE_V1: AtomicU64 = AtomicU64::new(0);

include!("k2_self_chosen_safe_inquiry_v1/protocol_tests.rs");
include!("k2_self_chosen_safe_inquiry_v1/fixture.rs");
include!("k2_self_chosen_safe_inquiry_v1/receipt_assertions.rs");
include!("k2_self_chosen_safe_inquiry_v1/process_fixture.rs");

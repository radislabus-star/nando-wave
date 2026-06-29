use super::super::{SemanticAtom, SemanticCandidate, SemanticSchemaKey, semantic_label_slot};
use super::L3SemanticExample;
use super::route_family;

pub(super) fn candidates_for_fact(fact: &super::super::SemanticFact) -> [SemanticCandidate; 3] {
    let family = fact.subject.family.clone();
    [
        SemanticCandidate::new(fact.subject.clone()),
        SemanticCandidate::new(SemanticAtom::new(
            fact.schema.object_role.clone(),
            family.clone(),
            fact.subject.slot,
            fact.object.label.clone(),
        )),
        SemanticCandidate::new(SemanticAtom::new(
            fact.schema.subject_role.clone(),
            family,
            fact.subject.slot.wrapping_add(1),
            next_label(&fact.subject.label),
        )),
    ]
}

pub(super) fn semantic_profile_examples(start_slot: u32, count: usize) -> Vec<L3SemanticExample> {
    let mut examples = Vec::with_capacity(count * 2);
    for offset in 0..count as u32 {
        let slot = start_slot + offset;
        examples.push(package_provider_example(slot));
        examples.push(service_executor_example(slot));
    }
    examples
}

pub(super) const HARD_PARAPHRASE_TEMPLATES_PER_FRAME: usize = 4;

#[derive(Clone, Copy, Debug)]
pub(super) struct HardFrameSpec {
    subject_role: &'static str,
    relation: &'static str,
    object_role: &'static str,
    route: &'static str,
    evidence_kind: &'static str,
    subject_prefix: &'static str,
    object_prefix: &'static str,
}

pub(super) const HARD_FRAME_SPECS: [HardFrameSpec; 4] = [
    HardFrameSpec {
        subject_role: "package",
        relation: "provides_command",
        object_role: "command",
        route: "linux.command.provider",
        evidence_kind: "package_metadata",
        subject_prefix: "pkgcmd",
        object_prefix: "cmd",
    },
    HardFrameSpec {
        subject_role: "service",
        relation: "executes_command",
        object_role: "command",
        route: "linux.service.runtime",
        evidence_kind: "unit_metadata",
        subject_prefix: "svc",
        object_prefix: "cmd",
    },
    HardFrameSpec {
        subject_role: "config",
        relation: "enables_service",
        object_role: "service",
        route: "linux.service.config",
        evidence_kind: "config_metadata",
        subject_prefix: "cfg",
        object_prefix: "svc",
    },
    HardFrameSpec {
        subject_role: "package",
        relation: "installs_file",
        object_role: "file",
        route: "linux.package.file",
        evidence_kind: "package_file_index",
        subject_prefix: "pkgfile",
        object_prefix: "file",
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HardTrapKind {
    RoleSwap,
    RouteSplice,
    MissingEvidence,
    NegativeRoute,
}

#[derive(Clone, Debug)]
pub(super) struct HardTrap {
    pub(super) kind: HardTrapKind,
    pub(super) text: String,
}

pub(super) fn hard_semantic_profile_examples(
    start_slot: u32,
    slot_count: usize,
) -> Vec<L3SemanticExample> {
    let mut examples = Vec::with_capacity(
        slot_count * HARD_FRAME_SPECS.len() * HARD_PARAPHRASE_TEMPLATES_PER_FRAME,
    );
    for offset in 0..slot_count as u32 {
        let slot = start_slot + offset;
        for spec in HARD_FRAME_SPECS {
            for template in 0..HARD_PARAPHRASE_TEMPLATES_PER_FRAME {
                examples.push(hard_semantic_example(spec, slot, template));
            }
        }
    }
    examples
}

pub(super) fn hard_semantic_example(
    spec: HardFrameSpec,
    slot: u32,
    template: usize,
) -> L3SemanticExample {
    let schema = SemanticSchemaKey::new(
        spec.subject_role,
        spec.relation,
        spec.object_role,
        spec.route,
        "positive",
        spec.evidence_kind,
    );
    let subject_label = format!("{}{:05}", spec.subject_prefix, slot);
    let object_label = format!("{}{:05}", spec.object_prefix, slot);
    let atom_slot = semantic_label_slot(
        &schema.route,
        &schema.relation,
        &schema.object_role,
        &object_label,
    );
    let family = route_family(&schema.route);
    L3SemanticExample {
        query_surface: hard_query_surface(spec.relation, template, &object_label),
        fact: super::super::SemanticFact::new(
            SemanticAtom::new(spec.subject_role, family.clone(), atom_slot, subject_label),
            schema,
            SemanticAtom::new(spec.object_role, family, atom_slot, object_label),
        ),
    }
}

pub(super) fn hard_shortcut_stress_examples(
    start_slot: u32,
    slot_count: usize,
) -> Vec<L3SemanticExample> {
    let mut examples = Vec::with_capacity(slot_count * HARD_FRAME_SPECS.len());
    for offset in 0..slot_count as u32 {
        let slot = start_slot + offset;
        for spec in HARD_FRAME_SPECS {
            examples.push(hard_shortcut_stress_example(spec, slot));
        }
    }
    examples
}

pub(super) fn hard_shortcut_stress_example(spec: HardFrameSpec, slot: u32) -> L3SemanticExample {
    let schema = SemanticSchemaKey::new(
        spec.subject_role,
        spec.relation,
        spec.object_role,
        spec.route,
        "positive",
        spec.evidence_kind,
    );
    let suffix = alpha_suffix(slot);
    let subject_label = format!("stress{}{}", spec.subject_prefix, suffix);
    let object_label = format!("stress{}{}", spec.object_prefix, suffix);
    let atom_slot = semantic_label_slot(
        &schema.route,
        &schema.relation,
        &schema.object_role,
        &object_label,
    );
    let family = route_family(&schema.route);
    L3SemanticExample {
        query_surface: hard_shortcut_stress_surface(spec.relation, &object_label),
        fact: super::super::SemanticFact::new(
            SemanticAtom::new(spec.subject_role, family.clone(), atom_slot, subject_label),
            schema,
            SemanticAtom::new(spec.object_role, family, atom_slot, object_label),
        ),
    }
}

pub(super) fn hard_shortcut_stress_surface(relation: &str, object_label: &str) -> String {
    match relation {
        "provides_command" => format!("package which provider command {object_label}"),
        "executes_command" => format!("command {object_label} runs service which"),
        "enables_service" => format!("config which enables source service {object_label}"),
        "installs_file" => format!("package which installs find file {object_label}"),
        _ => unreachable!("hard profile relation should be known"),
    }
}

pub(super) fn hard_query_surface(relation: &str, template: usize, object_label: &str) -> String {
    match relation {
        "provides_command" => match template {
            0 => format!("which package provides command {object_label}"),
            1 => format!("find package for command {object_label}"),
            2 => format!("command {object_label} belongs to which package"),
            _ => format!("package provider for command {object_label}"),
        },
        "executes_command" => match template {
            0 => format!("which service executes command {object_label}"),
            1 => format!("find service that runs command {object_label}"),
            2 => format!("command {object_label} is executed by which service"),
            _ => format!("service executor for command {object_label}"),
        },
        "enables_service" => match template {
            0 => format!("which config enables service {object_label}"),
            1 => format!("find config for service {object_label}"),
            2 => format!("service {object_label} is enabled by which config"),
            _ => format!("config source for service {object_label}"),
        },
        "installs_file" => match template {
            0 => format!("which package installs file {object_label}"),
            1 => format!("find package owning file {object_label}"),
            2 => format!("file {object_label} belongs to which package"),
            _ => format!("package owner for file {object_label}"),
        },
        _ => unreachable!("hard profile relation should be known"),
    }
}

pub(super) fn alpha_suffix(mut value: u32) -> String {
    let mut chars = Vec::new();
    loop {
        chars.push((b'a' + (value % 26) as u8) as char);
        value /= 26;
        if value == 0 {
            break;
        }
    }
    chars.into_iter().rev().collect()
}

pub(super) fn hard_traps_for_example(example: &L3SemanticExample) -> [HardTrap; 4] {
    let subject = &example.fact.subject.label;
    let object = &example.fact.object.label;
    match example.fact.schema.relation.as_str() {
        "provides_command" => [
            HardTrap {
                kind: HardTrapKind::RoleSwap,
                text: format!("which command provides package {subject}"),
            },
            HardTrap {
                kind: HardTrapKind::RouteSplice,
                text: format!("which service provides command {object}"),
            },
            HardTrap {
                kind: HardTrapKind::MissingEvidence,
                text: format!("who provides {object}"),
            },
            HardTrap {
                kind: HardTrapKind::NegativeRoute,
                text: format!("which package provides command {object} proves service running"),
            },
        ],
        "executes_command" => [
            HardTrap {
                kind: HardTrapKind::RoleSwap,
                text: format!("which command executes service {subject}"),
            },
            HardTrap {
                kind: HardTrapKind::RouteSplice,
                text: format!("which package executes command {object}"),
            },
            HardTrap {
                kind: HardTrapKind::MissingEvidence,
                text: format!("who executes {object}"),
            },
            HardTrap {
                kind: HardTrapKind::NegativeRoute,
                text: format!("which service executes command {object} proves package installed"),
            },
        ],
        "enables_service" => [
            HardTrap {
                kind: HardTrapKind::RoleSwap,
                text: format!("which service enables config {subject}"),
            },
            HardTrap {
                kind: HardTrapKind::RouteSplice,
                text: format!("which package enables service {object}"),
            },
            HardTrap {
                kind: HardTrapKind::MissingEvidence,
                text: format!("who enables {object}"),
            },
            HardTrap {
                kind: HardTrapKind::NegativeRoute,
                text: format!("which config enables service {object} proves service active"),
            },
        ],
        "installs_file" => [
            HardTrap {
                kind: HardTrapKind::RoleSwap,
                text: format!("which file installs package {subject}"),
            },
            HardTrap {
                kind: HardTrapKind::RouteSplice,
                text: format!("which service installs file {object}"),
            },
            HardTrap {
                kind: HardTrapKind::MissingEvidence,
                text: format!("who owns {object}"),
            },
            HardTrap {
                kind: HardTrapKind::NegativeRoute,
                text: format!("which package installs file {object} proves service enabled"),
            },
        ],
        _ => unreachable!("hard profile relation should be known"),
    }
}

pub(super) fn semantic_traps_for_example(example: &L3SemanticExample) -> Vec<HardTrap> {
    match example.fact.schema.relation.as_str() {
        "provides_command" | "executes_command" | "enables_service" | "installs_file" => {
            hard_traps_for_example(example).into_iter().collect()
        }
        _ => Vec::new(),
    }
}

pub(super) fn fact_key(fact: &super::super::SemanticFact) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}",
        fact.subject.role,
        fact.subject.label,
        fact.schema.relation,
        fact.schema.route,
        fact.schema.polarity,
        fact.schema.evidence_kind,
        fact.object.role,
        fact.object.label
    )
}

pub(super) fn package_provider_example(slot: u32) -> L3SemanticExample {
    let schema = SemanticSchemaKey::new(
        "package",
        "provides_command",
        "command",
        "linux.command.provider",
        "positive",
        "package_metadata",
    );
    let package = format!("pkg{slot:05}");
    let command = format!("cmd{slot:05}");
    let atom_slot = semantic_label_slot(
        &schema.route,
        &schema.relation,
        &schema.object_role,
        &command,
    );
    let family = route_family(&schema.route);
    L3SemanticExample {
        query_surface: format!("which package provides command {command}"),
        fact: super::super::SemanticFact::new(
            SemanticAtom::new("package", family.clone(), atom_slot, package),
            schema,
            SemanticAtom::new("command", family, atom_slot, command),
        ),
    }
}

pub(super) fn service_executor_example(slot: u32) -> L3SemanticExample {
    let schema = SemanticSchemaKey::new(
        "service",
        "executes_command",
        "command",
        "linux.service.runtime",
        "positive",
        "unit_metadata",
    );
    let service = format!("svc{slot:05}");
    let command = format!("cmd{slot:05}");
    let atom_slot = semantic_label_slot(
        &schema.route,
        &schema.relation,
        &schema.object_role,
        &command,
    );
    let family = route_family(&schema.route);
    L3SemanticExample {
        query_surface: format!("which service executes command {command}"),
        fact: super::super::SemanticFact::new(
            SemanticAtom::new("service", family.clone(), atom_slot, service),
            schema,
            SemanticAtom::new("command", family, atom_slot, command),
        ),
    }
}

pub(super) fn next_label(label: &str) -> String {
    format!("{label}_near_miss")
}

pub(super) fn ratio(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

pub(super) fn ratio_f32(numerator: f32, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator / denominator as f32
    }
}

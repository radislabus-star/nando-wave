use std::fs::{self, File};
use std::path::{Path, PathBuf};

use super::super::{K2CompositionErrorV1, K2CompositionResultV1, composition_sha256_file_v1};
use super::{
    K2_UNCERTAINTY_DEVELOPMENT_OWNER_PATH_V1, K2_UNCERTAINTY_DEVELOPMENT_OWNER_PUBLICATION_ID_V1,
    K2UncertaintyConfirmAttemptEventKindV1, K2UncertaintyConfirmAttemptJournalV1,
    K2UncertaintyConfirmAttemptModeV1, K2UncertaintyConfirmAttemptPhaseV1,
    K2UncertaintyConfirmOwnerRequestV1, K2UncertaintyDevelopmentRehearsalFullSplitV1,
    K2UncertaintyDevelopmentRehearsalOwnerReceiptV1, K2UncertaintyGeneratorResponseV1,
    K2UncertaintyImmutablePublicationFaultV1, canonical_private_lab_root_v1,
    dispatch_binding_root_v1, dispatch_self_formed_generator_once_v1,
    load_development_rehearsal_owner_full_v1, load_development_rehearsal_split_full_v1,
    publish_development_rehearsal_split_v1, publish_immutable_file_v1, uncertainty_bytes_v1,
    uncertainty_decode_v1, uncertainty_root_v1,
};

pub fn execute_self_formed_development_rehearsal_owner_v1(
    request: &K2UncertaintyConfirmOwnerRequestV1,
    owner_executable: &Path,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalOwnerReceiptV1> {
    request.validate()?;
    if request.descriptor.mode != K2UncertaintyConfirmAttemptModeV1::DevelopmentRehearsal {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_owner_mode_invalid",
        ));
    }
    let owner_sha256 = composition_sha256_file_v1(owner_executable)?;
    let generator_executable = PathBuf::from(&request.generator_executable_path);
    let generator_sha256 = composition_sha256_file_v1(&generator_executable)?;
    if owner_sha256 != request.descriptor.confirm_owner_executable_sha256
        || generator_sha256 != request.descriptor.generator_executable_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_owner_executable_mismatch",
        ));
    }

    let lab_root = canonical_private_lab_root_v1(&request.lab_root)?;
    let _lock = DevelopmentOwnerLockV1::acquire(&lab_root)?;
    let attempt_root = lab_root.join(&request.attempt_relative_path);
    if fs::symlink_metadata(&attempt_root).is_ok() {
        recover_development_attempt_v1(request, &attempt_root, owner_sha256)
    } else {
        execute_new_development_attempt_v1(
            request,
            &attempt_root,
            &generator_executable,
            owner_sha256,
        )
    }
}

fn execute_new_development_attempt_v1(
    request: &K2UncertaintyConfirmOwnerRequestV1,
    attempt_root: &Path,
    generator_executable: &Path,
    owner_sha256: String,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalOwnerReceiptV1> {
    let mut journal = K2UncertaintyConfirmAttemptJournalV1::create_exclusive(
        attempt_root,
        request.descriptor.clone(),
    )?;
    journal.append(
        K2UncertaintyConfirmAttemptEventKindV1::ArtifactsFrozen,
        owner_sha256.clone(),
        request.request_root_sha256.clone(),
        frozen_artifacts_root_v1(request)?,
    )?;
    dispatch_and_persist_development_v1(
        request,
        attempt_root,
        generator_executable,
        owner_sha256,
        &mut journal,
    )
}

fn dispatch_and_persist_development_v1(
    request: &K2UncertaintyConfirmOwnerRequestV1,
    attempt_root: &Path,
    generator_executable: &Path,
    owner_sha256: String,
    journal: &mut K2UncertaintyConfirmAttemptJournalV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalOwnerReceiptV1> {
    let generator_request =
        request
            .development_generator_request
            .as_ref()
            .ok_or(K2CompositionErrorV1::Invalid(
                "self_formed_development_owner_generator_request_missing",
            ))?;
    let generator_request_root = generator_request.request_root_sha256.clone();
    let mut request_bytes = uncertainty_bytes_v1(generator_request)?;
    journal.append(
        K2UncertaintyConfirmAttemptEventKindV1::GeneratorDispatched,
        owner_sha256.clone(),
        request.request_root_sha256.clone(),
        dispatch_binding_root_v1(
            &generator_request_root,
            &request.descriptor.generator_executable_sha256,
        )?,
    )?;
    let (response_bytes, pipe_receipt) = dispatch_self_formed_generator_once_v1(
        generator_executable,
        &request.descriptor.generator_executable_sha256,
        &generator_request_root,
        &mut request_bytes,
        None,
    )?;
    let response: K2UncertaintyGeneratorResponseV1 = uncertainty_decode_v1(&response_bytes)?;
    response.validate()?;
    let split = publish_development_rehearsal_split_v1(
        &attempt_root.join("generated"),
        request,
        owner_sha256.clone(),
        &response,
        &response_bytes,
        pipe_receipt,
    )?;
    let event = journal.append(
        K2UncertaintyConfirmAttemptEventKindV1::CasesGenerated,
        owner_sha256,
        request.request_root_sha256.clone(),
        split.split_receipt_root_sha256.clone(),
    )?;
    persist_development_owner_v1(request, attempt_root, &split, &event, journal)
}

fn recover_development_attempt_v1(
    request: &K2UncertaintyConfirmOwnerRequestV1,
    attempt_root: &Path,
    owner_sha256: String,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalOwnerReceiptV1> {
    let mut journal = K2UncertaintyConfirmAttemptJournalV1::open_existing(attempt_root)?;
    if journal.descriptor() != &request.descriptor {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_existing_attempt_descriptor_mismatch",
        ));
    }
    let projection = journal.projection();
    match projection.phase {
        K2UncertaintyConfirmAttemptPhaseV1::ReadyForGeneratorDispatch => {
            let generator = PathBuf::from(&request.generator_executable_path);
            dispatch_and_persist_development_v1(
                request,
                attempt_root,
                &generator,
                owner_sha256,
                &mut journal,
            )
        }
        K2UncertaintyConfirmAttemptPhaseV1::GeneratorDispatched => {
            recover_after_dispatch_v1(request, attempt_root, owner_sha256, &mut journal)
        }
        K2UncertaintyConfirmAttemptPhaseV1::CasesGenerated => {
            recover_after_cases_generated_v1(request, attempt_root, &journal)
        }
        K2UncertaintyConfirmAttemptPhaseV1::GeneratorResultIndeterminate => Err(
            K2CompositionErrorV1::Invalid("self_formed_development_generator_indeterminate"),
        ),
        _ => Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_restart_state_invalid",
        )),
    }
}

fn recover_after_dispatch_v1(
    request: &K2UncertaintyConfirmOwnerRequestV1,
    attempt_root: &Path,
    owner_sha256: String,
    journal: &mut K2UncertaintyConfirmAttemptJournalV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalOwnerReceiptV1> {
    let full =
        match load_development_rehearsal_split_full_v1(&attempt_root.join("generated"), request) {
            Ok(full) => full,
            Err(_) => {
                journal.recover_after_restart(
                    owner_sha256,
                    request.request_root_sha256.clone(),
                    None,
                    None,
                )?;
                return Err(K2CompositionErrorV1::Invalid(
                    "self_formed_development_generator_indeterminate",
                ));
            }
        };
    let event = journal.append(
        K2UncertaintyConfirmAttemptEventKindV1::CasesGenerated,
        owner_sha256,
        request.request_root_sha256.clone(),
        full.split.split_receipt_root_sha256.clone(),
    )?;
    persist_development_owner_v1(request, attempt_root, &full.split, &event, journal)
}

fn recover_after_cases_generated_v1(
    request: &K2UncertaintyConfirmOwnerRequestV1,
    attempt_root: &Path,
    journal: &K2UncertaintyConfirmAttemptJournalV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalOwnerReceiptV1> {
    match load_development_rehearsal_owner_full_v1(attempt_root, request) {
        Ok((owner, full)) => {
            validate_durable_owner_v1(&owner, &full, journal)?;
            Ok(owner)
        }
        Err(_) => {
            let full =
                load_development_rehearsal_split_full_v1(&attempt_root.join("generated"), request)?;
            let event = journal
                .events()
                .last()
                .ok_or(K2CompositionErrorV1::Invalid(
                    "self_formed_development_cases_event_missing",
                ))?;
            persist_development_owner_v1(request, attempt_root, &full.split, event, journal)
        }
    }
}

fn persist_development_owner_v1(
    request: &K2UncertaintyConfirmOwnerRequestV1,
    attempt_root: &Path,
    split: &super::K2UncertaintyDevelopmentRehearsalSplitReceiptV1,
    event: &super::K2UncertaintyConfirmAttemptEventV1,
    journal: &K2UncertaintyConfirmAttemptJournalV1,
) -> K2CompositionResultV1<K2UncertaintyDevelopmentRehearsalOwnerReceiptV1> {
    if event.kind != K2UncertaintyConfirmAttemptEventKindV1::CasesGenerated
        || event.payload_root_sha256 != split.split_receipt_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_cases_event_binding_invalid",
        ));
    }
    let owner = K2UncertaintyDevelopmentRehearsalOwnerReceiptV1::seal(
        request,
        split,
        event,
        journal.projection().generator_dispatch_count,
    )?;
    publish_immutable_file_v1(
        attempt_root,
        K2_UNCERTAINTY_DEVELOPMENT_OWNER_PATH_V1,
        &uncertainty_bytes_v1(&owner)?,
        0o600,
        K2_UNCERTAINTY_DEVELOPMENT_OWNER_PUBLICATION_ID_V1,
        K2UncertaintyImmutablePublicationFaultV1::None,
    )?;
    let (reopened, full) = load_development_rehearsal_owner_full_v1(attempt_root, request)?;
    validate_durable_owner_v1(&reopened, &full, journal)?;
    if reopened != owner {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_owner_reopen_mismatch",
        ));
    }
    Ok(reopened)
}

fn validate_durable_owner_v1(
    owner: &K2UncertaintyDevelopmentRehearsalOwnerReceiptV1,
    full: &K2UncertaintyDevelopmentRehearsalFullSplitV1,
    journal: &K2UncertaintyConfirmAttemptJournalV1,
) -> K2CompositionResultV1<()> {
    owner.validate()?;
    let projection = journal.projection();
    let event = journal
        .events()
        .last()
        .ok_or(K2CompositionErrorV1::Invalid(
            "self_formed_development_cases_event_missing",
        ))?;
    if projection.phase != K2UncertaintyConfirmAttemptPhaseV1::CasesGenerated
        || projection.generator_dispatch_count != 1
        || event.kind != K2UncertaintyConfirmAttemptEventKindV1::CasesGenerated
        || event.payload_root_sha256 != full.split.split_receipt_root_sha256
        || event.event_root_sha256 != owner.cases_generated_event_root_sha256
    {
        return Err(K2CompositionErrorV1::Invalid(
            "self_formed_development_durable_owner_invalid",
        ));
    }
    Ok(())
}

fn frozen_artifacts_root_v1(
    request: &K2UncertaintyConfirmOwnerRequestV1,
) -> K2CompositionResultV1<String> {
    uncertainty_root_v1(&(
        "nando.k2-self-formed-confirm-frozen-artifacts.v1",
        &request.request_root_sha256,
        &request.descriptor.executable_manifest_root_sha256,
    ))
}

struct DevelopmentOwnerLockV1(File);

impl DevelopmentOwnerLockV1 {
    fn acquire(lab_root: &Path) -> K2CompositionResultV1<Self> {
        let file = File::open(lab_root)
            .map_err(|_| K2CompositionErrorV1::Io("open_self_formed_development_owner_lock"))?;
        if file.try_lock().is_err() {
            return Err(K2CompositionErrorV1::Process(
                "development_attempt_owner_busy",
            ));
        }
        Ok(Self(file))
    }
}

impl Drop for DevelopmentOwnerLockV1 {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

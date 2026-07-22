use super::PhaseControlV3;
use nando_core::wave::RuntimeRelationPhaseComponent;

pub(super) fn score_phase_components_v3(
    components: &[RuntimeRelationPhaseComponent],
    control: PhaseControlV3,
) -> i64 {
    if control == PhaseControlV3::NoPhase || components.is_empty() {
        return 0;
    }
    components
        .iter()
        .enumerate()
        .map(|(index, component)| {
            let observed = match control {
                PhaseControlV3::ShuffledPhase => {
                    let shifted = components[(index + 1) % components.len()];
                    let (re, im) = shifted.observed_fixed();
                    (i64::from(re), i64::from(im))
                }
                PhaseControlV3::MagnitudeOnly => {
                    let (re, im) = component.observed_fixed();
                    let re = i64::from(re);
                    let im = i64::from(im);
                    (integer_sqrt((re * re + im * im) as u64) as i64, 0)
                }
                _ => {
                    let (re, im) = component.observed_fixed();
                    (i64::from(re), i64::from(im))
                }
            };
            let expected = if control == PhaseControlV3::MatchedRandomCenter {
                random_center(
                    component.plane(),
                    component.source_role(),
                    component.target_role(),
                )
            } else {
                let (re, im) = component.expected_fixed();
                (i64::from(re), i64::from(im))
            };
            (observed.0 * expected.0 + observed.1 * expected.1)
                / RuntimeRelationPhaseComponent::SCALE_FIXED
        })
        .fold(0_i64, i64::saturating_add)
}

pub(super) fn coherence_phase_components_v3(
    components: &[RuntimeRelationPhaseComponent],
    control: PhaseControlV3,
) -> i64 {
    let Ok(component_count) = i64::try_from(components.len()) else {
        return 0;
    };
    if component_count == 0 {
        return 0;
    }
    score_phase_components_v3(components, control)
        .checked_div(component_count)
        .unwrap_or_default()
        .clamp(
            -RuntimeRelationPhaseComponent::SCALE_FIXED,
            RuntimeRelationPhaseComponent::SCALE_FIXED,
        )
}

fn random_center(plane: u8, source_role: u8, target_role: u8) -> (i64, i64) {
    const DIAGONAL: i64 = 707_106_781;
    const CENTERS: [(i64, i64); 8] = [
        (1_000_000_000, 0),
        (DIAGONAL, DIAGONAL),
        (0, 1_000_000_000),
        (-DIAGONAL, DIAGONAL),
        (-1_000_000_000, 0),
        (-DIAGONAL, -DIAGONAL),
        (0, -1_000_000_000),
        (DIAGONAL, -DIAGONAL),
    ];
    let mixed = u32::from(plane)
        .wrapping_mul(0x9e37_79b9)
        .wrapping_add(u32::from(source_role).wrapping_mul(0x85eb_ca6b))
        .wrapping_add(u32::from(target_role).wrapping_mul(0xc2b2_ae35));
    CENTERS[(mixed as usize) % CENTERS.len()]
}

fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut current = value;
    let mut next = (current + value / current) / 2;
    while next < current {
        current = next;
        next = (current + value / current) / 2;
    }
    current
}

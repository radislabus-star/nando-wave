use super::root_v1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParentStageV3 {
    P00SourceValidated,
    P01ProducersClosed,
    P02PreSnapshotFrozen,
    P03AManagerBound,
    P03BDelegatedRequestFrozen,
    P04AResourcesFrozen,
    P04BManagerReverified,
    P05ProductionSurvived,
    P06PacketFrozen,
    P07Authorized,
    P08Published,
    P09Audited,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ParentRouteV3 {
    stage: ParentStageV3,
    packet_root_sha256: Option<String>,
    authorization_root_sha256: Option<String>,
    publication_root_sha256: Option<String>,
    p09_authorized: bool,
}

impl ParentRouteV3 {
    fn new() -> Self {
        Self {
            stage: ParentStageV3::P00SourceValidated,
            packet_root_sha256: None,
            authorization_root_sha256: None,
            publication_root_sha256: None,
            p09_authorized: false,
        }
    }

    fn advance(
        &mut self,
        next: ParentStageV3,
        durable_root_sha256: Option<String>,
    ) -> Result<(), &'static str> {
        let expected = match self.stage {
            ParentStageV3::P00SourceValidated => ParentStageV3::P01ProducersClosed,
            ParentStageV3::P01ProducersClosed => ParentStageV3::P02PreSnapshotFrozen,
            ParentStageV3::P02PreSnapshotFrozen => ParentStageV3::P03AManagerBound,
            ParentStageV3::P03AManagerBound => ParentStageV3::P03BDelegatedRequestFrozen,
            ParentStageV3::P03BDelegatedRequestFrozen => ParentStageV3::P04AResourcesFrozen,
            ParentStageV3::P04AResourcesFrozen => ParentStageV3::P04BManagerReverified,
            ParentStageV3::P04BManagerReverified => ParentStageV3::P05ProductionSurvived,
            ParentStageV3::P05ProductionSurvived => ParentStageV3::P06PacketFrozen,
            ParentStageV3::P06PacketFrozen => ParentStageV3::P07Authorized,
            ParentStageV3::P07Authorized => ParentStageV3::P08Published,
            ParentStageV3::P08Published if self.p09_authorized => ParentStageV3::P09Audited,
            _ => return Err("r8b_v8_parent_route_terminal"),
        };
        if next != expected {
            return Err("r8b_v8_parent_route_order_invalid");
        }
        if let Some(root) = &durable_root_sha256 {
            nando_operator_learning::require_composition_root_v1(root)
                .map_err(|_| "r8b_v8_parent_root_invalid")?;
        }
        match next {
            ParentStageV3::P06PacketFrozen => self.packet_root_sha256 = durable_root_sha256,
            ParentStageV3::P07Authorized => {
                if self.packet_root_sha256.is_none() {
                    return Err("r8b_v8_parent_packet_missing");
                }
                self.authorization_root_sha256 = durable_root_sha256;
            }
            ParentStageV3::P08Published => {
                if self.authorization_root_sha256.is_none() {
                    return Err("r8b_v8_parent_authorization_missing");
                }
                self.publication_root_sha256 = durable_root_sha256;
            }
            ParentStageV3::P09Audited if durable_root_sha256.is_some() => {}
            ParentStageV3::P09Audited => return Err("r8b_v8_parent_audit_root_missing"),
            _ if durable_root_sha256.is_some() => {
                return Err("r8b_v8_parent_early_authority_root");
            }
            _ => {}
        }
        self.stage = next;
        Ok(())
    }

    fn authorize_p09(&mut self) -> Result<(), &'static str> {
        if self.stage != ParentStageV3::P08Published {
            return Err("r8b_v8_p09_authorization_early");
        }
        self.p09_authorized = true;
        Ok(())
    }
}

#[test]
fn r8b_v8_parent_p00_p08_is_strict_and_root_preserving() {
    let mut route = ParentRouteV3::new();
    for stage in [
        ParentStageV3::P01ProducersClosed,
        ParentStageV3::P02PreSnapshotFrozen,
        ParentStageV3::P03AManagerBound,
        ParentStageV3::P03BDelegatedRequestFrozen,
        ParentStageV3::P04AResourcesFrozen,
        ParentStageV3::P04BManagerReverified,
        ParentStageV3::P05ProductionSurvived,
    ] {
        route.advance(stage, None).unwrap();
    }
    let packet = root_v1("v8-p06-packet");
    let authorization = root_v1("v8-p07-authorization");
    let publication = root_v1("v8-p08-publication");
    route.advance(ParentStageV3::P06PacketFrozen, Some(packet.clone())).unwrap();
    route.advance(ParentStageV3::P07Authorized, Some(authorization.clone())).unwrap();
    route.advance(ParentStageV3::P08Published, Some(publication.clone())).unwrap();
    assert_eq!(route.packet_root_sha256.as_deref(), Some(packet.as_str()));
    assert_eq!(route.authorization_root_sha256.as_deref(), Some(authorization.as_str()));
    assert_eq!(route.publication_root_sha256.as_deref(), Some(publication.as_str()));
}

#[test]
fn r8b_v8_parent_rejects_stage_skip_and_early_p09() {
    let mut route = ParentRouteV3::new();
    assert!(route.advance(ParentStageV3::P02PreSnapshotFrozen, None).is_err());
    assert!(route.authorize_p09().is_err());
}

#[test]
fn r8b_v8_parent_p09_requires_separate_authority_and_cannot_rewrite_p06_p08() {
    let mut route = ParentRouteV3::new();
    for stage in [
        ParentStageV3::P01ProducersClosed,
        ParentStageV3::P02PreSnapshotFrozen,
        ParentStageV3::P03AManagerBound,
        ParentStageV3::P03BDelegatedRequestFrozen,
        ParentStageV3::P04AResourcesFrozen,
        ParentStageV3::P04BManagerReverified,
        ParentStageV3::P05ProductionSurvived,
    ] {
        route.advance(stage, None).unwrap();
    }
    route.advance(ParentStageV3::P06PacketFrozen, Some(root_v1("packet"))).unwrap();
    route.advance(ParentStageV3::P07Authorized, Some(root_v1("authorization"))).unwrap();
    route.advance(ParentStageV3::P08Published, Some(root_v1("publication"))).unwrap();
    let before = route.clone();
    assert!(route.advance(ParentStageV3::P09Audited, Some(root_v1("audit"))).is_err());
    route.authorize_p09().unwrap();
    route.advance(ParentStageV3::P09Audited, Some(root_v1("audit"))).unwrap();
    assert_eq!(route.packet_root_sha256, before.packet_root_sha256);
    assert_eq!(route.authorization_root_sha256, before.authorization_root_sha256);
    assert_eq!(route.publication_root_sha256, before.publication_root_sha256);
}

# NANDA Triad Worksheet

task_id: s1c3h-authority-pair-install-route-v1
domain: code
query: Does the S1C-3H installation route deploy and roll back the runtime-authority compatibility unit without accepting a mixed pair?

## triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| t2 | S1C-3G transaction | installed_only | transition runtime and environment | exact prior install defect lines 24-27 | 1.0 | failed installer | partial compatibility unit | deployment | historical-s1c3g | operations | S1C-3G evidence owner | terminal receipt | rollback pass | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:24 | historical evidence |
| t3 | response admission controller | issued | old f8d955 runtime authority | exact old producer binding lines 24-26 | 1.0 | authority producer | old contract | authority | authority-producer | runtime | response admission owner | controller receipt | old digest | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:25 | production baseline |
| t4 | candidate runtime | rejected | mismatched old authority | exact runtime rejection lines 26-31 | 1.0 | authority consumer | incompatible receipt | authority | authority-consumer | runtime | transition serving owner | startup refresh | fail closed | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:26 | production safety |
| t5 | S1C-3H compatibility unit | includes | runtime config authority binary sidecars final admission | exact unit list lines 38-46 | 1.0 | deployment unit | jointly versioned artifacts | deployment | installation-unit | operations | S1C-3H installer | staged candidate | one unit | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:38 | production |
| t6 | off-path preparation | generates | candidate sidecars and final admission | exact staged route lines 53-61 | 1.0 | preparation executor | staged authority | preparation | authority-stage | proof | S1C-3H preparation owner | copied immutable inputs | frozen PASS packet | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:53 | deployment only |
| t7 | production mutation | occurs_after | staged candidate PASS and rollback armed | exact production route lines 63-73 | 1.0 | transaction executor | chronology boundary | deployment | production-install | operations | S1C-3H transaction owner | verified staging | bounded restart | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:63 | production safety |
| t8 | rollback | restores | complete old compatibility unit | exact rollback restore lines 95-103 | 1.0 | rollback owner | baseline bytes and authority | rollback | complete-rollback | operations | S1C-3H rollback owner | durable rollback directory | old pair READY | plans/effect-law-unification-v1/S1C3H_AUTHORITY_PAIR_INSTALLATION_PREREGISTRATION_V1.md:95 | production safety |

## candidate_triads

| id | subject | relation | object | evidence | confidence | subject_role | object_role | route | group | layer | owner | entrypoint | output | evidence_path | scope |
|----|---------|----------|--------|----------|------------|--------------|-------------|-------|-------|-------|-------|------------|--------|---------------|-------|
| c2 | S1C-3G transaction | installed_only | transition runtime and environment | candidate c2 | 1.0 | failed installer | partial compatibility unit | deployment | historical-s1c3g | operations | S1C-3G evidence owner | terminal receipt | rollback pass | candidate_answer:c2 | historical evidence |
| c3 | response admission controller | issued | old f8d955 runtime authority | candidate c3 | 1.0 | authority producer | old contract | authority | authority-producer | runtime | response admission owner | controller receipt | old digest | candidate_answer:c3 | production baseline |
| c4 | candidate runtime | rejected | mismatched old authority | candidate c4 | 1.0 | authority consumer | incompatible receipt | authority | authority-consumer | runtime | transition serving owner | startup refresh | fail closed | candidate_answer:c4 | production safety |
| c5 | S1C-3H compatibility unit | includes | runtime config authority binary sidecars final admission | candidate c5 | 1.0 | deployment unit | jointly versioned artifacts | deployment | installation-unit | operations | S1C-3H installer | staged candidate | one unit | candidate_answer:c5 | production |
| c6 | off-path preparation | generates | candidate sidecars and final admission | candidate c6 | 1.0 | preparation executor | staged authority | preparation | authority-stage | proof | S1C-3H preparation owner | copied immutable inputs | frozen PASS packet | candidate_answer:c6 | deployment only |
| c7 | production mutation | occurs_after | staged candidate PASS and rollback armed | candidate c7 | 1.0 | transaction executor | chronology boundary | deployment | production-install | operations | S1C-3H transaction owner | verified staging | bounded restart | candidate_answer:c7 | production safety |
| c8 | rollback | restores | complete old compatibility unit | candidate c8 | 1.0 | rollback owner | baseline bytes and authority | rollback | complete-rollback | operations | S1C-3H rollback owner | durable rollback directory | old pair READY | candidate_answer:c8 | production safety |

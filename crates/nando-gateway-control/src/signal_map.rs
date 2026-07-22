use crate::f5_runtime_status::ProofSummary;
use serde_json::Value;

pub(crate) struct LiveSignalView<'a> {
    pub(crate) partition: u64,
    pub(crate) generation: u64,
    pub(crate) transitions: u64,
    pub(crate) support: u64,
    pub(crate) physical_adapters: u64,
    pub(crate) matching: u64,
    pub(crate) matching_sessions: u64,
    pub(crate) after_watermark: u64,
    pub(crate) independent: u64,
    pub(crate) consistent: u64,
    pub(crate) routed: u64,
    pub(crate) future: u64,
    pub(crate) support_frame_rejects: u64,
    pub(crate) support_session_rejects: u64,
    pub(crate) support_intent_rejects: u64,
    pub(crate) support_event_rejects: u64,
    pub(crate) program_mismatch_rejects: u64,
    pub(crate) route_mismatch_rejects: u64,
    pub(crate) blocker: &'a str,
    pub(crate) admission_verdict: &'a str,
    pub(crate) admission_blocker: &'a str,
    pub(crate) admission_blocker_stage: &'a str,
    pub(crate) admission_age_seconds: u64,
    pub(crate) admission_relation_candidates: u64,
    pub(crate) admission_future_rows: u64,
    pub(crate) admission_runtime_parity_cases: u64,
    pub(crate) active_packages: u64,
    pub(crate) active_transition_profiles: u64,
    pub(crate) verified_local_accepts: u64,
    pub(crate) call_saving_share_milli: u64,
    pub(crate) input_token_saving_share_milli: u64,
    pub(crate) online_ready: bool,
    pub(crate) capture_phase: &'a str,
    pub(crate) capture_records: u64,
    pub(crate) capture_captured: u64,
    pub(crate) capture_censored: u64,
    pub(crate) capture_publish_sequence: u64,
    pub(crate) capture_last_error: &'a str,
    pub(crate) shadow_phase: &'a str,
    pub(crate) shadow_submitted: u64,
    pub(crate) shadow_evaluated: u64,
    pub(crate) shadow_verified: u64,
    pub(crate) shadow_parity_mismatches: u64,
}

#[derive(Clone, Copy)]
enum RouteState {
    Live,
    Work,
    Proven,
    Wait,
    Block,
    Locked,
}

impl RouteState {
    const fn class(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Work => "work",
            Self::Proven => "proven",
            Self::Wait => "wait",
            Self::Block => "block",
            Self::Locked => "locked",
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Live => "ЖИВОЙ ПОТОК",
            Self::Work => "РАБОТАЕТ",
            Self::Proven => "ТОЛЬКО ДОКАЗАТЕЛЬСТВО",
            Self::Wait => "ОЖИДАНИЕ",
            Self::Block => "БЛОК",
            Self::Locked => "ЗАКРЫТО",
        }
    }

    const fn live_flow(self) -> bool {
        matches!(self, Self::Live | Self::Work)
    }
}

struct PipelineStage {
    id: &'static str,
    title: &'static str,
    owner: String,
    input: String,
    live: String,
    proof: String,
    diagnostic: String,
    output: String,
    state: RouteState,
}

pub(crate) fn render(
    live: &LiveSignalView<'_>,
    proof: &ProofSummary,
    manifest: &Value,
    model_label: &str,
) -> String {
    let build_id = manifest
        .get("build_id")
        .and_then(Value::as_str)
        .unwrap_or("MISSING");
    let build_commit = manifest
        .get("git_commit")
        .and_then(Value::as_str)
        .unwrap_or("MISSING");
    let natural_operator_live = live.active_packages > 0;
    let transition_cpu_working =
        live.active_transition_profiles > 0 && live.verified_local_accepts > 0;
    let proof_state = if proof.verified {
        RouteState::Proven
    } else {
        RouteState::Locked
    };
    let downstream_state = if natural_operator_live {
        RouteState::Live
    } else {
        proof_state
    };
    let capture_error = if live.capture_last_error.is_empty() {
        "нет"
    } else {
        live.capture_last_error
    };
    let natural_blocker = natural_blocker_text(live);

    let proof_boundary = if proof.verified {
        format!(
            "STOP-F8 {} / проверено {} / {} / дата {}",
            proof.f8_external_verdict,
            proof.f8_verified_receipts,
            compact(&proof.f5_commit),
            proof.f7_receipt_date,
        )
    } else {
        format!(
            "доказательные квитанции недоступны: {}",
            proof
                .failure
                .as_deref()
                .unwrap_or("неизвестная ошибка проверки")
        )
    };
    let (current_stage, current_blocker, current_reason) = if !proof.verified {
        (
            "PROOF",
            "ПРОВЕРКА ДОКАЗАТЕЛЬНЫХ КВИТАНЦИЙ",
            "Квитанции контролируемого доказательства не прошли строгую проверку. Нижние модули не могут показывать доказанный статус, пока не проверены их канонические байты.".to_owned(),
        )
    } else if !natural_operator_live {
        (
            "L3",
            "L3 ОТКРЫТИЕ ЕСТЕСТВЕННОГО ОПЕРАТОРА",
            format!(
                "Обычные трассы доходят до обучения, но естественного OperatorPackage пока нет: {}. STOP-F8 доказывает только нижнюю техническую цепочку; его контролируемый seed не является естественным evidence.",
                natural_blocker,
            ),
        )
    } else {
        (
            "L11",
            "L11 ПРОИЗВОДСТВЕННЫЙ ДОПУСК",
            "Естественный пакет существует, но перед исполнением на CPU независимый допуск всё ещё должен выдать отдельную лицензию authority.".to_owned(),
        )
    };

    let stages = vec![
        PipelineStage {
            id: "L1",
            title: "Приём запроса провайдера",
            owner: "nando-nginx-gateway".into(),
            input: format!("запрос Codex / модель {model_label}"),
            live: "поток HTTPS активен; резервный маршрут к провайдеру доступен".into(),
            proof: "здоровье транспорта наблюдается независимо от authority оператора".into(),
            diagnostic: format!(
                "сборка {build_id} / режим SHADOW / новый локальный CPU-допуск выключен"
            ),
            output: "конверт запроса + завершённая трасса".into(),
            state: RouteState::Live,
        },
        PipelineStage {
            id: "L2",
            title: "Сбор трасс и хеш-доказательств",
            owner: "nando-transition-serving".into(),
            input: "граница провайдера + завершённая трасса сессии".into(),
            live: format!(
                "наблюдатель {} / переходов {}; сбор {} / получено сейчас {} / цензурировано {}",
                if live.online_ready {
                    "ГОТОВ"
                } else {
                    "ОЖИДАНИЕ"
                },
                live.transitions,
                live.capture_phase,
                live.capture_captured,
                live.capture_censored,
            ),
            proof: if proof.verified {
                format!(
                    "сбор только хешей ПРОЙДЕН / устойчивых контролируемых записей {} / сырой payload 0 Б",
                    proof.f8_provider_records.max(live.capture_records)
                )
            } else {
                "доказательство сбора только хешей недоступно".into()
            },
            diagnostic: format!(
                "публикация {} / ошибка {} / счётчики процесса могут сбрасываться, устойчивый индекс нет",
                live.capture_publish_sequence, capture_error,
            ),
            output: "фрагменты отношений + неизменяемая квитанция запроса".into(),
            state: if live.online_ready {
                RouteState::Live
            } else {
                RouteState::Wait
            },
        },
        PipelineStage {
            id: "L3",
            title: "Открытие естественного оператора",
            owner: "nando-operator-learning".into(),
            input: "фрагменты отношений + teacher-evidence завершённой трассы".into(),
            live: format!(
                "support {}/32 / адаптеров {} / совпадений {} в {} сессиях / независимых {} / future {}/32",
                live.support,
                live.physical_adapters,
                live.matching,
                live.matching_sessions,
                live.independent,
                live.future,
            ),
            proof: "НЕ ОЦЕНЕНО в STOP-F8; контролируемый seed вводится после этой границы".into(),
            diagnostic: format!(
                "watermark {} / support-rejects frame:{} session:{} intent:{} event:{} / program-rejects {} / route-rejects {} / согласованных {} / маршрутизировано {}",
                live.after_watermark,
                live.support_frame_rejects,
                live.support_session_rejects,
                live.support_intent_rejects,
                live.support_event_rejects,
                live.program_mismatch_rejects,
                live.route_mismatch_rejects,
                live.consistent,
                live.routed,
            ),
            output: "естественный circuit-attractor + типизированный OperatorPackage".into(),
            state: if natural_operator_live {
                RouteState::Live
            } else {
                RouteState::Block
            },
        },
        PipelineStage {
            id: "L4",
            title: "Кристаллизатор оператора",
            owner: "nando-operator-learning".into(),
            input: "фазово-когерентный circuit-attractor".into(),
            live: live_downstream_text(natural_operator_live, "естественный circuit"),
            proof: if proof.verified {
                "контролируемый circuit скомпилирован в ограниченные данные оператора".into()
            } else {
                "доказательство контролируемой кристаллизации недоступно".into()
            },
            diagnostic:
                "нужна когерентность полного circuit; одних выбранных фрагментов недостаточно"
                    .into(),
            output: "версионированный неизменяемый OperatorPackage".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L5",
            title: "Хранение поколения",
            owner: "nando-operator-persistence".into(),
            input: "неизменяемое поколение OperatorPackage".into(),
            live: live_downstream_text(natural_operator_live, "естественное поколение"),
            proof: if proof.verified {
                format!(
                    "закреплённое поколение ПРОЙДЕНО / очередь <= {} / parity после перезапуска ПРОЙДЕНА",
                    proof.f7_queue_max
                )
            } else {
                "доказательство поколения недоступно".into()
            },
            diagnostic: format!(
                "живая lineage partition.v{} / поколение {}",
                live.partition, live.generation
            ),
            output: "устойчивое к перезапуску поколение маршрутизации".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L6",
            title: "Фазовый маршрутизатор",
            owner: "nando-operator-runtime".into(),
            input: "runtime-состояние отношений + поколение кандидатов".into(),
            live: live_downstream_text(natural_operator_live, "маршрутизация обычных запросов"),
            proof: if proof.verified {
                format!(
                    "полная фаза выбрала {0}/{0}; абляции выбрали 0; прирост применимости {1}",
                    proof.f8_verified_receipts, proof.f8_full_phase_gain,
                )
            } else {
                "доказательство фазового контроля недоступно".into()
            },
            diagnostic: format!(
                "прирост поиска {} / F7 без совпадения p99 {} нс / F8 максимум p99 без совпадения {} нс",
                proof.f8_search_gain, proof.f7_no_match_p99_ns, proof.f8_no_match_p99_max_ns,
            ),
            output: "один кандидат оператора с запасом когерентности | ABSTAIN".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L7",
            title: "Связывание ролей во время исполнения",
            owner: "nando-operator-runtime".into(),
            input: "выбранный оператор + текущая структурная поверхность".into(),
            live: live_downstream_text(natural_operator_live, "связывание ролей обычного запроса"),
            proof: if proof.verified {
                "контролируемое связывание ролей владельцем-победителем ПРОЙДЕНО".into()
            } else {
                "доказательство связывания ролей недоступно".into()
            },
            diagnostic:
                "неоднозначная или отсутствующая структурная роль всегда возвращает ABSTAIN".into(),
            output: "единственная BoundRoleEnvironment | ABSTAIN".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L8",
            title: "Виртуальная машина оператора",
            owner: "nando-operator-runtime".into(),
            input: "связанные роли + версионированная программа оператора".into(),
            live: live_downstream_text(natural_operator_live, "исполнение обычного запроса в VM"),
            proof: if proof.verified {
                "контролируемое исполнение actor ПРОЙДЕНО / execution authority=false".into()
            } else {
                "доказательство VM недоступно".into()
            },
            diagnostic: format!(
                "F7 с совпадением p99 {} нс / F8 максимум p99 с совпадением {} нс / жёсткий максимум {} нс",
                proof.f7_matched_p99_ns, proof.f8_matched_p99_max_ns, proof.f8_hard_max_ns,
            ),
            output: "результат кандидата + квитанция actor".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L9",
            title: "Независимый верификатор",
            owner: "nando-operator-proof".into(),
            input: "результат кандидата + неизменяемый ожидаемый контракт".into(),
            live: live_downstream_text(natural_operator_live, "проверка обычного кандидата"),
            proof: if proof.verified {
                format!(
                    "проверено {0}/{0} контролируемых квитанций",
                    proof.f8_verified_receipts
                )
            } else {
                "доказательство верификатора недоступно".into()
            },
            diagnostic: format!(
                "расхождений parity {} / ложных допусков 0",
                live.shadow_parity_mismatches,
            ),
            output: "VerifiedDeltaReceipt | отказ верификатора".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L10",
            title: "Журнал квитанций поколения",
            owner: "nando-operator-learning".into(),
            input: "квитанция actor + квитанция независимого верификатора".into(),
            live: format!(
                "процесс {} / отправлено {} / оценено {} / проверено {}",
                live.shadow_phase,
                live.shadow_submitted,
                live.shadow_evaluated,
                live.shadow_verified,
            ),
            proof: if proof.verified {
                format!(
                    "устойчивых контролируемых квитанций {} / добавление после перезапуска ПРОЙДЕНО",
                    proof.f8_verified_receipts
                )
            } else {
                "доказательство устойчивого журнала недоступно".into()
            },
            diagnostic: format!(
                "горячий RSS {} / {} Б / максимум precursor {} Б на {} наблюдениях / сырой payload 0 Б",
                proof.f8_hot_rss_bytes,
                proof.f8_rss_target_bytes,
                proof.f8_rss_bytes,
                proof.f8_resource_observations,
            ),
            output: "неизменяемый набор evidence для внешнего допуска".into(),
            state: downstream_state,
        },
        PipelineStage {
            id: "L11",
            title: "Независимый допуск",
            owner: "nando-operator-admission".into(),
            input: "поколение + capture + журнал + обязательства причинного контроля".into(),
            live: format!(
                "обычный контроллер {} / future кандидата {}/32 / случаев parity {}",
                live.admission_verdict,
                live.admission_future_rows,
                live.admission_runtime_parity_cases,
            ),
            proof: if proof.verified {
                format!(
                    "{} / обязательства {} / authority=false",
                    proof.f8_external_verdict,
                    compact(&proof.f8_commitments_sha256),
                )
            } else {
                "доказательство внешней реконструкции недоступно".into()
            },
            diagnostic: format!(
                "{} / {} / кандидатов отношений {} / возраст снимка {} с",
                live.admission_blocker_stage,
                live.admission_blocker,
                live.admission_relation_candidates,
                live.admission_age_seconds,
            ),
            output: "подписанная лицензия authority | SHADOW_READY | отказ".into(),
            state: downstream_state,
        },
    ];

    let mut pipeline = String::new();
    for (index, stage) in stages.iter().enumerate() {
        pipeline.push_str(&render_stage(stage));
        if stage.id == "L3" && !natural_operator_live {
            pipeline.push_str(&live_signal_break(live));
        } else if index + 1 < stages.len() {
            let next = stages[index + 1].id;
            pipeline.push_str(&handoff(
                stage.id,
                next,
                if natural_operator_live || index < 2 {
                    "типизированная передача живого сигнала"
                } else {
                    "только передача контролируемого доказательства"
                },
                natural_operator_live || index < 2,
            ));
        }
    }
    pipeline.push_str(&authority_boundary(proof));
    pipeline.push_str(&render_stage(&PipelineStage {
        id: "CPU",
        title: "Активное исполнение на CPU",
        owner: "nando-transition-serving".into(),
        input: "допущенные transition-профили; естественным операторам нужна отдельная лицензия"
            .into(),
        live: format!(
            "проверенных локальных исполнений {} / активных transition-профилей {} / обычный трафик на CPU {} / экономия токенов {}",
            live.verified_local_accepts,
            live.active_transition_profiles,
            format_ratio_milli(live.call_saving_share_milli),
            format_ratio_milli(live.input_token_saving_share_milli),
        ),
        proof: format!(
            "Естественный кандидат STOP-F8 остаётся authority=false / естественных ACTIVE response-пакетов {}",
            live.active_packages,
        ),
        diagnostic: "существующий проверенный transition-маршрут активен; линия естественного response-оператора остаётся ЗАКРЫТА"
            .into(),
        output: "проверенный локальный ответ | резервный ответ OpenAI".into(),
        state: if transition_cpu_working {
            RouteState::Work
        } else {
            RouteState::Locked
        },
    }));

    format!(
        r#"<section class="architecture signal-pipeline" data-pipeline-route="single" data-current-stage="{}" data-proof-verified="{}">
<div class="architecture-head">
<div class="architecture-title"><h2>NANDO MACHINE · МАРШРУТ СИГНАЛА</h2><p>один вход, один текущий владелец на каждом этапе, одна строгая передача до CPU</p></div>
<div class="architecture-state"><span class="state-chip live">ОБЫЧНЫЙ ВХОД</span><span class="state-chip proven">КОНТРОЛИРУЕМОЕ ДОКАЗАТЕЛЬСТВО</span><span class="state-chip locked">AUTHORITY ВЫКЛЮЧЕНА</span></div>
</div>
<div class="identity-line"><span><b>МОДЕЛЬ</b> {}</span><span><b>РАЗВЁРНУТО</b> {} · {}</span><span><b>ЖИВАЯ LINEAGE</b> partition.v{} · поколение {}</span><span><b>ДОКАЗАТЕЛЬСТВО</b> {}</span></div>
<div class="pipeline-legend"><span><b>ЖИВОЙ ПОТОК</b> обычный трафик проходит этап</span><span><b>РАБОТАЕТ</b> проверенный transition-маршрут исполняется на CPU</span><span><b>ТОЛЬКО ДОКАЗАТЕЛЬСТВО</b> способность доказана без живого покрытия</span><span><b>БЛОК</b> обычный сигнал остановлен</span><span><b>ЗАКРЫТО</b> authority отсутствует</span></div>
<div class="current-blocker"><span class="blocker-label">ТЕКУЩИЙ БЛОКЕР</span><strong>{}</strong><p>{}</p></div>
<div class="pipeline-stack">{}</div>
<div class="terminal-rule">один этап = один владелец | владение передаётся только через типизированные квитанции | authority выдаёт только независимый допуск | нет evidence = ABSTAIN</div>
</section>"#,
        escape(current_stage),
        proof.verified,
        escape(model_label),
        escape(build_id),
        escape(&compact(build_commit)),
        live.partition,
        live.generation,
        escape(&proof_boundary),
        escape(current_blocker),
        escape(&current_reason),
        pipeline,
    )
}

fn live_downstream_text(natural_operator_live: bool, operation: &str) -> String {
    if natural_operator_live {
        format!("{operation} получает обычный трафик")
    } else {
        format!("{operation} не работает: живой сигнал остановлен на L3")
    }
}

fn render_stage(stage: &PipelineStage) -> String {
    format!(
        r#"<article class="pipeline-stage {}" data-stage="{}" data-owner="{}" data-live-flow="{}">
<div class="pipeline-stage-head"><span class="pipeline-index">{}</span><div><h3>{}</h3><p class="stage-owner"><b>ВЛАДЕЛЕЦ</b> {}</p></div><span class="state-chip {}">{}</span></div>
<dl class="pipeline-diagnostics">
<div class="diagnostic-row input"><dt>ВХОД</dt><dd>{}</dd></div>
<div class="diagnostic-row live"><dt>ЖИВОЙ</dt><dd>{}</dd></div>
<div class="diagnostic-row proof"><dt>ДОКАЗ</dt><dd>{}</dd></div>
<div class="diagnostic-row diagnostic"><dt>ДИАГ</dt><dd>{}</dd></div>
<div class="diagnostic-row output"><dt>ВЫХОД</dt><dd>{}</dd></div>
</dl>
</article>"#,
        stage.state.class(),
        escape(stage.id),
        escape(&stage.owner),
        stage.state.live_flow(),
        escape(stage.id),
        escape(stage.title),
        escape(&stage.owner),
        stage.state.class(),
        stage.state.label(),
        escape(&stage.input),
        escape(&stage.live),
        escape(&stage.proof),
        escape(&stage.diagnostic),
        escape(&stage.output),
    )
}

fn handoff(from: &str, to: &str, label: &str, live: bool) -> String {
    format!(
        r#"<div class="pipeline-handoff {}" data-handoff="{}-{}"><span class="handoff-line"></span><span>{}</span></div>"#,
        if live { "live" } else { "proof" },
        escape(from),
        escape(to),
        escape(label),
    )
}

fn live_signal_break(live: &LiveSignalView<'_>) -> String {
    let blocker = natural_blocker_text(live);
    format!(
        r#"<div class="pipeline-break" data-blocker-stage="L3"><strong>ЖИВОЙ СИГНАЛ ОСТАНОВЛЕН ЗДЕСЬ</strong><span>{}</span><span>Ниже этой границы синий маршрут показывает только контролируемое доказательство STOP-F8.</span></div>"#,
        escape(&blocker),
    )
}

fn natural_blocker_text(live: &LiveSignalView<'_>) -> String {
    if live.physical_adapters == 0 {
        return format!(
            "текущий winner не восстановил physical adapter; runtime parity и routing не начинаются (raw future {}/32)",
            live.future
        );
    }
    if live.matching == 0 {
        return "physical adapter есть, но нет ни одной runtime-parity строки, совпавшей с программой и маршрутом"
            .to_owned();
    }
    if live.after_watermark == 0 {
        return format!(
            "{} runtime-parity строк совпали, но новых наблюдений после frozen watermark нет",
            live.matching
        );
    }
    if live.independent == 0 {
        let support_rejects = live
            .support_frame_rejects
            .saturating_add(live.support_session_rejects)
            .saturating_add(live.support_intent_rejects)
            .saturating_add(live.support_event_rejects);
        return format!(
            "{} post-freeze наблюдений не дали независимого evidence: {} отклонены как support reuse (session {}); нужны новые независимые сессии",
            live.after_watermark, support_rejects, live.support_session_rejects,
        );
    }
    if live.consistent == 0 {
        return format!(
            "{} независимых строк не прошли program consistency; rejects {}",
            live.independent, live.program_mismatch_rejects,
        );
    }
    if live.routed == 0 {
        return format!(
            "{} program-consistent строк не прошли фазовую маршрутизацию; rejects {}",
            live.consistent, live.route_mismatch_rejects,
        );
    }
    format!(
        "доказанный маршрут сформирован, но immutable future ещё {}/32; блокер {}",
        live.future, live.blocker,
    )
}

fn authority_boundary(proof: &ProofSummary) -> String {
    let detail = if proof.verified {
        "SHADOW_READY является контролируемым кандидатом и не может допустить себя сам. Нужны естественное evidence и отдельная лицензия authority."
    } else {
        "Authority остаётся закрытой, потому что квитанции контролируемого доказательства недоступны."
    };
    format!(
        r#"<div class="authority-boundary" data-authority="false"><strong>ГРАНИЦА AUTHORITY ЕСТЕСТВЕННОГО ОПЕРАТОРА</strong><span>{}</span></div>"#,
        escape(detail),
    )
}

fn format_ratio_milli(value: u64) -> String {
    format!("{}.{:01}%", value / 10, value % 10)
}

fn compact(value: &str) -> String {
    match value.get(..12) {
        Some(prefix) if value.len() > 12 => format!("{prefix}..."),
        _ => value.to_owned(),
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proof(verified: bool) -> ProofSummary {
        ProofSummary {
            verified,
            failure: (!verified).then(|| "tampered receipt".into()),
            f5_commit: "1234567890abcdef".into(),
            f7_receipt_date: "2026-07-22".into(),
            f7_queue_max: 48,
            f7_no_match_p99_ns: 100,
            f7_matched_p99_ns: 200,
            f8_rss_bytes: 10,
            f8_rss_target_bytes: 16,
            f8_resource_observations: 12,
            f8_no_match_p99_max_ns: 195_340,
            f8_provider_records: 4,
            f8_verified_receipts: 3,
            f8_full_phase_gain: 3,
            f8_search_gain: 0,
            f8_external_verdict: "SHADOW_READY".into(),
            f8_commitments_sha256:
                "84e0c8fc735d5c7397757fbbedd40bf8c2f49a17df9d33a754d6c8989fdb7c7f".into(),
            f8_matched_p99_max_ns: 648_010,
            f8_hard_max_ns: 690_741,
            f8_hot_rss_bytes: 10_493_952,
        }
    }

    fn live() -> LiveSignalView<'static> {
        LiveSignalView {
            partition: 16,
            generation: 7,
            transitions: 18_721,
            support: 32,
            physical_adapters: 1,
            matching: 19,
            matching_sessions: 4,
            after_watermark: 16,
            independent: 16,
            consistent: 16,
            routed: 16,
            future: 11,
            support_frame_rejects: 0,
            support_session_rejects: 0,
            support_intent_rejects: 0,
            support_event_rejects: 0,
            program_mismatch_rejects: 0,
            route_mismatch_rejects: 0,
            blocker: "future_rows_below_32",
            admission_verdict: "BLOCK",
            admission_blocker: "no_candidate",
            admission_blocker_stage: "runtime_parity",
            admission_age_seconds: 5,
            admission_relation_candidates: 1,
            admission_future_rows: 11,
            admission_runtime_parity_cases: 22,
            active_packages: 0,
            active_transition_profiles: 5,
            verified_local_accepts: 38,
            call_saving_share_milli: 4,
            input_token_saving_share_milli: 7,
            online_ready: true,
            capture_phase: "ready_hash_only",
            capture_records: 4,
            capture_captured: 0,
            capture_censored: 0,
            capture_publish_sequence: 13,
            capture_last_error: "",
            shadow_phase: "ready_shadow",
            shadow_submitted: 0,
            shadow_evaluated: 0,
            shadow_verified: 0,
            shadow_parity_mismatches: 0,
        }
    }

    fn manifest() -> Value {
        serde_json::json!({
            "build_id": "build-1",
            "git_commit": "abcdef1234567890"
        })
    }

    #[test]
    fn map_is_one_ordered_pipeline_with_one_owner_per_stage() {
        let html = render(&live(), &proof(true), &manifest(), "gpt-test");
        let stages = [
            "L1", "L2", "L3", "L4", "L5", "L6", "L7", "L8", "L9", "L10", "L11", "CPU",
        ];
        let mut previous = 0;
        for id in stages {
            let marker = format!("data-stage=\"{id}\"");
            let position = html.find(&marker).expect("stage must exist");
            assert!(position >= previous, "{id} is out of order");
            previous = position;
        }
        assert!(html.contains("data-pipeline-route=\"single\""));
        assert_eq!(html.matches("data-owner=").count(), stages.len());
        assert!(!html.contains("CURRENT LIVE OBSERVER BRANCH"));
        assert!(!html.contains("CONTROLLED F8 PROOF BRANCH"));
    }

    #[test]
    fn ordinary_signal_stops_at_natural_discovery_only() {
        let html = render(&live(), &proof(true), &manifest(), "gpt-test");

        assert!(html.contains("data-current-stage=\"L3\""));
        assert!(html.contains(
            "data-stage=\"L1\" data-owner=\"nando-nginx-gateway\" data-live-flow=\"true\""
        ));
        assert!(html.contains(
            "data-stage=\"L2\" data-owner=\"nando-transition-serving\" data-live-flow=\"true\""
        ));
        assert!(html.contains(
            "data-stage=\"L3\" data-owner=\"nando-operator-learning\" data-live-flow=\"false\""
        ));
        assert!(html.contains("data-blocker-stage=\"L3\""));
        assert_eq!(html.matches("ЖИВОЙ СИГНАЛ ОСТАНОВЛЕН ЗДЕСЬ").count(), 1);
        assert!(html.contains("контролируемое доказательство STOP-F8"));
        assert!(html.contains(
            "data-stage=\"L4\" data-owner=\"nando-operator-learning\" data-live-flow=\"false\""
        ));
        assert!(html.contains(
            "data-stage=\"CPU\" data-owner=\"nando-transition-serving\" data-live-flow=\"true\""
        ));
    }

    #[test]
    fn controlled_proof_does_not_upgrade_natural_or_authority_status() {
        let html = render(&live(), &proof(true), &manifest(), "gpt-test");

        assert!(html.contains("STOP-F8 SHADOW_READY"));
        assert!(html.contains("НЕ ОЦЕНЕНО в STOP-F8"));
        assert!(html.contains("data-authority=\"false\""));
        assert!(html.contains("Естественный кандидат STOP-F8 остаётся authority=false"));
        assert!(html.contains("естественных ACTIVE response-пакетов 0"));
        assert!(!html.contains(
            "data-stage=\"L4\" data-owner=\"nando-operator-learning\" data-live-flow=\"true\""
        ));
    }

    #[test]
    fn blocker_explains_support_session_reuse_before_future_threshold() {
        let mut live = live();
        live.matching = 64;
        live.matching_sessions = 5;
        live.after_watermark = 32;
        live.independent = 0;
        live.consistent = 0;
        live.routed = 0;
        live.future = 0;
        live.support_session_rejects = 32;

        let html = render(&live, &proof(true), &manifest(), "gpt-test");

        assert!(html.contains("32 post-freeze наблюдений"));
        assert!(html.contains("32 отклонены как support reuse"));
        assert!(html.contains("нужны новые независимые сессии"));
    }

    #[test]
    fn verified_transition_route_is_work_while_natural_authority_stays_locked() {
        let html = render(&live(), &proof(true), &manifest(), "gpt-test");

        assert!(html.contains(
            "class=\"pipeline-stage work\" data-stage=\"CPU\" data-owner=\"nando-transition-serving\" data-live-flow=\"true\""
        ));
        assert!(html.contains("class=\"state-chip work\">РАБОТАЕТ"));
        assert!(html.contains("проверенных локальных исполнений 38"));
        assert!(html.contains("активных transition-профилей 5"));
        assert!(html.contains("обычный трафик на CPU 0.4%"));
        assert!(html.contains("экономия токенов 0.7%"));
        assert!(html.contains("ГРАНИЦА AUTHORITY ЕСТЕСТВЕННОГО ОПЕРАТОРА"));
        assert!(html.contains("data-authority=\"false\""));
    }

    #[test]
    fn cpu_route_stays_locked_without_verified_transition_work() {
        let mut live = live();
        live.verified_local_accepts = 0;
        let html = render(&live, &proof(true), &manifest(), "gpt-test");

        assert!(html.contains(
            "class=\"pipeline-stage locked\" data-stage=\"CPU\" data-owner=\"nando-transition-serving\" data-live-flow=\"false\""
        ));
    }

    #[test]
    fn invalid_proof_receipt_locks_downstream_proof_claims() {
        let html = render(&live(), &proof(false), &manifest(), "gpt-test");

        assert!(html.contains("data-current-stage=\"PROOF\""));
        assert!(html.contains("ПРОВЕРКА ДОКАЗАТЕЛЬНЫХ КВИТАНЦИЙ"));
        assert!(html.contains(
            "data-stage=\"L4\" data-owner=\"nando-operator-learning\" data-live-flow=\"false\""
        ));
        assert!(!html.contains("STOP-F8 SHADOW_READY"));
    }

    #[test]
    fn diagnostics_escape_live_blocker_text() {
        let mut live = live();
        live.blocker = "<bad&blocker>";
        let html = render(&live, &proof(true), &manifest(), "gpt-test");

        assert!(html.contains("&lt;bad&amp;blocker&gt;"));
        assert!(!html.contains("<bad&blocker>"));
    }
}

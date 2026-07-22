use super::f8_final_receipt::F8FinalStatus;
use super::f8_resource_receipt::F8ResourceStatus;
use super::receipt::PipelineStatus;

pub(super) fn verified_panel(
    status: &PipelineStatus,
    resource: &F8ResourceStatus,
    final_status: &F8FinalStatus,
) -> String {
    let commit = status
        .f5_implementation_commit
        .get(..12)
        .unwrap_or(&status.f5_implementation_commit);
    let mut stages = String::new();
    stages.push_str(&stage(
        "f5-a",
        "5A",
        "Исполнимый артефакт оператора",
        "OperatorArtifactV1 владеет неизменяемым законом эффекта и скомпилированной программой.",
        "артефакт",
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("типизированный артефакт + корни доказательства"));
    stages.push_str(&stage(
        "f5-b",
        "5B",
        "Канонический runtime-контекст",
        "Нормализует входящую поверхность без выдачи execution authority.",
        "контекст",
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("каноническая структурная поверхность"));
    stages.push_str(&stage(
        "f5-c",
        "5C",
        "Структурная маршрутизация и связывание ролей",
        "Выбирает ограниченный набор структурных кандидатов и строго разрешает runtime-роли.",
        "связывание",
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("единственное окружение ролей"));
    stages.push_str(&stage(
        "f5-d",
        "5D",
        "Связывание capability и действия",
        "Связывает семантический режим с объявленной физической capability и действием.",
        "связывание",
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("физическое действие, принадлежащее победителю"));
    stages.push_str(&stage(
        "f5-e",
        "5E",
        "Actor и Operator VM в SHADOW",
        "Исполняет скомпилированный режим в SHADOW и доказывает parity actor/VM.",
        "VM в SHADOW",
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("ограниченные кандидаты, ранжированные фазой"));
    stages.push_str(&stage(
        "f5-f",
        "5F",
        "Интеграция фазы",
        "Фазовое ранжирование сохраняет безопасность; этот корпус пока не доказал сокращение поиска.",
        &status.phase_search_gain,
        "НАБЛЮДЕНИЕ",
        "wait",
    ));
    stages.push_str(&edge("проекция трафика + закреплённые поколения"));
    stages.push_str(&stage(
        "f5-g",
        "5G",
        "Входящий трафик в SHADOW",
        "Учитывает обычное окно и соблюдает жёсткий потолок трафика без локальных допусков.",
        &format!(
            "{} / {} обычных",
            status.accounted_rows, status.ordinary_rows
        ),
        "НАБЛЮДЕНИЕ",
        "wait",
    ));
    stages.push_str(&facts(status));
    stages.push_str(
        r#"<div class="research-boundary" data-edge="f5-to-f6"><span class="tree-glyph">│</span><strong>ПОЛНЫЙ КОНТРОЛИРУЕМЫЙ СИГНАЛ F5 ДО ВХОДА F6 ПОДТВЕРЖДЁН</strong><span>это не заявление о полном production-маршруте</span></div>"#,
    );
    stages.push_str(&stage(
        "f6",
        "F6",
        "Независимый верификатор",
        "Независимо восстанавливает сцену запроса, роли, capability, действие и ожидаемый результат.",
        &format!(
            "состязательных {} · p99 {} нс",
            status.f6_integration_pass, status.f6_matched_p99_ns
        ),
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge(
        "непрозрачная квитанция верификатора · authority=false",
    ));
    stages.push_str(
        r#"<div class="research-boundary" data-edge="f6-to-f7"><span class="tree-glyph">│</span><strong>ПОЛНЫЙ КОНТРОЛИРУЕМЫЙ ПУТЬ ДОКАЗАТЕЛЬСТВА F5-F6 ПОДТВЕРЖДЁН</strong><span>authority поколения остаётся false</span></div>"#,
    );
    stages.push_str(&stage(
        "f7-a",
        "7A",
        "Идентичность поколения и restart-пакет",
        "Один канонический ID поколения связывает неизменяемые артефакты и восстановленную маршрутизацию.",
        "канонический перезапуск",
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("разделы, принадлежащие поколению"));
    stages.push_str(&stage(
        "f7-b",
        "7B",
        "Журнал support и frozen-future",
        "Разделяет support, future, цензурированные исходы и watermark после заморозки.",
        "повтор между разделами запрещён",
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("поколение + lineage + корень запроса"));
    stages.push_str(&stage(
        "f7-c",
        "7C",
        "Квитанция верификатора, связанная с поколением",
        "Связывает независимый вердикт F6 с одним поколением и точной идентичностью capture.",
        "подмена и переименование запрещены",
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("самопроверяемый checkpoint"));
    stages.push_str(&stage(
        "f7-d",
        "7D",
        "Атомарное хранение и восстановление",
        "Публикует чередующиеся слоты поколений и восстанавливает только монотонное байт-идентичное состояние.",
        "fsync + rename + восстановление",
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("точное соединение с capture запроса провайдера"));
    stages.push_str(&stage(
        "f7-e",
        "7E",
        "Контролируемое поколение в SHADOW",
        "Загружается после привязки HTTP, закрепляет поколение до постановки в очередь, запускает F5 и независимо проверяет через F6.",
        &format!(
            "очередь <= {} · p99 {} нс",
            status.f7_queue_max, status.f7_matched_p99_ns
        ),
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&f7_facts(status));
    stages.push_str(
        r#"<div class="research-boundary" data-edge="f7-to-f8"><span class="tree-glyph">│</span><strong>ПОЛНЫЙ КОНТРОЛИРУЕМЫЙ ПУТЬ ДОКАЗАТЕЛЬСТВА F5-F7 ПОДТВЕРЖДЁН</strong><span>живой producer, допуск и authority не заявлены</span></div>"#,
    );
    stages.push_str(&stage(
        "f8-0",
        "8-0",
        "Реальные ресурсы production-аллокатора",
        "Отделяет удержание компилятора от сохранённого горячего реестра при уже развёрнутой политике аллокатора.",
        &format!(
            "RSS production-политики {} / цель {} Б · запусков {}",
            resource.max_peak_rss_delta_bytes,
            resource.rss_target_bytes,
            resource.resource_observations,
        ),
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("квитанция запроса провайдера только с хешами"));
    stages.push_str(&stage(
        "f8-a",
        "8-A",
        "Владелец живого capture провайдера",
        "Сохраняет ограниченную provenance запроса без удержания или повторного хеширования сырого payload провайдера.",
        &format!("устойчивых записей {}", final_status.provider_records),
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("проверенная квитанция, принадлежащая поколению"));
    stages.push_str(&stage(
        "f8-b",
        "8-B",
        "Устойчивый журнал SHADOW",
        "Соединяет квитанции capture, поколения, actor и независимого верификатора без semantic authority.",
        &format!("проверенных квитанций {}", final_status.verified_receipts),
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("неизменяемые входы реконструкции"));
    stages.push_str(&stage(
        "f8-c",
        "8-C",
        "Реконструкция внешнего допуска",
        "Восстанавливает одного кандидата из неизменяемого checkpoint, индекса capture и байтов журнала SHADOW.",
        &final_status.external_verdict,
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("фазовое evidence, принадлежащее runtime"));
    stages.push_str(&stage(
        "f8-d",
        "8-D",
        "Причинный контроль и бюджеты трафика",
        "Повторно вычисляет полные и аблированные фазовые исходы по точному устойчивому набору квитанций трафика.",
        &format!(
            "прирост применимости {} · прирост поиска {}",
            final_status.full_phase_gain, final_status.search_gain
        ),
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge(
        "точный после перезапуска контролируемый живой SHADOW",
    ));
    stages.push_str(&stage(
        "f8-e",
        "8-E",
        "Контролируемый живой SHADOW",
        "Проходит через реальный Rust HTTP-сервис и независимо восстанавливает SHADOW_READY после перезапуска.",
        &format!(
            "проверено {}/{}",
            final_status.verified_receipts, final_status.verified_receipts
        ),
        "ПРОЙДЕНО",
        "pass",
    ));
    stages.push_str(&edge("требуется отдельная лицензия authority"));
    stages.push_str(&stage(
        "cpu",
        "CPU",
        "Активное исполнение",
        "Контролируемое evidence не может выдать production authority или заявить покрытие естественным оператором.",
        "ACTIVE=0 · authority=false",
        "ЗАКРЫТО",
        "locked",
    ));

    format!(
        r#"<section class="architecture research-architecture" data-research-status="stop-f8-pass-authority-false">
<div class="architecture-head">
<div class="architecture-title"><h2>ИССЛЕДОВАТЕЛЬСКИЙ КОНТУР ОПЕРАТОРА</h2><p>артефакт -&gt; связывание -&gt; VM -&gt; верификатор -&gt; поколение -&gt; допуск F8</p></div>
<div class="architecture-state"><span class="state-chip pass">F5 ЗАВЕРШЁН</span><span class="state-chip pass">F6 ЗАВЕРШЁН</span><span class="state-chip pass">F7 ЗАВЕРШЁН</span><span class="state-chip pass">F8 ЗАВЕРШЁН</span><span class="state-chip locked">AUTHORITY ВЫКЛЮЧЕНА</span><span class="architecture-meta">F5 {} · квитанция F7 {}</span></div>
</div>
<div class="flow-tree">{}</div>
<div class="terminal-rule">контролируемый путь F5 -&gt; F8 в SHADOW подтверждён | p99 без совпадения/с совпадением {}/{} нс | жёсткий максимум {} нс | горячий RSS {} Б | естественный оператор НЕ ОЦЕНЕН | authority=false</div>
</section>"#,
        escape(commit),
        escape(&status.f7_receipt_date),
        stages,
        final_status.no_match_p99_max_ns,
        final_status.matched_p99_max_ns,
        final_status.hard_max_ns,
        final_status.hot_rss_bytes,
    )
}

fn facts(status: &PipelineStatus) -> String {
    format!(
        r#"<div class="research-facts">
<span>проекция {}/{}</span><span>естественный replay {}</span><span>F5 без совпадения p99 {} / цель {} нс</span><span>F5 с совпадением p99 {} / цель {} нс</span><span>жёсткий потолок F5 {} нс ПРОЙДЕН</span><span>RSS {} / цель {} Б</span><span>F6 без совпадения p99 {} нс</span><span>F6 с совпадением p99 {} нс</span><span>максимум F6 {} нс</span>
</div>"#,
        status.projection_controls_passed,
        status.projection_controls_total,
        escape(&status.organic_runtime_replay),
        status.no_match_p99_ns,
        status.no_match_target_ns,
        status.matched_shadow_p99_ns,
        status.matched_target_ns,
        status.hard_ceiling_ns,
        status.rss_delta_bytes,
        status.rss_target_bytes,
        status.f6_no_match_p99_ns,
        status.f6_matched_p99_ns,
        status.f6_hard_max_ns,
    )
}

fn f7_facts(status: &PipelineStatus) -> String {
    format!(
        r#"<div class="research-facts">
<span>точное соединение capture</span><span>поколение запроса закреплено</span><span>сохранено сырых данных 0 Б</span><span>локальных допусков 0</span><span>F7 без совпадения p99 {} нс</span><span>F7 с совпадением p99 {} нс</span><span>максимум F7 {} нс</span><span>консервативный RSS F5 {} / цель {} Б · НАБЛЮДЕНИЕ</span>
</div>"#,
        status.f7_no_match_p99_ns,
        status.f7_matched_p99_ns,
        status.f7_hard_max_ns,
        status.f7_rss_delta_bytes,
        status.f7_rss_target_bytes,
    )
}

fn stage(
    id: &str,
    step: &str,
    title: &str,
    logic: &str,
    metric: &str,
    label: &str,
    class: &str,
) -> String {
    let branch = if id == "cpu" { "└─" } else { "├─" };
    format!(
        r#"<div class="terminal-stage {}" data-rd-stage="{}" title="{}">
<div class="terminal-line"><span class="tree-glyph">{}</span><span class="stage-index">[{}]</span><strong class="stage-title">{}</strong><span class="stage-metric">{}</span><span class="state-chip {}">{}</span></div>
</div>"#,
        escape(class),
        escape(id),
        escape(logic),
        branch,
        escape(step),
        escape(title),
        escape(metric),
        escape(class),
        escape(label),
    )
}

fn edge(label: &str) -> String {
    format!(
        "<div class=\"terminal-edge\"><span class=\"tree-glyph\">│</span>{}</div>",
        escape(label)
    )
}

pub(super) fn unavailable_panel(error: &str) -> String {
    format!(
        r#"<section class="architecture research-architecture" data-research-status="unavailable">
<div class="architecture-head">
<div class="architecture-title"><h2>ИССЛЕДОВАТЕЛЬСКИЙ КОНТУР ОПЕРАТОРА</h2><p>статус по доказательным квитанциям недоступен</p></div>
<div class="architecture-state"><span class="state-chip block">СТАТУС ИССЛЕДОВАНИЯ НЕДОСТУПЕН</span><span class="state-chip locked">F8 ЗАКРЫТ</span></div>
</div>
<div class="flow-tree"><div class="terminal-line terminal-failure"><span class="tree-glyph">└─</span><strong>СТРОГИЙ ОТКАЗ</strong><span>{}</span></div></div>
<div class="terminal-rule">нет квитанции = нет заявления об успехе | authority остаётся false</div>
</section>"#,
        escape(error)
    )
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

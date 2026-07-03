# Domain DSL V1

Domain DSL V1 is the compact domain-description layer for WavePredictor Task
Factory.

It is not a task dataset, not a trained model, and not runtime answer authority.

## Locked First Proving Domain

```text
linux_networking_vpn
```

Russian translation:

```text
первый зафиксированный полигон: Linux / сеть / VPN / диагностика
```

Lock status:

```text
status = locked_first_proving_domain
lock_scope = first_proving_domain
locked_on = 2026-06-30
```

## Files

```text
domains.json
validate_domain_dsl_v1.py
linux_networking_vpn/domain_lock.json
linux_networking_vpn/entities.json
linux_networking_vpn/operators.json
linux_networking_vpn/templates.json
linux_networking_vpn/negative_rules.json
```

## Meanings

```text
domains = domain registry
entities = reusable domain objects and surface forms
operators = transition rules that tasks should pressure
templates = compact task blueprints
negative_rules = hard-near-negative mistake families
domain_lock = selected scope, included boundaries, excluded boundaries, task gates
```

Russian translation:

```text
domains = реестр доменов
entities = сущности домена
operators = операторы перехода
templates = шаблоны задач
negative_rules = правила похожих ошибочных ответов
domain_lock = выбранная область, что входит, что не входит, какие гейты обязательны
```

## Lock Boundary

Included:

```text
VPN connectivity and tunnel
Linux routes and route scope
DNS and internal zones
Firewall, ACL, and port filtering
Auth, TLS, RADIUS, and time
Safe troubleshooting: snapshot before mutation, minimal action, refusal on missing evidence
```

Russian translation:

```text
VPN-подключение и туннель
Linux-маршруты и область маршрутизации
DNS и внутренние зоны
Firewall, ACL и фильтрация портов
Авторизация, TLS, RADIUS и время
Безопасная диагностика: снимок до мутации, минимальное действие, отказ при нехватке evidence
```

Excluded:

```text
General Linux administration
Windows networking
Full cloud platforms
Broad cybersecurity
General chat
Using DSL as runtime answer authority
```

## Domain Pack V1

Step 5 is closed for the first proving domain.

```text
entities: 48
operators: 24
negative_rules: 24
templates: 24
operator_template_coverage: complete
```

Russian translation:

```text
entities = словарь объектов и evidence-форм внутри домена
operators = переходы вида "по этим признакам следующий правильный ход такой"
negative_rules = близкие неправильные ходы
templates = компактные чертежи задач для Task Factory
```

## Validation

```bash
python3 data/domain_dsl_v1/validate_domain_dsl_v1.py
```

## Boundary

Domain DSL metadata is for corpus generation, balancing, and shortcut audits.
It must not be fed to L3 as answer authority.

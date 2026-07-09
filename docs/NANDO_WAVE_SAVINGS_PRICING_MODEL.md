# Nando Wave Savings Pricing Model

Current technical contract:

```text
docs/NANDO_WAVE_STREAMING_ARCHITECTURE_CONTRACT.md
```

Commercial boundary on 2026-07-06:

```text
Current best internal shadow frontier:
  calls_saved: 22.3177%
  tokens_saved: 72.0541%
  false_accepts: 0

But:
  local_accept_enabled: false
  market_money_claim_allowed: false
  provider billing proof is still missing
```

Therefore pricing can be discussed as a model, but sales claims must be based
on verified provider-billing savings over the customer's baseline.

Да. Надо быть адекватными: брать не “лицензию из воздуха”, а долю от проверенной дополнительной экономии.

  Формула:

  Nando fee = verified incremental savings over baseline * share

  Где baseline:

  текущий exact cache / prompt cache / semantic cache / router клиента

  Главный Вердикт
  Я бы ставил так:

  малые клиенты:        не гнаться, мало денег
  хороший первый рынок: $50k-$500k/month LLM spend
  крупные клиенты:      $500k+/month, но длинные продажи

  нормальная доля Nando:
    20-30% от verified savings для пилота
    10-25% для долгого контракта

  Таблица Цен

   Клиент                             LLM/AI spend в месяц    Реальная цель экономии    Экономия $/мес                  Сколько брать           Nando MRR    Клиент оставляет
  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━
   Малый AI startup                               $5k-$20k                    15-25%          $750-$5k                audit fee, не %    $2k-$5k one-time           почти всё
  ─────────────────────────────────  ──────────────────────  ────────────────────────  ────────────────  ─────────────────────────────  ──────────────────  ──────────────────
   Growing AI SaaS                               $20k-$50k                    20-30%          $4k-$15k                 25-30% savings           $1k-$4.5k          $3k-$10.5k
  ─────────────────────────────────  ──────────────────────  ────────────────────────  ────────────────  ─────────────────────────────  ──────────────────  ──────────────────
   Хороший первый клиент                        $50k-$100k                    20-35%         $10k-$35k                    25% savings        $2.5k-$8.75k          $7.5k-$26k
  ─────────────────────────────────  ──────────────────────  ────────────────────────  ────────────────  ─────────────────────────────  ──────────────────  ──────────────────
   Сильный B2B SaaS / agent                    $100k-$250k                    20-35%       $20k-$87.5k                 20-25% savings            $4k-$22k           $16k-$65k
   platform
  ─────────────────────────────────  ──────────────────────  ────────────────────────  ────────────────  ─────────────────────────────  ──────────────────  ──────────────────
   Enterprise AI platform                        $250k-$1M                    15-30%      $37.5k-$300k                 15-25% savings          $5.6k-$75k          $32k-$225k
  ─────────────────────────────────  ──────────────────────  ────────────────────────  ────────────────  ─────────────────────────────  ──────────────────  ──────────────────
   Очень крупный enterprise                        $1M-$5M                    10-25%      $100k-$1.25M           10-20% savings + cap          $10k-$250k           $90k-$1M+
  ─────────────────────────────────  ──────────────────────  ────────────────────────  ────────────────  ─────────────────────────────  ──────────────────  ──────────────────
   Gateway/OEM партнёр                         чужой объём                   зависит           зависит    5-15% savings или rev-share         $25k-$500k+             зависит

  Самый Адекватный Первый Прайсинг

  Trace audit:
    $5k-$15k
    можно зачесть в пилот

  Shadow-mode pilot:
    $15k-$50k setup
    +
    20-30% verified incremental savings за 3 месяца

  Production:
    15-25% verified incremental savings
    minimum $3k-$10k/month

  Для крупного клиента:

  Enterprise production:
    10-20% verified incremental savings
    minimum $25k/month
    annual cap или volume discount

  Почему Так Адекватно
  Потому что в cost-optimization рынке уже есть gainsharing: поставщик берёт долю от реально полученной экономии. ProsperOps прямо описывается как сервис, который берёт долю
  realized savings, а не процент cloud spend: https://www.prosperops.com/blog/price-vs-value-selecting-the-best-aws-cost-optimization-tool/

  Performance/outcome pricing в SaaS считается нормальным, если ROI можно ясно измерить, но главный риск — attribution и доверие к метрике. Это ровно наша ситуация:
  https://www.sage.com/en-us/blog/saas-pricing-models/ и https://www.lek.com/insights/saas-pricing/rise-outcome-based-pricing-saas-aligning-value-cost

  Правильная Коммерческая Формула

  Customer keeps at least 70-80% of verified savings.
  Nando takes 20-30% during pilot.
  Long-term share drops to 10-25%, depending on volume.

  То есть:

  если мы сэкономили $30k/month:
    Nando берёт $6k-$9k
    клиент оставляет $21k-$24k

  Это адекватно.

  Но если клиент экономит только $2k/month, брать 30% бессмысленно:

  $600/month не покрывает продажу и поддержку

  Поэтому малым клиентам:

  self-serve / audit / фиксированный пакет

  Кого Брать В Первую Очередь

   Приоритет    Кого                                                    Почему
  ━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
   1            AI coding / agent tools с $50k-$500k/month API spend    много повторяемых operator-shaped calls
  ───────────  ──────────────────────────────────────────────────────  ─────────────────────────────────────────
   2            B2B SaaS с support/workflow agents                      понятная экономия и безопасный fallback
  ───────────  ──────────────────────────────────────────────────────  ─────────────────────────────────────────
   3            Enterprise platform teams                               высокая боль, но длинная продажа
  ───────────  ──────────────────────────────────────────────────────  ─────────────────────────────────────────
   4            LLM gateway vendors                                     можно стать модулем в их stack
  ───────────  ──────────────────────────────────────────────────────  ─────────────────────────────────────────
   5            Малые startups                                          только как быстрые кейсы, денег мало

  Минимальный Deal, Ниже Которого Не Надо
  Я бы поставил нижнюю границу:

  минимальный paid audit: $5k
  минимальный pilot: $15k
  минимальный production: $3k/month

  Иначе мы утонем в поддержке.

  Расчёт Для Клиента На $100k/month

  текущий LLM spend:              $100k/month
  Nando incremental saving:       25%
  gross saving:                   $25k/month

  Nando share 25%:
    Nando получает:               $6.25k/month
    клиент оставляет:             $18.75k/month

  год:
    Nando ARR:                    $75k
    клиент net saving:            $225k

  Это нормально.

  Расчёт Для Клиента На $1M/month

  текущий LLM spend:              $1M/month
  Nando incremental saving:       20%
  gross saving:                   $200k/month

  Nando share 15%:
    Nando получает:               $30k/month
    клиент оставляет:             $170k/month

  год:
    Nando ARR:                    $360k
    клиент net saving:            $2.04M

  Это уже enterprise deal.

  Рыночная Опора
  Почему это не фантазия:

  - Zylo пишет, что средняя организация тратит около $1.2M/year на AI-native apps, рост 108% год к году: https://zylo.com/blog/ai-cost/
  - Menlo Ventures оценивал model API spend в $8.4B: https://menlovc.com/perspective/2025-mid-year-llm-market-update/
  - UBS/Business Insider пишет, что около 60% enterprise throttling AI spend, то есть боль уже дошла до CFO/CTO:
    https://www.businessinsider.com/ubs-enterprises-ai-spending-tokens-2026-7

  - OpenAI pricing показывает, что output/reasoning tokens дорогие, а cached input решает только часть проблемы: https://developers.openai.com/api/docs/pricing
  - Langfuse Enterprise уже стоит от $2499/month, то есть рынок платит за LLM infra tooling: https://langfuse.com/pricing
  - LiteLLM/Portkey enterprise pricing кастомный для high-volume production workloads: https://docs.litellm.ai/docs/enterprise и https://portkey.ai/pricing

  Итоговая Рекомендация
  Не продавать “лицензию $20k/month” первой фразой.

  Продавать так:

  Мы берём 20-30% от доказанной дополнительной экономии LLM-cost.
  Клиент всегда оставляет 70-80%.
  Если экономии нет — большой оплаты нет.

  Первый коммерческий пакет:

  Nando Savings Pilot

  setup: $10k-$25k
  success fee: 25% verified incremental savings
  duration: 90 days
  minimum production floor after pilot: $3k-$10k/month

  Это выглядит адекватно и не нагло: клиент платит только если мы реально режем его AI bill.

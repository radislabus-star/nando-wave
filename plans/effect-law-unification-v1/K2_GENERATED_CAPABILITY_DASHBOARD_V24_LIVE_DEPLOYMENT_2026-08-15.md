# K2 Generated Capability Dashboard V24 Live Deployment

Status: `CONTROL PLANE DEPLOYED / GENERATED CAPABILITY VISIBLE / NATURAL K2 NOT PROVED`

Date: `2026-08-15`

## Live Result

The control page now shows the bounded generated-environment result as a
separate block:

```text
Generated causal AI
|- Hidden effects learned       PASS
|- Explicit composition         PASS
|- Hidden representation        PASS
|- Confirm exact goals          2 / 2
|- Search evaluations           61 / 67 each
|  `- complete denominator      8,659 each
|- Negative controls            18 / 18
|- Production authority         FALSE
`- Natural K2                   NOT PROVED
```

The ordinary-traffic research row independently remains:

```text
K1 laws                         1 / 3
Law #2                          NOT PROVED
Natural K2                      NOT PROVED
```

Generated PASS therefore means that the learner demonstrated the bounded
capability. It does not issue a LawCertificate, enter the K1 registry, activate
a product package, mutate phase memory, or prove natural K2.

## Deployment Identity

```text
source commit
  1701c468fc2bd607486280c06b0500575cf44da1

source tree
  8dbb7041642705ffa13ea145e72224bcdbac645b

deployment receipt
  /var/lib/nando-wave/deployments/
    20260815T085454Z-1701c468fc2b/deployment-receipt.json

receipt root
  5332195d30bc7f9db7939f0d6cd84bbe62881f8ba7bf21f9018e1c026c13c7f0

release / installed / running SHA-256
  d447b899e4830adf2604b2c056a80375d5105b28764c90ccab80d1d0107ad090
```

The remote receipt is owned by `root:root`, mode `0400`; its stored root equals
the independently recomputed canonical payload root. The previous control
binary remains in its rollback directory.

## Runtime Preservation

```text
gateway-control            298415 -> 2304299   intentional restart
hot transition-serving    1816591              unchanged
cold response-learning     298492              unchanged
certification authority    150005              unchanged
Nginx / transport          682430              unchanged
local connector            2919                unchanged
service restart counters   0
survival check              15 seconds PASS
false accepts / parity      0 / 0
services                    3 / 3
```

Only `nando-gateway-control.service` was replaced and restarted. The data plane,
learner, authority, Nginx, and connector were not restarted.

## Verification

```text
nando-gateway-control tests              65 / 65 PASS
generated experimental-lab tests          5 / 5 PASS
nando-operator-learning --lib           444 / 444 PASS
workspace check --all-targets                 PASS
cargo fmt --all --check                       PASS
gateway-control strict Clippy                 PASS
source / evidence SHA bindings                PASS

desktop viewport                       1440 x 900
mobile viewport                         390 x 844
horizontal overflow                         0 / 0
page errors / console errors                 0 / 0
responsive widths                            5 / 5 PASS
temporary browser sessions after QA              0
```

The responsive evidence manifest is
`/home/ubu/Загрузки/agent-browser/responsive/20260815T085616.906021593-3919766-a85ba9ca-nando-k2-v24-responsive-nando-k2-v24/manifest.json`,
SHA-256
`89d8e0984d90149f7edd42272d9fad5702f41ca61933f6704b7bc7f9f52f6cfc`.

The machine-readable repository verification receipt is
`evidence/K2_GENERATED_CAPABILITY_DASHBOARD_V24/live-deployment-verification.v1.json`,
SHA-256
`af51dfdb8289bdddcd8025f9f742ff3b99811a720810f830501ec8586c1f00ef`.

## Boundary

This deployment makes the completed generated causal-AI result visible and
truthful. It does not change the natural discovery result: K1 remains `1 / 3`,
Law #2 remains unproved, and Natural K2 remains unproved.

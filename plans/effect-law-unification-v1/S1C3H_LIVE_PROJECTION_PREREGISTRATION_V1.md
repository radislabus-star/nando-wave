# S1C-3H Live Projection Preregistration V1

Status: `FROZEN CONTROL-PLANE PROJECTION / NO SCIENTIFIC PROMOTION`

## Claim Boundary

The immutable S1C-3H terminal state may establish only:

```text
decision recorder                 INSTALLED
S1C-4 natural census              COLLECTING
natural records at installation  0
K2 grounded meaning               CLOSED
scientific authority              FALSE
```

The source of truth is the rooted terminal state from transaction
`20260812T222900Z-6f83abf21c24-s1c3h-v1`, bound to deployment receipt root
`0647e5a6b96ffff8addb44f2bd6f57fa389aeca5a00cb7cc9a615837858dff3a`
and independent final verification root
`a124be09017176cb32e786ec64d0782f3c864cf4fcc9179d609f88a246340297`.

The control plane must reject any changed field or invalid root. The HTML may
render the admitted projection but cannot grant authority.

## Route

```text
immutable s1c3h-state.json
-> rooted exact parser
-> dashboard API projection
-> S1C operational panel
```

The decision census remains a separate source. Installation does not create a
goal, alternative, selected action, verified satisfaction, decision episode,
grounded meaning, K2 law, model-training permission, or phase-mutation
permission.

## Deployment Contract

Only gateway-control and the operational status sidecar may change. The
transition runtime, journal, Nginx, connector, learning, and certification
services must remain untouched. The sidecar is installed atomically before a
rollback-capable gateway-control restart. A failed readiness or API projection
restores both the previous binary and previous sidecar.

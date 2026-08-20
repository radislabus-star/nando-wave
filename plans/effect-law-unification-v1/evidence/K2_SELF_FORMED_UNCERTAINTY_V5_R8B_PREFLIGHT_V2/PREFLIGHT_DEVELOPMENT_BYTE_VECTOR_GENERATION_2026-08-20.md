# R8B Preimplementation Development Byte Vector Generation

Status: `PASS / MIRROR BYTE VECTOR ONLY / NO IMPLEMENTATION OR EXECUTION AUTHORITY`

## Bound Inputs

```text
source HEAD         bdcae5351c7de75f325b0ebe752804066823cc38
Cargo.lock SHA-256  9328508784d6d5a560f8d1b3c4af446c20fb3aac2e556771d93a7d763fd97f08
probe SHA-256       5ddc196695ff1d3085f91c0ae72b37e7eb3ff054b22cb1e538aae3b04c98ff6d
pipe fixture root   30e2d23e8c8f0e8d2425bc55293b5f4ffa293e8df0b252886cf6204fe4e92666
machine             e@192.168.3.94 / 20 cores
build               release / --locked / CARGO_BUILD_JOBS=20
```

The probe was mounted as an untracked workspace example. It defines mirror
types from `development-byte-contract.v1.json`; it does not import future
Development model types or call owner, generator, journal, persistence or
authorization code.

## Result

```text
probe exit                            0
artifact descriptors                34
canonical roundtrips                 4 / 4 files
private reconstruction root  6f4f865654612db327dcad1503790e151e881e8d888abc2a5e16df0545bad8ef
split receipt root           12199a9d2bdbe3172b17e571bbd056f45723e23c3765564da59794eb804c67e5
owner receipt root           0b413483f55c213604cca3bac9821fea79d0067692e4abb0b89dd1e193c6f4c3
sealed attempts                       0
authority effects                     0
```

The retained vector manifest SHA-256 is
`0e2ddc1b0a835d0ce98226e62d0ffc88d5636010d9b44b728a65a40afca3bebf`.
The vector `SHA256SUMS` root is
`19b9efe0a4fea2d77540ce992a633987c142e8c70d4117d2794f40d53be8e8e7`.

Future production structs must decode and reserialize these bytes exactly and
must reproduce all three roots. A match proves byte-contract parity only; it
does not prove validation, persistence, recovery or R8B scientific success.

# Research: commit identity survey (issue #15)

Feeds ADR-0005. Three questions: the hash function for content addressing, the
signing scheme, and what belongs in a commit header.

ADR-0002 already fixed that objects are content-addressed and self-describing
with a `kind` tag, and left "assume a 32-byte content hash" for this ticket to
settle. ADR-0003 fixed that a memory node carries `event_time` (when the agent
formed a belief), which is separate from a commit's record time.

## Hash function

### The options

| | speed vs SHA-256 | security | standard | Rust |
| --- | --- | --- | --- | --- |
| SHA-1 | ~2x | broken (SHAttered) | legacy | yes |
| SHA-256 | baseline (~3 GB/s) | strong | NIST, FIPS | yes |
| SHA-3-256 | ~0.3x (slower) | strong | NIST, FIPS | yes |
| BLAKE2b | ~3x | strong | RFC 7693 | yes |
| BLAKE3 | 4 to 10x, parallel (tens of GB/s on many cores) | strong, same profile as SHA-2 and SHA-3 | not a NIST standard | reference impl in Rust |

### What git did, and why it does not bind us

Git is moving from SHA-1 to SHA-256 for Git 3.0 (targeted late 2026). It chose
SHA-256, not BLAKE3, and the reason was **ecosystem interop**: TLS and X.509
already use SHA-256, and the transition's main blocker is forge support (GitHub
still does not accept SHA-256 repositories). Git needs to interoperate with a
huge installed base.

Mnemosyne has no such constraint. It is a local-first tool with its own format
(ADR-0002). Nothing outside a `.mnem/` store needs to recompute its hashes.

### Recommendation: BLAKE3

- 4 to 10 times faster than SHA-256, and it parallelises, which matters when
  hashing a large state or many objects in one commit.
- Same security profile as SHA-2 and SHA-3.
- The Rust community default for a greenfield project with no external standard
  mandate, with a first-class reference implementation.
- 32-byte output, displayed as 64 hex characters, with `mnem` accepting an
  unambiguous prefix in the spirit of git's short hashes.

The one real cost is FIPS. BLAKE3 is not a NIST standard, so a regulated or
government user could not use it. Mitigation: `config` records `hash_algo`
(ADR-0002's format is self-describing), so a future SHA-256 mode is possible
without a format break. It is not built now.

## Signing

### The options

- **GPG**: git's historical default. Heavy key management, a separate keyring.
- **SSH signatures (ed25519)**: native to git since 2.34. Reuses an existing
  `id_ed25519` key, no separate keypair. The modern default.
- **Sigstore gitsign**: keyless. Signs against a short-lived certificate minted
  from an OIDC identity, logs the signature in a public transparency log, and
  discards the key. Strong for CI and supply-chain, but it depends on Fulcio and
  Rekor, which is heavy infrastructure for a local-first tool.

### What gets signed

Git signs all commit data except the signature header itself: strip the
signature, and the remaining canonical buffer is what was signed. The commit
hash therefore covers the signature, so an unsigned commit and its later-signed
version have different hashes.

The AT Protocol repository model is similar: the issuer signs the root hash plus
metadata, and stores a signed manifest alongside.

### Recommendation: ed25519, optional, in a side table

- **ed25519 keys**, the same shape as an SSH `id_ed25519`. A user can reuse an
  existing SSH key.
- **Optional.** A commit with no signature is valid. A single-agent local store
  often does not need signatures; they matter for v2 (shared memory, agent
  identity) and v3 (a registry).
- **In a side table**, not a commit field: a `redb` table `signatures: commit
  hash -> signature`. The commit object has no signature field, so signing does
  not change a commit's hash. `mnem verify` checks the side table.

This is a deliberate departure from git. In a content-addressed store, keeping
commit identity purely about content, and signing as a separate optional
attestation, is cleaner: you can sign after the fact, sign someone else's
commit, or hold several signatures for one commit, without rewriting anything.

Sigstore is not chosen for v1 because of its infrastructure weight, but the
side-table design does not preclude a Sigstore-style entry later (the value can
be a bare ed25519 signature or a certificate bundle).

The glossary currently calls a commit "signed"; that becomes "optionally
signed", with the signature held separately.

## Commit header

### What git and others carry

Git: tree hash, parent hashes, author (name, email, time), committer (name,
email, time), message, optional signature. The author and committer split
exists because of patches and rebases.

AT Protocol: CID, root hash, signature, parameters, issuer certificates,
metadata.

### Recommendation for the Mnemosyne commit object

| Field | Type | Notes |
| --- | --- | --- |
| `kind` | string, always `commit` | ADR-0002's self-describing objects |
| `parents` | list of commit hashes | 0 for the first commit, 1 normally, 2 or more for a merge |
| `state` | state hash | the state this commit points at |
| `message` | string | freeform |
| `author` | string | who or what made the commit. One field, not author plus committer: an agent commit is made in one step by one identity. Structured agent identity is a v2 and v3 concern. |
| `time` | timestamp | the record time, always set by `mnem` at commit. Distinct from a memory node's `event_time`. |

Not in the header:

- **signature**: side table, as above.
- **committer**: no rebase or patch flow in v1, so one `author` is enough.
- **format_version**: lives in `config`, not per commit (ADR-0002).
- **a run reference**: grouping commits by agent run is plausibly useful, but it
  is deferred; not in v0.1.

### Canonical serialisation

The commit object, and every object, must serialise deterministically: the same
logical content produces the same bytes and therefore the same hash. That means
sorted keys, a fixed field order, and canonical number and string encoding.

This ADR fixes the **requirement** (deterministic canonical serialisation) and
the field set. The **encoding** (canonical CBOR, or a hand-rolled canonical
form) is ADR-0008's job (#20).

## Recommendation for ADR-0005, in one place

- **Hash**: BLAKE3, 32 bytes, hex display, prefix matching. `hash_algo` recorded
  in `config` for a future switch, not built.
- **Signing**: ed25519, optional, held in a side table keyed by commit hash. The
  commit object has no signature field. `mnem verify` checks signatures where
  present. Sigstore deferred.
- **Commit header**: `{kind, parents, state, message, author, time}`. One
  `author`, no `committer`. Record time only; per-belief time is on the node.
- **Serialisation**: deterministic and canonical is a requirement; the encoding
  is ADR-0008.

## Open questions for the grilling (#16)

- BLAKE3 only, or a configurable `hash_algo` built from day one. This survey
  says BLAKE3 only, with the config field reserved.
- Signature in a side table, as recommended, or as a commit field like git.
  The side table changes the "signed" wording in the glossary.
- Does a minimal `mnem sign` and `mnem verify` ship in v1, or is signing purely
  reserved (design only, nothing built)?
- `author`: a bare string in v1, as recommended, or a small structured identity
  now.
- Should a commit reference its agent run? Deferred here; confirm.

## Sources

- [BLAKE3 vs SHA-256 (2026)](https://innovativasofttech.com/blog/blake3-vs-sha-256-speed-security.html) and [BLAKE2 and BLAKE3 alternatives to SHA-256](https://guptadeepak.com/blake2-and-blake3-high-performance-hashing-alternatives/) and [Rust hashing FAQ](https://www.rustfaq.org/en/how-to-hash-data-in-rust-sha-256-blake3-etc/)
- [Git hash function transition](https://git-scm.com/docs/hash-function-transition) and [Git 3.0 and SHA-256 as default](https://www.ossflow.fr/git-3-0-and-beyond-whats-coming/)
- [Signing git commits with SSH keys](https://emmanuelbernard.com/blog/2023/11/27/git-signing-ssh/) and [ditching GnuPG for SSH signing](https://tobywf.com/2026/01/ditch-gnupg-signing-commits-with-ssh/)
- [Keyless commit signing with Sigstore gitsign](https://www.chainguard.dev/unchained/keyless-git-commit-signing-with-gitsign-and-github-actions)
- [Signing commits in Git, explained (GitButler)](https://blog.gitbutler.com/signing-commits-in-git-explained)
- [AT Protocol repository spec](https://atproto.com/specs/repository)
- [IPFS Merkle-DAG spec](https://github.com/ipfs/specs/blob/main/MERKLE_DAG.md) and [Merkle-CRDTs (arXiv 2004.00107)](https://arxiv.org/pdf/2004.00107)

# Golden vectors

Canonical objects, their exact CBOR bytes, and their `ObjectId`. Format version
1, frozen for the `0.0.x` line.

These are pinned in `crates/mnemosyne-store/tests/golden_vectors.rs`, and a test checks
that every hex string below appears there verbatim. If a change to the encoder
moves any byte, that test fails. Such a change is a `format_version` bump
(ADR-0007, ADR-0010), never a silent one.

A reader in another language is correct for these objects when it produces the
same bytes and the same id. `id = BLAKE3(cbor)`, shown as 64 lowercase hex. The
`cbor` lines are single lines; scroll to see the whole value.

## note_minimal

A memory node with no provenance and no `event_time`, so both are absent from
the encoding.

```
object  MemoryNode { id: "greeting", content: "hello", content_kind: note }
id      5107a6e4f686efec31950bf3171e7b4affd3c83697e8ab5182dc4a5f867cce10
cbor    a4626964686772656574696e67646b696e646b6d656d6f72795f6e6f646567636f6e74656e746568656c6c6f6c636f6e74656e745f6b696e64646e6f7465
```

Decoded:

```
a4                                  map(4)
   62 6964                          "id"
   68 6772656574696e67              "greeting"
   64 6b696e64                      "kind"
   6b 6d656d6f72795f6e6f6465        "memory_node"
   67 636f6e74656e74                "content"
   65 68656c6c6f                    "hello"
   6c 636f6e74656e745f6b696e64      "content_kind"
   64 6e6f7465                      "note"
```

The keys are in canonical order: `id`, `kind`, `content`, `content_kind`. That
is the length-first order of RFC 8949 section 4.2, since the encoded keys begin
`62`, `64`, `67`, `6c`.

## note_full

Every provenance field set, `content` a two-key map, `event_time` present.

```
object  MemoryNode {
          id: "customer-4821",
          content: { "plan": "enterprise", "seats": 40 },
          content_kind: note,
          provenance: {
            agent_step: "triage", observation: "ticket-4821 body",
            tool_call: "read-ticket", source: "ticket-4821", note: "first contact"
          },
          event_time: 1757000000000
        }
id      85ff5bf72e74962be19991ba2f67edeb71212d80fc17a9abc83f0aedf228f8ec
cbor    a66269646d637573746f6d65722d34383231646b696e646b6d656d6f72795f6e6f646567636f6e74656e74a264706c616e6a656e746572707269736565736561747318286a6576656e745f74696d651b00000199155c62006a70726f76656e616e6365a5646e6f74656d666972737420636f6e7461637466736f757263656b7469636b65742d3438323169746f6f6c5f63616c6c6b726561642d7469636b65746a6167656e745f73746570667472696167656b6f62736572766174696f6e707469636b65742d3438323120626f64796c636f6e74656e745f6b696e64646e6f7465
```

Notes:

- Top-level keys in canonical order: `id`, `kind`, `content`, `event_time`,
  `provenance`, `content_kind`.
- `event_time` is `1b 00000199155c6200`, an unsigned 64-bit integer
  (1757000000000). CBOR has no 5-byte integer, so a value that does not fit in
  four bytes uses eight. This is shortest form.
- `seats` is `18 28`, an unsigned integer in one trailing byte (40).
- Inside `provenance` the keys are `note`, `source`, `tool_call`, `agent_step`,
  `observation`: length-first, then bytewise.

## state_empty

```
object  State { nodes: {} }
id      d6f3970d8ebe9b9ed1a48d4433b2872b324aa84e97bab1cfbfa50a112a620295
cbor    a2646b696e64657374617465656e6f646573a0
```

```
a2                          map(2)
   64 6b696e64              "kind"
   65 7374617465            "state"
   65 6e6f646573            "nodes"
   a0                       map(0)
```

## state_one

One entry, mapping a node id to the `note_minimal` object id.

```
object  State { nodes: { "greeting": 5107a6e4...cce10 } }
id      d1cdd28f6540e9e7a344554add2bd056bbf3717e7680833582fc463435446991
cbor    a2646b696e64657374617465656e6f646573a1686772656574696e6758205107a6e4f686efec31950bf3171e7b4affd3c83697e8ab5182dc4a5f867cce10
```

The value is `5820` followed by 32 bytes: a CBOR byte string of length 32, not
an array of integers. Every `ObjectId` in the format is encoded this way.

## commit_root

No parents. Its `state` is `state_one`.

```
object  Commit {
          parents: [], state: d1cdd28f...46991,
          message: "first commit", author: "agent", time: 1757000000000
        }
id      925570c5e44700e4e36d74cffeaa010cbf3caeecb364a383f955ff835d088814
cbor    a6646b696e6466636f6d6d69746474696d651b00000199155c62006573746174655820d1cdd28f6540e9e7a344554add2bd056bbf3717e7680833582fc46343544699166617574686f72656167656e74676d6573736167656c666972737420636f6d6d697467706172656e747380
```

Keys in canonical order: `kind`, `time`, `state`, `author`, `message`,
`parents`. `parents` is `80`, an empty array. The commit's hashed bytes are
exactly this object; a signature, if one exists, lives in a side table and is
not part of the id (ADR-0005).

## commit_child

One parent, `commit_root`. Its `state` is `state_empty`.

```
object  Commit {
          parents: [925570c5...88814], state: d6f3970d...20295,
          message: "second", author: "agent", time: 1757000060000
        }
id      fa02c3c4d1d9bdad0adb3f8bddd3ed9f36810e6158ef40d5467e38a579d4d447
cbor    a6646b696e6466636f6d6d69746474696d651b00000199155d4c606573746174655820d6f3970d8ebe9b9ed1a48d4433b2872b324aa84e97bab1cfbfa50a112a62029566617574686f72656167656e74676d657373616765667365636f6e6467706172656e7473815820925570c5e44700e4e36d74cffeaa010cbf3caeecb364a383f955ff835d088814
```

`parents` is `81 5820 <32 bytes>`: an array of one byte string.

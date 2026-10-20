# Research: Gemini 4 Argon through ADK (issue #575)

Feeds #579 (the demo model decision), alongside #614's watch for a
fresher candidate. Confirms Argon's real model id, how ADK routes to
it, which capabilities actually work on that path, the real access
gate, and a real cost estimate for one demo run - all checked
directly against ADK's own `v2.12.0` source and Google's own current
docs, not assumed from the map's earlier secondhand notes.

## Model id and ADK's real routing: no special-casing needed

Model id is `gemini-4-argon`, confirmed directly on Google's own
Argon developer guide. ADK's `Gemini` class (`models/google_llm.py`)
registers itself for `supported_models()` with the regex `r'gemini-.*'`
(confirmed in that file directly) - a plain prefix match, no
allow-list of specific model names. `gemini-4-argon` matches that
regex the same way `gemini-2.5-flash` does, so ADK routes it through
its native `google.genai` client path - the same `Gemini` model class
every other sample in #570 already uses, not `LiteLlm`, and nothing
in `mnem-adk` or the demo agent needs to special-case it. This settles
the map's own open Fog item ("whether ADK 2.12 routes gemini-4-argon
cleanly") with a real yes, by source, not by trying it.

## Function calling and thinking: both pass through generically, with one real narrow gate

Neither capability is gated by model name in ADK's own code:

- **Function calling** is not conditioned on which Gemini model is in
  use anywhere in `google_llm.py` or `_capabilities.py`. Tools are
  attached to the request generically; Argon accepts them the same
  way every other Gemini model does.
- **Thinking** is passed straight through: when
  `llm_request.config.thinking_config` is set, ADK forwards it
  untouched to the real `google.genai` request (confirmed at
  `google_llm.py:598-600`). ADK does not validate the level against
  the model; whether `low`/`medium`/`high` are accepted is entirely
  the live API's own call. Argon's own developer guide confirms it
  accepts exactly those three levels, `medium` the default.
- **The one real gate found**: `_capabilities.py`'s
  `gemini_output_schema_and_tools()` returns true only when the
  backend variant is Vertex AI **and** the model is a Gemini model -
  pairing a structured output schema with tool calls in the same
  request needs Vertex AI specifically, not AI Studio. This is narrow
  (only the combination, not tools or thinking alone) but real, and
  worth knowing before scripting a demo that pairs both: AI Studio
  access does not clear that particular combination.

## Access: two different gates, not one

The map's earlier note ("AI Studio and Antigravity access needs AI
Ultra") describes the *consumer UI* gate correctly, but ADK itself
never goes through that UI - it calls the Gemini API directly with a
key. That path's real gate, confirmed on Google's own pricing page, is
simpler: Argon is "not available on the Free tier" of the Gemini API,
so the key needs the API's own paid tier (billing enabled on the
underlying Cloud project), not an AI Ultra personal subscription.
These are two separate products gating two separate access paths; the
map's existing plan to set up paid Gemini API billing is the one that
actually unblocks `mnem-adk`'s own call path.

## Cost: a real estimate, and it is not the obstacle

Introductory pricing, confirmed on Google's own pricing page: $2/$10
per million input/output tokens through 30 November 2026, rising to
$4/$20 standard after. A short scripted demo (a handful of turns,
growing conversation history, one or two tool calls, a modest
preloaded-memory block) runs on the order of 10-20k input tokens and
2-5k output tokens total across the whole run - call it $0.03-$0.09 at
input/output pricing alone. `thinking="high"` bills its reasoning
tokens as output tokens too, which can add real cost on a harder
prompt, so a wider honest bound is **a few cents to perhaps $0.50 for
the whole recorded run**, not per turn. Either way this confirms
#580's own existing framing directly: Argon's problem for this launch
was never affordability, it is zero-friction access - the real
billing-setup step the maintainer still owes, not the cost of running
the demo once that is done.

## Recommendation for #579

Nothing here argues against Argon on technical grounds - ADK routes
it cleanly, function calling and thinking both work, and one full
demo run costs cents. The decision in #579 is unchanged by this
research: it is still a timing and friction question (zero-friction
access vs. four-days-old freshness), not a capability or cost
question. If #614 does not surface a fresher candidate before ship,
Argon is a technically clean fallback with no new blocker found here.

Related: #568 (map), #569/#570 (the `Gemini`/`LlmAgent` call patterns
this routing confirms), #579 (the decision this feeds), #580 (the
locked pitch this cost finding supports), #614 (the parallel watch for
a fresher candidate).
